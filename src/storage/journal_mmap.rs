use bytemuck::Pod;
use memmap2::{MmapMut, MmapOptions};
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

/// A memory-mapped buffer optimized for sequential, append-only operations.
///
/// It supports wait-free reads while the writer is appending data.
pub(crate) struct JournalMmap {
    _mmap: Arc<MmapMut>,
    ptr: *mut u8,
    len: usize,
    write_index: Arc<AtomicUsize>,
    read_only: bool,
}

impl JournalMmap {
    /// CREATE: Creates a brand new file, truncating any existing data.
    pub(crate) fn new(path: Option<PathBuf>, total_size: usize) -> Result<Self, std::io::Error> {
        let mut mmap = if let Some(p) = &path {
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(p)?;

            file.set_len(total_size as u64)?;
            unsafe { MmapOptions::new().huge(Some(21)).map_mut(&file)? }
        } else {
            MmapOptions::new().len(total_size).map_anon()?
        };

        let ptr = mmap.as_mut_ptr();
        let len = mmap.len();
        Ok(Self {
            _mmap: Arc::new(mmap),
            ptr,
            len,
            write_index: Arc::new(Default::default()),
            read_only: false,
        })
    }

    /// OPEN: Loads an existing file and maps its current size.
    pub(crate) fn load(path: PathBuf) -> Result<Self, std::io::Error> {
        let file = OpenOptions::new().read(true).write(true).open(&path)?;

        let mut mmap = unsafe { MmapOptions::new().map_mut(&file)? };

        let ptr = mmap.as_mut_ptr();
        let len = mmap.len();
        Ok(Self {
            _mmap: Arc::new(mmap),
            ptr,
            len,
            write_index: Arc::new(Default::default()),
            read_only: false,
        })
    }

    // --- Bytemuck Methods ---

    /// Casts bytes at offset to a reference of T.
    #[inline(always)]
    pub(crate) fn read<T: Pod>(&self, offset: usize) -> &T {
        let size = size_of::<T>();
        let end = offset + size;
        assert!(
            end <= self.len,
            "Read crosses buffer boundary - alignment issue?"
        );
        let slice = unsafe { std::slice::from_raw_parts(self.ptr.add(offset), size) };
        bytemuck::from_bytes(slice)
    }

    #[inline(always)]
    pub(crate) fn read_window_const<T: Pod, const N: usize>(&self, offset: usize) -> &[T] {
        let size = size_of::<T>() * N;
        let end = offset + size;
        assert!(
            end <= self.len,
            "Read crosses buffer boundary - alignment issue?"
        );
        let bytes = unsafe { std::slice::from_raw_parts(self.ptr.add(offset), size) };

        bytemuck::cast_slice(bytes)
    }

    /// Returns a slice of T starting at the given offset.
    ///
    /// This is more efficient than calling `read` multiple times.
    #[inline(always)]
    pub(crate) fn read_window<T: Pod>(&self, offset: usize, count: usize) -> &[T] {
        let size = size_of::<T>() * count;
        let end = offset + size;
        assert!(
            end <= self.len,
            "Read crosses buffer boundary - alignment issue?"
        );
        let bytes = unsafe { std::slice::from_raw_parts(self.ptr.add(offset), size) };

        bytemuck::cast_slice(bytes)
    }

    /// Appends an item to the buffer.
    ///
    /// # Panics
    /// Panics if the buffer is full.
    #[inline(always)]
    pub(crate) fn append<T: Pod>(&mut self, state: &T) {
        assert!(!self.read_only, "Cannot mutate read-only buffer");
        let current_pos = self.write_index.load(std::sync::atomic::Ordering::Relaxed);
        let size = size_of::<T>();
        let end = current_pos + size;

        // Check for boundary crossing
        assert!(end <= self.len, "Journal is full. Cannot append more data.");

        // Perform the write
        unsafe {
            let dest_ptr = self.ptr.add(current_pos);
            let src_ptr = bytemuck::bytes_of(state).as_ptr();
            std::ptr::copy_nonoverlapping(src_ptr, dest_ptr, size);
        }

        self.write_index
            .store(end, std::sync::atomic::Ordering::Release);
    }

    #[inline(always)]
    pub(crate) fn get_write_index(&self) -> usize {
        self.write_index.load(std::sync::atomic::Ordering::Acquire)
    }

    #[inline(always)]
    pub(crate) fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    pub(crate) fn reader(&self) -> JournalMmap {
        JournalMmap {
            _mmap: self._mmap.clone(),
            ptr: self.ptr,
            len: self.len,
            write_index: self.write_index.clone(),
            read_only: true,
        }
    }
}

unsafe impl Send for JournalMmap {}

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::Zeroable;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_new_anonymous() {
        let size = 1024;
        let journal = JournalMmap::new(None, size).unwrap();
        assert_eq!(journal.len(), size);
        assert_eq!(journal.get_write_index(), 0);
    }

    #[test]
    fn test_append_and_read() {
        let mut journal = JournalMmap::new(None, 1024).unwrap();
        let val: u32 = 0x12345678;
        journal.append(&val);
        assert_eq!(journal.get_write_index(), 4);

        let read_val: u32 = *journal.read(0);
        assert_eq!(read_val, val);
    }

    #[test]
    fn test_append_multiple() {
        let mut journal = JournalMmap::new(None, 1024).unwrap();
        journal.append(&10u64);
        journal.append(&20u64);
        assert_eq!(journal.get_write_index(), 16);

        assert_eq!(*journal.read::<u64>(0), 10);
        assert_eq!(*journal.read::<u64>(8), 20);
    }

    #[test]
    fn test_read_window() {
        let mut journal = JournalMmap::new(None, 1024).unwrap();
        journal.append(&1u32);
        journal.append(&2u32);
        journal.append(&3u32);

        let window: &[u32] = journal.read_window_const::<u32, 3>(0);
        assert_eq!(window, &[1, 2, 3]);
    }

    #[test]
    #[should_panic(expected = "Journal is full. Cannot append more data.")]
    fn test_boundary_append() {
        let mut journal = JournalMmap::new(None, 4).unwrap();
        journal.append(&1u32);
        journal.append(&1u8); // This should panic
    }

    #[test]
    #[should_panic(expected = "Read crosses buffer boundary")]
    fn test_boundary_read() {
        let journal = JournalMmap::new(None, 4).unwrap();
        let _: &u64 = journal.read(0); // Should panic
    }

    #[test]
    #[should_panic(expected = "Read crosses buffer boundary")]
    fn test_boundary_read_window() {
        let mut journal = JournalMmap::new(None, 8).unwrap();
        journal.append(&1u32);
        journal.append(&2u32);
        let _: &[u32] = journal.read_window_const::<u32, 3>(0); // Should panic
    }

    #[test]
    fn test_reader_concurrency() {
        let mut journal = JournalMmap::new(None, 1024).unwrap();
        let reader = journal.reader();

        let handle = thread::spawn(move || {
            let mut last_idx = 0;
            let mut count = 0;
            while count < 10 {
                let current_idx = reader.get_write_index();
                if current_idx > last_idx {
                    let val: u32 = *reader.read(last_idx);
                    assert_eq!(val, count);
                    last_idx += std::mem::size_of::<u32>();
                    count += 1;
                }
                thread::yield_now();
            }
        });

        for i in 0..10u32 {
            journal.append(&i);
            thread::sleep(Duration::from_millis(1));
        }

        handle.join().unwrap();
    }

    #[test]
    #[should_panic(expected = "Cannot mutate read-only buffer")]
    fn test_reader_cannot_append() {
        let journal = JournalMmap::new(None, 1024).unwrap();
        let mut reader = journal.reader();
        reader.append(&1u32);
    }

    #[test]
    fn test_file_backed() {
        let path = std::env::temp_dir().join(format!("test_journal_{}.mmap", std::process::id()));
        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }

        {
            let mut journal = JournalMmap::new(Some(path.clone()), 1024).unwrap();
            journal.append(&123u64);
        }

        {
            let journal = JournalMmap::load(path.clone()).unwrap();
            assert_eq!(journal.len(), 1024);
            // write_index is not persisted
            assert_eq!(journal.get_write_index(), 0);
            assert_eq!(*journal.read::<u64>(0), 123u64);
        }

        let _ = std::fs::remove_file(&path);
    }

    #[repr(C)]
    #[derive(Copy, Clone, Debug, Pod, Zeroable, PartialEq)]
    struct LargeData {
        a: u64,
        b: u64,
        c: u64,
        d: u64,
    }

    #[test]
    fn test_reader_no_corruption() {
        let mut journal = JournalMmap::new(None, 1024 * 1024).unwrap();
        let reader = journal.reader();

        let handle = thread::spawn(move || {
            let mut last_idx = 0;
            while last_idx < 1000 * size_of::<LargeData>() {
                let current_idx = reader.get_write_index();
                while last_idx < current_idx {
                    let data: LargeData = *reader.read(last_idx);
                    // Check if data is corrupted (a, b, c, d should all be equal to the same value)
                    assert_eq!(
                        data.a, data.b,
                        "Data corruption detected at index {}",
                        last_idx
                    );
                    assert_eq!(
                        data.a, data.c,
                        "Data corruption detected at index {}",
                        last_idx
                    );
                    assert_eq!(
                        data.a, data.d,
                        "Data corruption detected at index {}",
                        last_idx
                    );
                    last_idx += size_of::<LargeData>();
                }
                thread::yield_now();
            }
        });

        for i in 0..1000u64 {
            let data = LargeData {
                a: i,
                b: i,
                c: i,
                d: i,
            };
            journal.append(&data);
        }

        handle.join().unwrap();
    }

    #[test]
    fn test_immediate_read() {
        let mut journal = JournalMmap::new(None, 1024).unwrap();
        let val: u64 = 0xDEADBEEFCAFEBABE;
        journal.append(&val);

        // Data should be immediately available at the expected offset
        let read_val: u64 = *journal.read(0);
        assert_eq!(read_val, val);

        let val2: u64 = 0x1122334455667788;
        journal.append(&val2);
        assert_eq!(*journal.read::<u64>(8), val2);
    }

    #[test]
    fn test_mixed_type_alignment_failure() {
        let mut journal = JournalMmap::new(None, 1024).unwrap();

        journal.append(&0xAA_u8); // write_index becomes 1
        journal.append(&0xDEADBEEF_u32); // written at offset 1

        // This will panic and FAIL the test runner because offset 1 is unaligned for u32.
        let _val: &u8 = journal.read(0);
    }
}
