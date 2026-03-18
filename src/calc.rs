#![allow(clippy::inconsistent_digit_grouping)] // makes percentages easier to read

use core::f32::consts::LN_2;
use core::num::{FpCategory, NonZero};

#[cfg(feature = "std")]
use libm as _;
#[cfg(not(feature = "std"))]
use libm::{ceilf, logf};

/// For an expected `num_items` and an acceptable `error_rate`, how big should the backing
/// [`AtomicUsize`][core::sync::atomic::AtomicUsize] slice be?
///
/// The argument `num_items` is clamped between `1` and (a bit less than) [`isize::MAX`].
///
/// The argument `error_rate` is clamped between `0.00001` and `0.1` (0.001% to 10%).
/// If the argument is `NaN`, the lower bound is used. If the argument is negative or positive
/// infinity, then the lower or upper bound is used, respectively.
pub fn optimal_cell_count(num_items: usize, error_rate: f32) -> NonZero<usize> {
    let num_items = num_items.clamp(MIN_CELL_COUNT, MAX_CELL_COUNT);
    let error_rate = clamp_error_rate(error_rate);

    let bit_count = num_items as f32 * logf(error_rate) * const { -(LN_2 * LN_2).recip() };
    let cell_count = (ceilf(bit_count) as usize).div_ceil(BITS_PER_CELL);
    clamp_cell_count(cell_count)
}

/// For an expected `num_items` and a [`AtomicUsize`][core::sync::atomic::AtomicUsize] slice length,
/// how many hash bits should be used per entry?
///
/// The `cell_count` should be calculated with [`optimal_cell_count()`].
pub fn optimal_hash_count(num_items: usize, cell_count: usize) -> NonZero<u8> {
    let bits = (cell_count as f32) * (BITS_PER_CELL as f32);
    let hash_count = ceilf((bits / (num_items as f32)) * LN_2) as u8;
    clamp_hash_count(hash_count)
}

#[cfg(feature = "std")]
#[inline]
fn logf(x: f32) -> f32 {
    std::primitive::f32::ln(x)
}

#[cfg(feature = "std")]
#[inline]
fn ceilf(x: f32) -> f32 {
    std::primitive::f32::ceil(x)
}

const fn clamp_cell_count(cell_count: usize) -> NonZero<usize> {
    if cell_count <= MIN_CELL_COUNT {
        #[allow(clippy::unwrap_used)] // cannot fail: we know that `MIN_CELL_COUNT >= 1`
        NonZero::new(MIN_CELL_COUNT).unwrap()
    } else if cell_count >= MAX_CELL_COUNT {
        #[allow(clippy::unwrap_used)] // cannot fail: we know that `MAX_CELL_COUNT >= 1`
        NonZero::new(MAX_CELL_COUNT).unwrap()
    } else {
        #[allow(clippy::unwrap_used)] // cannot fail: we checked that `cell_count >= 1`
        NonZero::new(cell_count).unwrap()
    }
}

const fn clamp_hash_count(hash_count: u8) -> NonZero<u8> {
    if hash_count <= MIN_HASH_COUNT {
        #[allow(clippy::unwrap_used)] // cannot fail: we know that `MIN_HASH_COUNT >= 1`
        NonZero::new(MIN_HASH_COUNT).unwrap()
    } else if hash_count >= MAX_HASH_COUNT {
        #[allow(clippy::unwrap_used)] // cannot fail: we know that `MAX_HASH_COUNT >= 1`
        NonZero::new(MAX_HASH_COUNT).unwrap()
    } else {
        #[allow(clippy::unwrap_used)] // cannot fail: we checked that `hash_count >= 1`
        NonZero::new(hash_count).unwrap()
    }
}

const fn clamp_error_rate(error_rate: f32) -> f32 {
    match error_rate.classify() {
        FpCategory::Normal => error_rate.clamp(MIN_ERROR_LEVEL, MAX_ERROR_LEVEL),
        FpCategory::Nan | FpCategory::Zero | FpCategory::Subnormal => MIN_ERROR_LEVEL,
        FpCategory::Infinite => {
            if error_rate.is_sign_negative() {
                MIN_ERROR_LEVEL
            } else {
                MAX_ERROR_LEVEL
            }
        },
    }
}

const MIN_ERROR_LEVEL: f32 = 0.00_001;
const MAX_ERROR_LEVEL: f32 = 0.10_000;
const MIN_CELL_COUNT: usize = 1;
const MAX_CELL_COUNT: usize = ((isize::MAX as usize) / BITS_PER_CELL) & !0xfff;
const MIN_HASH_COUNT: u8 = 1;
pub(crate) const MAX_HASH_COUNT: u8 = 32;
pub(crate) const BITS_PER_CELL: usize = usize::BITS as usize;
