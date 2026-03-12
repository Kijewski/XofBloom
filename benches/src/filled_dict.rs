use crate::{BloomFilter, Dict, FastBloom, XofBloom};

pub fn xofbloom() -> (XofBloom, Dict) {
    let (mut bloom, words) = crate::empty_dict::xofbloom();
    crate::insert::xofbloom(&mut bloom, words);
    (bloom, words)
}

pub fn fastbloom() -> (FastBloom, Dict) {
    let (mut bloom, words) = crate::empty_dict::fastbloom();
    crate::insert::fastbloom(&mut bloom, words);
    (bloom, words)
}

pub fn bloomfilter() -> (BloomFilter, Dict) {
    let (mut bloom, words) = crate::empty_dict::bloomfilter();
    crate::insert::bloomfilter(&mut bloom, words);
    (bloom, words)
}
