use std::hint::black_box;

use crate::{BloomFilter, Dict, FastBloom, XofBloom};

pub fn xofbloom(bloom: &mut XofBloom, words: Dict) -> usize {
    let mut false_positive = 0usize;
    for word in black_box(words) {
        false_positive += bloom.insert(word) as usize;
    }
    false_positive
}

pub fn fastbloom(bloom: &mut FastBloom, words: Dict) -> usize {
    let mut false_positive = 0usize;
    for word in black_box(words) {
        false_positive += bloom.insert(word) as usize;
    }
    false_positive
}

pub fn bloomfilter(bloom: &mut BloomFilter, words: Dict) -> usize {
    let mut false_positive = 0usize;
    for word in black_box(words) {
        false_positive += bloom.check_and_set(word) as usize;
    }
    false_positive
}
