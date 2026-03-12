// SPDX-License-Identifier: ISC OR MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 René Kijewski <crates.io@k6i.de>

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod calc;

use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering::Relaxed;
use core::{fmt, hash, mem, num};

use bytemuck::Zeroable;

use crate::calc::{BITS_PER_CELL, MAX_HASH_COUNT};
pub use crate::calc::{optimal_cell_count, optimal_hash_count};

#[cfg(all(feature = "alloc", feature = "blake3"))]
pub type XofBloom = BoxBloom<Blake3>;

#[cfg(feature = "alloc")]
pub type BoxBloom<H> = CustomBloom<alloc::boxed::Box<[AtomicUsize]>, H>;

pub type SliceBloom<'a, H> = CustomBloom<&'a [AtomicUsize], H>;

#[cfg(feature = "blake3")]
pub type Blake3 = blake3::Hasher;

#[derive(Clone)]
pub struct CustomBloom<S, H> {
    storage: S,
    hasher: H,
    hash_count: num::NonZero<u8>,
}

impl<S, H> fmt::Debug for CustomBloom<S, H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CustomBloom").finish_non_exhaustive()
    }
}

impl<S: BuildableStorage, H: Hasher + Default> CustomBloom<S, H> {
    #[cfg(feature = "new")]
    pub fn new(num_items: num::NonZero<usize>, error_rate: f32) -> Self {
        let mut seed = [mem::MaybeUninit::uninit(); 24];
        let seed = getrandom::fill_uninit(&mut seed).unwrap();
        Self::new_with_seed(seed, num_items, error_rate)
    }

    pub fn new_with_seed<V>(seed: &V, num_items: num::NonZero<usize>, error_rate: f32) -> Self
    where
        V: hash::Hash + ?Sized,
    {
        let mut hasher = H::default();
        seed.hash(&mut Digester(&mut hasher));
        Self::new_with_hasher(hasher, num_items, error_rate)
    }
}

impl<S: BuildableStorage, H: Default> CustomBloom<S, H> {
    pub fn new_unseeded(num_items: num::NonZero<usize>, error_rate: f32) -> Self {
        let hasher = H::default();
        Self::new_with_hasher(hasher, num_items, error_rate)
    }
}

impl<S: BuildableStorage, H> CustomBloom<S, H> {
    pub fn new_with_hasher(hasher: H, num_items: num::NonZero<usize>, error_rate: f32) -> Self {
        let storage = S::new_storage(optimal_cell_count(num_items.get(), error_rate));
        Self::new_with_storage(storage, hasher, num_items)
    }
}

impl<S: AsRef<[AtomicUsize]>, H> CustomBloom<S, H> {
    pub fn new_with_storage(storage: S, hasher: H, num_items: num::NonZero<usize>) -> Self {
        let hash_count = optimal_hash_count(num_items.get(), storage.as_ref().len());
        Self::new_with_hash_count(hash_count, storage, hasher)
    }
}

impl<S, H> CustomBloom<S, H> {
    #[inline]
    pub fn new_with_hash_count(hash_count: num::NonZero<u8>, storage: S, hasher: H) -> Self {
        Self {
            storage,
            hasher,
            hash_count,
        }
    }

    #[inline]
    pub fn deconstruct(self) -> (num::NonZero<u8>, S, H) {
        let Self {
            storage,
            hasher,
            hash_count,
        } = self;
        (hash_count, storage, hasher)
    }
}

impl<S: AsRef<[AtomicUsize]>, H: Hasher> CustomBloom<S, H> {
    /// Insert `value` into the bloom filter, and return if it was (probably) already contained.
    pub fn insert<V: hash::Hash + ?Sized>(&self, value: &V) -> bool {
        let mut indices = mem::MaybeUninit::uninit();
        let indices = calc_indices(value, &self.hasher, self.hash_count, &mut indices);
        insert(self.storage.as_ref(), indices)
    }

    /// Test if `value` is (probably) contained in this bloom filter.
    pub fn contains<V: hash::Hash + ?Sized>(&self, value: &V) -> bool {
        let mut indices = mem::MaybeUninit::uninit();
        let indices = calc_indices(value, &self.hasher, self.hash_count, &mut indices);
        contains(self.storage.as_ref(), indices)
    }

    pub fn share(&self) -> SliceBloom<'_, H> {
        SliceBloom {
            storage: self.storage.as_ref(),
            hasher: self.hasher.clone(),
            hash_count: self.hash_count,
        }
    }
}

fn insert(storage: &[AtomicUsize], indices: &[usize]) -> bool {
    let total_bits = storage.len() * BITS_PER_CELL;
    let mut result = true;
    for &idx in indices {
        let idx = idx % total_bits;
        let cell = idx / BITS_PER_CELL;
        let mask = 1 << (idx % BITS_PER_CELL);

        let prev = storage[cell].fetch_or(mask, Relaxed);
        result &= (prev & mask) != 0;
    }
    result
}

fn contains(storage: &[AtomicUsize], indices: &[usize]) -> bool {
    let total_bits = storage.len() * BITS_PER_CELL;
    for &idx in indices {
        let idx = idx % total_bits;
        let cell = idx / BITS_PER_CELL;
        let mask = 1 << (idx % BITS_PER_CELL);

        if (storage[cell].load(Relaxed) & mask) == 0 {
            return false;
        }
    }
    true
}

fn calc_indices<'a, V: hash::Hash + ?Sized, H: Hasher>(
    value: &V,
    hasher: &H,
    hash_count: num::NonZero<u8>,
    indices: &'a mut mem::MaybeUninit<[usize; MAX_HASH_COUNT as usize]>,
) -> &'a [usize] {
    let indices = zero_indices(hash_count, indices);

    let mut hasher = hasher.clone();
    value.hash(&mut Digester(&mut hasher));
    hasher.finalize_xof_into(bytemuck::cast_slice_mut(indices));

    &*indices
}

fn zero_indices(
    hash_count: num::NonZero<u8>,
    indices: &mut mem::MaybeUninit<[usize; MAX_HASH_COUNT as usize]>,
) -> &mut [usize] {
    let indices = indices.write(Zeroable::zeroed());
    &mut indices[..hash_count.get().min(MAX_HASH_COUNT) as usize]
}

struct Digester<'a, H>(&'a mut H);

impl<H: Hasher> hash::Hasher for Digester<'_, H> {
    #[inline]
    fn finish(&self) -> u64 {
        0
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        self.0.update(bytes);
    }
}

pub trait Hasher: Clone {
    fn update(&mut self, data: &[u8]);

    fn finalize_xof_into(self, out: &mut [u8]);
}

pub trait BuildableStorage: AsRef<[AtomicUsize]> {
    fn new_storage(size: num::NonZero<usize>) -> Self;
}

#[cfg(feature = "alloc")]
impl BuildableStorage for alloc::boxed::Box<[AtomicUsize]> {
    fn new_storage(size: num::NonZero<usize>) -> Self {
        <alloc::vec::Vec<AtomicUsize>>::new_storage(size).into_boxed_slice()
    }
}

#[cfg(feature = "alloc")]
impl BuildableStorage for alloc::vec::Vec<AtomicUsize> {
    fn new_storage(size: num::NonZero<usize>) -> Self {
        let mut storage = alloc::vec::Vec::new();
        storage.reserve_exact(size.get());
        storage.resize_with(size.get(), AtomicUsize::default);
        storage
    }
}

#[cfg(feature = "blake3")]
impl Hasher for Blake3 {
    #[inline]
    fn update(&mut self, data: &[u8]) {
        self.update(data);
    }

    #[inline]
    fn finalize_xof_into(self, out: &mut [u8]) {
        self.finalize_xof().fill(out);
    }
}

#[test]
fn read_words() {
    extern crate std;

    use std::vec::Vec;

    const ERROR_RATE: f32 = 0.001;

    let words = std::fs::read_to_string("/usr/share/dict/words").unwrap();
    let words: Vec<_> = words
        .lines()
        .filter_map(|s| match s.trim() {
            "" => None,
            s => Some(s),
        })
        .collect();

    let mut false_positives = 0usize;
    let bloom = XofBloom::new(words.len().try_into().unwrap(), ERROR_RATE);
    for word in &words {
        if bloom.insert(word) {
            false_positives += 1;
        }
    }

    std::eprintln!(
        "false positives: {false_positives} == {:.3}%",
        false_positives as f32 / words.len() as f32 * 100.0,
    );

    assert!(false_positives <= (words.len() as f32 * ERROR_RATE).ceil() as usize);
}
