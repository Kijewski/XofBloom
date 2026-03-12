use std::hint::black_box;

use benches::{Dict, ERROR_RATE};
use gungraun::{library_benchmark, library_benchmark_group};

library_benchmark_group!(
    name = bench_build,
    benchmarks = [xofbloom, fastbloom, bloomfilter]
);

macro_rules! benchers {
    ($($name:ident: $ty:ident),* $(,)?) => { $(
        #[library_benchmark]
        #[bench::$name(setup = benches::empty_dict::$name, teardown = check_error_rate)]
        fn $name(setup: (benches::$ty, Dict)) -> (benches::$ty, usize, usize) {
            let (mut bloom, words) = black_box(setup);
            let false_positive = benches::insert::$name(&mut bloom, words);
            (bloom, words.len(), false_positive)
        }
    )* };
}

#[track_caller]
fn check_error_rate<T>(teardown: (T, usize, usize)) {
    let (_, len, false_positive) = black_box(teardown);
    let error_rate = false_positive as f32 / len as f32;
    assert!(error_rate <= ERROR_RATE, "{error_rate} <= {ERROR_RATE}");
}

benchers! {
    xofbloom: XofBloom,
    fastbloom: FastBloom,
    bloomfilter: BloomFilter,
}
