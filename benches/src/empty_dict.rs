use crate::{BloomFilter, Dict, ERROR_RATE, FastBloom, XofBloom, read_dict};

pub fn xofbloom() -> (XofBloom, Dict) {
    let words = read_dict();
    let bloom = XofBloom::new_unseeded(read_dict().len().try_into().unwrap(), ERROR_RATE);
    (bloom, words)
}

pub fn fastbloom() -> (FastBloom, Dict) {
    let words = read_dict();
    let bits = fastbloom::optimal_size(words.len(), ERROR_RATE as _);
    let hashes = fastbloom::optimal_hashes(bits, words.len());
    let bloom = FastBloom::with_num_bits(bits).hashes(hashes);
    (bloom, words)
}

pub fn bloomfilter() -> (BloomFilter, Dict) {
    let words = read_dict();
    let bloom = BloomFilter::new_for_fp_rate(words.len(), ERROR_RATE as _).unwrap();
    (bloom, words)
}
