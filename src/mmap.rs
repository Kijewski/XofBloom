#![allow(unsafe_code)]

//! Functions to work with file-system backed storage for [`CustomBloom`] instances.

use core::sync::atomic::AtomicUsize;
use core::{hash, num, slice};
use std::{fs, io};

use memmap2::{Advice, MmapOptions};
#[doc(no_inline)]
pub use memmap2::{MmapAsRawDesc, MmapMut};

use crate::calc::BITS_PER_CELL;
use crate::{
    AllocError, Blake3, BuildableStorage, CustomBloom, Digester, Hasher, optimal_cell_count,
};

/// A Bloom filter that uses the extendable output of [`blake3`] and memory-mapped storage.
#[cfg(feature = "blake3")]
pub type MmapXofBloom = MmapBloom<Blake3>;

/// A Bloom filter implementation with a custom [`Hasher`] that uses memory-mapped files as
/// backing storage.
#[allow(type_alias_bounds)] // Not enforced, but still documented.
pub type MmapBloom<H: Hasher> = CustomBloom<Mmap, H>;

/// A memory-mapped storage for Bloom filters.
#[derive(Debug)]
pub struct Mmap {
    map: MmapMut,
    file: Option<fs::File>,
}

impl AsRef<MmapMut> for Mmap {
    #[inline]
    fn as_ref(&self) -> &MmapMut {
        &self.map
    }
}

impl AsMut<MmapMut> for Mmap {
    #[inline]
    fn as_mut(&mut self) -> &mut MmapMut {
        &mut self.map
    }
}

impl From<MmapMut> for Mmap {
    #[inline]
    fn from(map: MmapMut) -> Self {
        Self { map, file: None }
    }
}

impl From<Mmap> for MmapMut {
    #[inline]
    fn from(value: Mmap) -> Self {
        value.map
    }
}

impl AsRef<[AtomicUsize]> for Mmap {
    #[inline]
    fn as_ref(&self) -> &[AtomicUsize] {
        const { assert!(align_of::<AtomicUsize>() <= 0x1000) };

        let data = self.map.as_ref();
        // SAFETY: we know that the memory is aligned to a page
        unsafe {
            slice::from_raw_parts(data.as_ptr().cast(), data.len() / size_of::<AtomicUsize>())
        }
    }
}

impl BuildableStorage for Mmap {
    #[inline]
    fn new_storage(size: num::NonZero<usize>) -> Result<Self, AllocError> {
        Anon.into_mmap(size)
    }
}

impl<H: Hasher + Default> CustomBloom<Mmap, H> {
    /// Initialize a new Bloom filter instance in a memory-mapped file for an expected item count
    /// `num_items` and an acceptable `error_rate`.
    ///
    /// The instance is seeded with a runtime generated random number.
    #[cfg(feature = "new")]
    pub fn new_in_file<F>(
        file: F,
        num_items: num::NonZero<usize>,
        error_rate: f32,
    ) -> Result<Self, AllocError>
    where
        F: IntoMmap,
    {
        Self::new_in_file_with_seed(
            file,
            crate::random_seed(&mut [core::mem::MaybeUninit::uninit(); _]),
            num_items,
            error_rate,
        )
    }

    /// Initialize a new Bloom filter instance in a memory-mapped file for an expected item count
    /// `num_items` and an acceptable `error_rate`.
    ///
    /// Using an unseeded [`Hasher`] makes it easier for attackers to generate false-positive
    /// results. Depending on the use case, this can aid denial-of-service attacks.
    pub fn new_in_file_with_seed<F, V>(
        file: F,
        seed: &V,
        num_items: num::NonZero<usize>,
        error_rate: f32,
    ) -> Result<Self, AllocError>
    where
        F: IntoMmap,
        V: hash::Hash + ?Sized,
    {
        let mut hasher = H::default();
        seed.hash(&mut Digester(&mut hasher));
        Self::new_in_file_with_hasher(file, hasher, num_items, error_rate)
    }
}

impl<H: Hasher> CustomBloom<Mmap, H> {
    /// Initialize a new Bloom filter instance in a memory-mapped file with a custom `hasher` for
    /// an expected item count `num_items` and an acceptable `error_rate`.
    ///
    /// The [`Hasher`] should be pre-seeded, as using an unseeded `Hasher` makes it easier for
    /// attackers to generate false-positive results. Depending on the use case, this can aid
    /// denial-of-service attacks.
    pub fn new_in_file_with_hasher<F>(
        file: F,
        hasher: H,
        num_items: num::NonZero<usize>,
        error_rate: f32,
    ) -> Result<Self, AllocError>
    where
        F: IntoMmap,
    {
        let storage = file.into_mmap(optimal_cell_count(num_items.get(), error_rate))?;
        Ok(Self::new_with_storage(storage, hasher, num_items))
    }
}

/// A type that can be converted into a memory-mapped storage for Bloom filters.
pub trait IntoMmap: Sized {
    /// Convert `self` into a memory-mapped storage for Bloom filters.
    fn into_mmap(self, size: num::NonZero<usize>) -> Result<Mmap, AllocError>;
}

/// A marker type for creating anonymous memory-mapped storage.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Anon;

impl IntoMmap for Anon {
    fn into_mmap(self, size: num::NonZero<usize>) -> Result<Mmap, AllocError> {
        mapped_memory(mmap_options(size).map_anon())
    }
}

/// Lock the file before memory-mapping it, and keep the file lock around until the Bloom filter
/// instance is discarded.
#[derive(Debug)]
pub struct LockedFile {
    /// The file to memory-map.
    pub file: fs::File,
    /// Should a shared lock be used?
    pub shared: bool,
    /// Should the file be resized?
    pub resize: bool,
}

impl IntoMmap for LockedFile {
    fn into_mmap(self, size: num::NonZero<usize>) -> Result<Mmap, AllocError> {
        let Self {
            file,
            shared,
            resize,
        } = self;

        match shared {
            true => file.try_lock_shared().map_err(|_| AllocError)?,
            false => file.try_lock().map_err(|_| AllocError)?,
        }

        let mut storage = file.into_mmap(size)?;

        if resize {
            file.set_len(storage.map.len() as u64)
                .map_err(|_| AllocError)?;
        }

        storage.file = Some(file);
        Ok(storage)
    }
}

impl<T: MmapAsRawDesc> IntoMmap for T {
    fn into_mmap(self, size: num::NonZero<usize>) -> Result<Mmap, AllocError> {
        // SAFETY: It is safe to modify the underlying data. The results might be wrong, though.
        mapped_memory(unsafe { mmap_options(size).map_mut(self) })
    }
}

fn mmap_options(size: std::num::NonZero<usize>) -> MmapOptions {
    let mut options = MmapOptions::new();
    let _ = options.len((size.get() * size_of::<AtomicUsize>()).next_multiple_of(page_size::get()));
    let _ = options.no_reserve_swap();
    options
}

fn mapped_memory(result: io::Result<MmapMut>) -> Result<Mmap, AllocError> {
    let map = result.map_err(|_| AllocError)?;

    let _: io::Result<()> = map.advise(Advice::Random);
    #[cfg(target_os = "linux")]
    let _: io::Result<()> = map.advise(Advice::DontDump);

    Ok(Mmap { map, file: None })
}

#[test]
fn read_words_mmap() {
    crate::dict_test(|num_items, error_rate| {
        let file = LockedFile {
            file: tempfile::tempfile().unwrap(),
            shared: false,
            resize: true,
        };
        MmapXofBloom::new_in_file(file, num_items, error_rate).unwrap()
    })
}
