# XofBloom

A remarkably slow Bloom filter implementation using the extendable output of BLAKE3.

The implementation should provide additional safety by employing a cryptographic hash function,
but in practice there is most likely never a reason why you would need this safety.
Also, other implementations are much more likely to be properly reviewed … or even correct.

The only advantage over other implementations is that the underlying memory can be shared.
It uses atomic operations, so it can be used concurrently without locks in multiple threads.
Or, if you are adventurous, even across multiple processes, if you use `mmap` backed memory.

The implementation is (on my system) about half as fast as [`bloomfilter`],
and 6 times slower than [`fastbloom`].
The latter library provides atomic bloom filters, too, so it can easily be used across threads,
but not as easily across processes as this library.

[`bloomfilter`]: <https://lib.rs/crates/bloomfilter>
[`fastbloom`]: <https://lib.rs/crates/fastbloom>
