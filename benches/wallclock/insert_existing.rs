use std::hint::black_box;
use std::time::{Duration, Instant};

use benches::{Dict, read_dict};
use criterion::{Criterion, Throughput, criterion_group};

criterion_group!(
    name = bench_insert_existing;
    config = Criterion::default();
    targets = xofbloom, fastbloom, bloomfilter,
);

macro_rules! benchers {
    ($($name:ident),* $(,)?) => { $(
        fn $name(c: &mut Criterion) {
            let words = read_dict();

            let mut g = c.benchmark_group("insert_existing");
            let _ = g.throughput(Throughput::ElementsAndBytes {
                elements: words.len() as u64,
                bytes: words.iter().map(|s| s.len() as u64).sum(),
            });
            let _ = g.bench_function(stringify!($name), |b| {
                b.iter_custom(|iters| {
                    custom(iters, benches::filled_dict::$name, benches::insert::$name)
                });
            });
            g.finish();
        }
    )* };
}

#[inline]
#[track_caller]
fn custom<B>(
    iters: u64,
    setup: impl Fn() -> (B, Dict),
    insert: impl Fn(&mut B, Dict) -> usize,
) -> Duration {
    let mut total = Duration::default();
    for _ in 0..iters {
        let (mut bloom, words) = black_box(setup());

        let start = Instant::now();
        let false_positive = insert(&mut bloom, words);
        total += start.elapsed();

        let error_rate = false_positive as f32 / words.len() as f32;
        assert_eq!(false_positive, words.len(), "{error_rate} == 1.0");

        drop(black_box(bloom));
    }
    total
}

benchers!(xofbloom, fastbloom, bloomfilter);
