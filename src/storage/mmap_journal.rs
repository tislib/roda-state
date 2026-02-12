use bytemuck::Pod;
use memmap2::{MmapMut, MmapOptions};
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

pub(crate) struct MmapJournal {
    ptr: *mut u8,
    len: usize,
    write_index: Arc<AtomicUsize>,
}

impl MmapJournal {
    /// CREATE: Creates a brand new file, truncating any existing data.
    pub fn new(path: Option<PathBuf>, total_size: usize) -> Result<Self, std::io::Error> {
        let mut mmap = if let Some(p) = &path {
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(p)?;

            file.set_len(total_size as u64)?;
            unsafe { MmapOptions::new().map_mut(&file)? }
        } else {
            MmapOptions::new().len(total_size).map_anon()?
        };

        let ptr = mmap.as_mut_ptr();
        let len = mmap.len();
        Ok(Self {
            ptr,
            len,
            write_index: Arc::new(Default::default()),
        })
    }

    /// OPEN: Loads an existing file and maps its current size.
    pub fn load(path: PathBuf) -> Result<Self, std::io::Error> {
        let file = OpenOptions::new().read(true).write(true).open(&path)?;

        let mut mmap = unsafe { MmapOptions::new().map_mut(&file)? };

        let ptr = mmap.as_mut_ptr();
        let len = mmap.len();
        Ok(Self {
            ptr,
            len,
            write_index: Arc::new(Default::default()),
        })
    }

    // --- Bytemuck Methods ---

    /// 1. Read (Immutable)
    /// Casts bytes at offset to a reference of T.
    pub fn read<T: Pod>(&self, offset: usize) -> &T {
        let end = offset + size_of::<T>();
        assert!(offset + size_of::<T>() <= self.len);
        bytemuck::from_bytes(&self.slice()[offset..end])
    }

    pub fn append<T: Pod>(&mut self, state: &T) {
        let offset = self.write_index.load(std::sync::atomic::Ordering::Relaxed);
        let size = std::mem::size_of::<T>();
        let end = offset + size;

        let dest_slice = self.slice_mut();

        // 1. Check for buffer overflow
        assert!(end <= dest_slice.len(), "Append exceeds buffer capacity!");

        // 2. Perform the write
        dest_slice[offset..end].copy_from_slice(bytemuck::bytes_of(state));

        self.write_index.store(end, std::sync::atomic::Ordering::Release);
    }

    fn slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }

    fn slice_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }

    pub(crate) fn get_write_index(&self) -> usize {
        self.write_index.load(std::sync::atomic::Ordering::Acquire)
    }

    pub(crate) fn reader(&self) -> MmapJournal {
        MmapJournal {
            ptr: self.ptr,
            len: self.len,
            write_index: self.write_index.clone(),
        }
    }
}

unsafe impl Send for MmapJournal {}
