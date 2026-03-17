#![allow(missing_docs)]

mod build;
mod insert_existing;

use gungraun::main;

use crate::build::bench_build;
use crate::insert_existing::bench_insert_existing;

main!(library_benchmark_groups = [bench_build, bench_insert_existing]);
