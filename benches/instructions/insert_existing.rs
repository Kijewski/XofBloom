use std::hint::black_box;

use benches::Dict;
use gungraun::{library_benchmark, library_benchmark_group};

library_benchmark_group!(
    name = bench_insert_existing,
    benchmarks = [xofbloom, fastbloom, bloomfilter]
);

macro_rules! benchers {
    ($($name:ident: $ty:ident),* $(,)?) => { $(
        #[library_benchmark]
        #[bench::$name(setup = benches::filled_dict::$name, teardown = drop)]
        fn $name(setup: (benches::$ty, Dict)) -> benches::$ty {
            let (mut bloom, words) = black_box(setup);
            let false_positive = benches::insert::$name(&mut bloom, words);

            let error_rate = false_positive as f32 / words.len() as f32;
            assert_eq!(false_positive, words.len(), "{error_rate} == 1.0");

            black_box(bloom)
        }
    )* };
}

benchers! {
    xofbloom: XofBloom,
    fastbloom: FastBloom,
    bloomfilter: BloomFilter,
}
