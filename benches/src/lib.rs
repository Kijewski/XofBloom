pub mod empty_dict;
pub mod filled_dict;
pub mod insert;

use std::sync::OnceLock;

pub type Dict = &'static [&'static str];
pub type XofBloom = xofbloom::XofBloom;
pub type BloomFilter = bloomfilter::Bloom<&'static str>;
pub type FastBloom = fastbloom::AtomicBloomFilter;

pub const ERROR_RATE: f32 = 0.001;

pub fn read_dict() -> Dict {
    static WORDS: OnceLock<Vec<&'static str>> = OnceLock::new();
    WORDS.get_or_init(|| {
        static DICT: OnceLock<String> = OnceLock::new();
        DICT.get_or_init(|| std::fs::read_to_string("/usr/share/dict/words").unwrap())
            .lines()
            .filter_map(|s| match s.trim() {
                "" => None,
                s => Some(s),
            })
            .take(10_000)
            .collect()
    })
}
