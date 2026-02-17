use bytemuck::Pod;
use memmap2::{MmapMut, MmapOptions};
use std::fs::OpenOptions;
use std::hint::spin_loop;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// A memory-mapped buffer for random-access, slot-based storage.
///
/// It uses a versioning scheme (SeqLock-like) for consistent reads without blocking the writer.
pub struct SlotMmap<T: Pod> {
    _mmap: Arc<MmapMut>,
    ptr: *mut u8,
    num_slots: usize,
    slot_size: usize,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Pod> SlotMmap<T> {
    pub fn new(path: Option<PathBuf>, num_slots: usize) -> Result<Self, std::io::Error> {
        // We manually calculate the slot size.
        // 8 bytes for version + T + padding to reach 64-byte alignment (cache line).
        let slot_size = 8 + size_of::<T>();

        let mut mmap = if let Some(p) = path {
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(p)?;

            file.set_len((num_slots * slot_size) as u64)?;
            unsafe { MmapOptions::new().map_mut(&file)? }
        } else {
            MmapOptions::new().len(num_slots * slot_size).map_anon()?
        };

        Ok(Self {
            ptr: mmap.as_mut_ptr(),
            num_slots,
            slot_size,
            _mmap: Arc::new(mmap),
            _marker: std::marker::PhantomData,
        })
    }

    /// OPEN: Loads an existing file and maps its current size.
    pub fn load(path: PathBuf) -> Result<Self, std::io::Error> {
        let file = OpenOptions::new().read(true).write(true).open(&path)?;

        let mmap = unsafe { MmapOptions::new().map_mut(&file)? };

        let len = mmap.len();

        let slot_size = 8 + size_of::<T>();
        let num_slots = len / slot_size;
        Ok(Self {
            ptr: mmap.as_ptr() as *mut u8,
            num_slots,
            slot_size,
            _mmap: Arc::new(mmap),
            _marker: std::marker::PhantomData,
        })
    }

    /// WRITER: Updates the specific slot by index using versioning.
    pub fn write(&mut self, index: usize, state: &T) {
        assert!(index < self.num_slots);
        let offset = index * self.slot_size;

        unsafe {
            let version_ptr = self.ptr.add(offset) as *const AtomicU64;

            // 1. Increment to ODD
            (*version_ptr).fetch_add(1, Ordering::Relaxed);
            std::sync::atomic::fence(Ordering::SeqCst);

            // 2. Copy data
            let data_ptr = self.ptr.add(offset + 8);
            std::ptr::copy_nonoverlapping(
                bytemuck::bytes_of(state).as_ptr(),
                data_ptr,
                std::mem::size_of::<T>(),
            );

            // 3. Increment to EVEN
            std::sync::atomic::fence(Ordering::SeqCst);
            (*version_ptr).fetch_add(1, Ordering::Relaxed);
        }
    }

    /// READER: Performs a consistent snapshot read with spin-retry logic.
    pub fn read_snapshot_with_retry(&self, index: usize, max_retries: usize) -> Option<T> {
        assert!(index < self.num_slots);
        let offset = index * self.slot_size;

        unsafe {
            let version_ptr = self.ptr.add(offset) as *const AtomicU64;
            let data_ptr = self.ptr.add(offset + 8);

            for _ in 0..max_retries {
                let v1 = (*version_ptr).load(Ordering::Relaxed);
                std::sync::atomic::fence(Ordering::SeqCst);

                if v1.is_multiple_of(2) {
                    let mut data: T = std::mem::zeroed();
                    std::ptr::copy_nonoverlapping(
                        data_ptr,
                        &mut data as *mut T as *mut u8,
                        std::mem::size_of::<T>(),
                    );

                    std::sync::atomic::fence(Ordering::SeqCst);
                    let v2 = (*version_ptr).load(Ordering::Relaxed);
                    if v1 == v2 {
                        return Some(data);
                    }
                }
                spin_loop();
            }
        }
        None
    }

    pub fn reader(&self) -> Self {
        Self {
            _mmap: self._mmap.clone(),
            ptr: self.ptr,
            num_slots: self.num_slots,
            slot_size: self.slot_size,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn num_slots(&self) -> usize {
        self.num_slots
    }
}

unsafe impl<T: Pod> Send for SlotMmap<T> {}
unsafe impl<T: Pod> Sync for SlotMmap<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::Zeroable;
    use std::thread;

    #[repr(C)]
    #[derive(Copy, Clone, Debug, Pod, Zeroable, PartialEq)]
    struct TestData {
        a: u64,
        b: u64,
        c: u64,
        d: u64,
    }

    #[test]
    fn test_new_anonymous() {
        let mut slot_mmap = SlotMmap::<TestData>::new(None, 10).unwrap();
        assert_eq!(slot_mmap.num_slots(), 10);

        let data = TestData {
            a: 1,
            b: 2,
            c: 3,
            d: 4,
        };
        slot_mmap.write(0, &data);

        let read_data = slot_mmap.read_snapshot_with_retry(0, 10).unwrap();
        assert_eq!(data, read_data);
    }

    #[test]
    fn test_file_backed() {
        let path = std::env::temp_dir().join(format!("test_slots_{}.mmap", std::process::id()));
        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }

        {
            let mut slot_mmap = SlotMmap::<TestData>::new(Some(path.clone()), 5).unwrap();
            slot_mmap.write(
                2,
                &TestData {
                    a: 10,
                    b: 20,
                    c: 30,
                    d: 40,
                },
            );
        }

        {
            let slot_mmap = SlotMmap::<TestData>::load(path.clone()).unwrap();
            assert_eq!(slot_mmap.num_slots(), 5);
            let data = slot_mmap.read_snapshot_with_retry(2, 10).unwrap();
            assert_eq!(
                TestData {
                    a: 10,
                    b: 20,
                    c: 30,
                    d: 40
                },
                data
            );
        }

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    #[should_panic]
    fn test_boundary_write() {
        let mut slot_mmap = SlotMmap::<TestData>::new(None, 5).unwrap();
        slot_mmap.write(
            5,
            &TestData {
                a: 1,
                b: 2,
                c: 3,
                d: 4,
            },
        );
    }

    #[test]
    #[should_panic]
    fn test_boundary_read() {
        let slot_mmap = SlotMmap::<TestData>::new(None, 5).unwrap();
        slot_mmap.read_snapshot_with_retry(5, 10);
    }

    #[test]
    fn test_multithreaded_consistency() {
        let mut slot_mmap = SlotMmap::<TestData>::new(None, 1).unwrap();
        let reader = slot_mmap.reader();

        let writer_thread = thread::spawn(move || {
            for i in 0..1_000_000 {
                slot_mmap.write(
                    0,
                    &TestData {
                        a: i,
                        b: i,
                        c: i,
                        d: i,
                    },
                );
            }
        });

        let reader_thread = thread::spawn(move || {
            let mut success_count = 0;
            for _ in 0..1_000_000 {
                if let Some(data) = reader.read_snapshot_with_retry(0, 100) {
                    success_count += 1;
                    assert_eq!(
                        data.a, data.b,
                        "Data corruption detected! a: {}, b: {}",
                        data.a, data.b
                    );
                    assert_eq!(
                        data.a, data.c,
                        "Data corruption detected! a: {}, b: {}",
                        data.a, data.c
                    );
                    assert_eq!(
                        data.a, data.d,
                        "Data corruption detected! a: {}, b: {}",
                        data.a, data.d
                    );
                }
            }
            assert!(success_count > 0, "Reader thread made no successful reads");
        });

        writer_thread.join().unwrap();
        reader_thread.join().unwrap();
    }

    #[test]
    fn test_multiple_readers_consistency() {
        let mut slot_mmap = SlotMmap::<TestData>::new(None, 1).unwrap();

        let mut readers = vec![];
        for _ in 0..4 {
            readers.push(slot_mmap.reader());
        }

        let writer_thread = thread::spawn(move || {
            for i in 0..1_000_000 {
                slot_mmap.write(
                    0,
                    &TestData {
                        a: i,
                        b: i,
                        c: i,
                        d: i,
                    },
                );
            }
        });

        let mut reader_threads = vec![];
        for reader in readers {
            reader_threads.push(thread::spawn(move || {
                let mut success_count = 0;
                for _ in 0..1_000_000 {
                    if let Some(data) = reader.read_snapshot_with_retry(0, 100) {
                        success_count += 1;
                        assert_eq!(data.a, data.b);
                        assert_eq!(data.a, data.c);
                        assert_eq!(data.a, data.d);
                    }
                }
                assert!(success_count > 0, "Reader thread made no successful reads");
            }));
        }

        writer_thread.join().unwrap();
        for t in reader_threads {
            t.join().unwrap();
        }
    }

    #[test]
    fn test_reader_cloning() {
        let mut slot_mmap = SlotMmap::<TestData>::new(None, 10).unwrap();
        let reader1 = slot_mmap.reader();
        let reader2 = reader1.reader();

        let data = TestData {
            a: 1,
            b: 2,
            c: 3,
            d: 4,
        };
        slot_mmap.write(5, &data);

        assert_eq!(reader1.read_snapshot_with_retry(5, 10), Some(data));
        assert_eq!(reader2.read_snapshot_with_retry(5, 10), Some(data));
    }
}
