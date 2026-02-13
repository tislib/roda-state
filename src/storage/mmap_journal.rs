use bytemuck::Pod;
use memmap2::{MmapMut, MmapOptions};
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

pub(crate) struct MmapJournal {
    _mmap: Arc<MmapMut>,
    ptr: *mut u8,
    len: usize,
    write_index: Arc<AtomicUsize>,
    read_only: bool,
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
            _mmap: Arc::new(mmap),
            ptr,
            len,
            write_index: Arc::new(Default::default()),
            read_only: false,
        })
    }

    /// OPEN: Loads an existing file and maps its current size.
    pub fn load(path: PathBuf) -> Result<Self, std::io::Error> {
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

    /// 1. Read (Immutable)
    ///
    /// Casts bytes at offset to a reference of T.
    pub fn read<T: Pod>(&self, offset: usize) -> &T {
        let end = offset + size_of::<T>();
        assert!(
            end <= self.len,
            "Read crosses buffer boundary - alignment issue?"
        );
        bytemuck::from_bytes(&self.slice()[offset..end])
    }

    pub(crate) fn read_window<T: Pod, const N: usize>(&self, offset: usize) -> &[T] {
        let end = offset + size_of::<T>() * N;
        assert!(
            end <= self.len,
            "Read crosses buffer boundary - alignment issue?"
        );
        let bytes = &self.slice()[offset..end];

        bytemuck::cast_slice(bytes)
    }

    pub fn append<T: Pod>(&mut self, state: &T) {
        let current_pos = self.write_index.load(std::sync::atomic::Ordering::Relaxed);
        let size = size_of::<T>();
        let end = current_pos + size;

        let dest_slice = self.slice_mut();

        // Check for boundary crossing
        assert!(
            end <= dest_slice.len(),
            "Journal is full. Cannot append more data."
        );

        // Perform the write
        dest_slice[current_pos..end].copy_from_slice(bytemuck::bytes_of(state));

        self.write_index
            .store(end, std::sync::atomic::Ordering::Release);
    }

    fn slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }

    fn slice_mut(&mut self) -> &mut [u8] {
        assert!(!self.read_only, "Cannot mutate read-only buffer");
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }

    pub(crate) fn get_write_index(&self) -> usize {
        self.write_index.load(std::sync::atomic::Ordering::Acquire)
    }

    pub(crate) fn len(&self) -> usize {
        self.len
    }

    pub(crate) fn reader(&self) -> MmapJournal {
        MmapJournal {
            _mmap: self._mmap.clone(),
            ptr: self.ptr,
            len: self.len,
            write_index: self.write_index.clone(),
            read_only: true,
        }
    }
}

unsafe impl Send for MmapJournal {}
