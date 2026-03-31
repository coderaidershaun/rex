# Performance-Oriented Crate Guide

A categorized reference for crate selection when performance matters. Organized by domain. Each entry includes when to reach for it and when to avoid it.

## Table of Contents

1. [Data Structures](#data-structures)
2. [Concurrency & Synchronization](#concurrency--synchronization)
3. [Channels & Message Passing](#channels--message-passing)
4. [Serialization & Data Formats](#serialization--data-formats)
5. [Tabular Data & Analytics](#tabular-data--analytics)
6. [Memory Management](#memory-management)
7. [Hashing](#hashing)
8. [Numerics & SIMD](#numerics--simd)
9. [I/O & Networking](#io--networking)
10. [String Processing](#string-processing)
11. [Collections & Containers](#collections--containers)

---

## Data Structures

### `rustc-hash` (FxHashMap / FxHashSet)
- **What:** Fast hash map/set using Firefox's FxHash algorithm
- **When:** Small keys (integers, short strings), non-cryptographic use cases. 2-5x faster than default SipHash for small keys
- **Avoid when:** Keys are user-controlled (DoS-vulnerable), need cryptographic properties
- **Deps:** Zero dependencies

### `indexmap`
- **What:** Hash map that preserves insertion order
- **When:** Need deterministic iteration order + O(1) lookup. Good for config, ASTs, JSON-like structures
- **Avoid when:** Pure performance — slightly slower than HashMap for lookups
- **Deps:** Minimal

### `slotmap`
- **What:** Generational arena with O(1) insert/remove/lookup
- **When:** Graph-like structures, ECS, any case where you'd use indices into a Vec but need stable handles and safe removal
- **Avoid when:** Simple sequential access patterns where Vec suffices

### `smallvec`
- **What:** Vec that stores small arrays inline (stack) before spilling to heap
- **When:** Collections that are usually small (1-8 items) but occasionally large. Avoids heap allocation in the common case
- **Avoid when:** Collections are always large, or always empty

### `tinyvec`
- **What:** Similar to smallvec but 100% safe code (no unsafe)
- **When:** Same use case as smallvec when you want to avoid unsafe in your dependency tree
- **Avoid when:** Need the last bit of performance (smallvec's unsafe is well-audited and slightly faster)

### `bitvec`
- **What:** Bit-level addressing for slices and vectors
- **When:** Compact boolean arrays, bitfield manipulation, packed data
- **Avoid when:** `fixedbitset` is enough (simpler API for pure set operations)

### `fixedbitset`
- **What:** Fixed-size bitset with set operations
- **When:** Tracking presence/absence in a known-size universe, graph algorithms
- **Avoid when:** Need dynamic resizing or bit-level addressing (use `bitvec`)

### `im`
- **What:** Persistent/immutable data structures (Vector, HashMap, OrdMap) with structural sharing
- **When:** Need cheap clones of large structures (undo history, concurrent reads of snapshots)
- **Avoid when:** Mutable-only workloads — persistent structures have overhead vs their mutable counterparts

### `slab`
- **What:** Pre-allocated storage with stable keys (usize indices)
- **When:** Object pools, connection tracking, timer wheels — anywhere you need O(1) insert/remove with stable handles
- **Avoid when:** Need generational safety (use `slotmap` instead)

---

## Concurrency & Synchronization

### `parking_lot`
- **What:** Faster Mutex, RwLock, Condvar, Once replacements
- **When:** Always. Drop-in replacement for std sync primitives. Smaller, faster, no poisoning
- **Avoid when:** You explicitly want poisoning semantics (rare)

### `crossbeam`
- **What:** Suite of concurrency tools: channels, queues, epoch GC, scoped threads
- **When:** Any concurrent data structure need. The `crossbeam::channel` is the go-to channel. `crossbeam::queue::SegQueue` for lock-free MPMC queues. Scoped threads for borrowing stack data across threads
- **Avoid when:** You only need async channels (use tokio's)

### `arc-swap`
- **What:** Atomically swappable Arc pointers
- **When:** Config hot-reload, read-heavy shared state where writes are infrequent. Readers never block. Near-zero overhead for reads
- **Avoid when:** Writes are frequent (each swap allocates a new Arc)

### `dashmap`
- **What:** Concurrent hash map (sharded internally)
- **When:** Read-heavy concurrent access from many threads. Good default concurrent map
- **Avoid when:** Write-heavy (consider partitioning instead), or single-threaded (just use HashMap)

### `papaya`
- **What:** Lock-free concurrent hash map using more modern techniques
- **When:** Evaluating alternatives to dashmap. Newer, potentially better for mixed read/write workloads
- **Avoid when:** Need battle-tested maturity (dashmap has wider adoption)

### `rayon`
- **What:** Data parallelism with work-stealing thread pool
- **When:** CPU-bound parallel iteration (`par_iter`), parallel sorting, any embarrassingly parallel workload
- **Avoid when:** I/O-bound work (use tokio), fine-grained task control needed

### `thread_local`
- **What:** Per-thread storage with `thread_local!` macro (std) or the `thread_local` crate for more ergonomic per-thread data
- **When:** Accumulating per-thread results to merge later, avoiding contention by partitioning state per thread
- **Avoid when:** Threads are short-lived (thread-local setup cost)

---

## Channels & Message Passing

### `crossbeam::channel`
- **What:** Bounded/unbounded MPMC channels with select
- **When:** Default choice for thread-to-thread communication. Fast, flexible, well-tested
- **Bounded vs unbounded:** Always prefer bounded with explicit backpressure unless you have a specific reason for unbounded

### `flume`
- **What:** MPMC channel with async support
- **When:** Need sync-to-async bridging. Clean API, good performance
- **Avoid when:** Don't need async (crossbeam is more mature for pure sync)

### `kanal`
- **What:** High-performance channel
- **When:** Channel throughput is your measured bottleneck and crossbeam isn't enough
- **Avoid when:** Default cases — crossbeam is battle-tested

### `rtrb`
- **What:** Lock-free SPSC ring buffer
- **When:** Exactly one producer, one consumer. Real-time audio, low-latency pipelines. Bounded, allocation-free after init
- **Avoid when:** Multiple producers or consumers

### `ringbuf`
- **What:** SPSC ring buffer with sync and async variants
- **When:** Same as rtrb but need async support or prefer this API
- **Avoid when:** Multiple producers/consumers

### `bus`
- **What:** Broadcast channel (one sender, many receivers each get all messages)
- **When:** Fan-out patterns, event buses, pub-sub within a process
- **Avoid when:** Need MPSC or point-to-point communication

### `tokio::sync::mpsc`
- **What:** Async MPSC channel
- **When:** Inside tokio runtime, async producer/consumer patterns
- **Avoid when:** Pure sync code (use crossbeam)

---

## Serialization & Data Formats

### `serde` + `serde_json`
- **What:** The Rust serialization framework + JSON
- **When:** Default for any serialization need. JSON for APIs, configs, interchange
- **Avoid when:** Performance-critical deserialization (see simd-json, sonic-rs)

### `simd-json`
- **What:** SIMD-accelerated JSON parser
- **When:** JSON parsing is a bottleneck. 2-4x faster than serde_json
- **Note:** Requires mutable input buffer (in-place parsing)

### `sonic-rs`
- **What:** High-performance JSON library with SIMD
- **When:** Maximum JSON throughput needed
- **Avoid when:** Need proven stability (newer than simd-json)

### `rkyv`
- **What:** Zero-copy deserialization
- **When:** Need absolute fastest deserialization. Data is read directly from bytes without copying or parsing. Excellent for mmap'd files, IPC, caches
- **Avoid when:** Need schema evolution, cross-language compatibility, or human-readable format

### `bitcode`
- **What:** Compact binary serialization
- **When:** Need small binary size + good speed. Simpler than rkyv, good for network protocols
- **Avoid when:** Need zero-copy (use rkyv) or human readability

### `prost`
- **What:** Protocol Buffers implementation
- **When:** Cross-language IPC, schema evolution, gRPC
- **Avoid when:** Rust-only systems where rkyv or bitcode are faster and simpler

### `flatbuffers`
- **What:** Zero-copy cross-language serialization
- **When:** Need zero-copy + cross-language. Game engines, performance-critical IPC
- **Avoid when:** Rust-only (rkyv is more ergonomic), simple use cases

### `bincode`
- **What:** Binary serialization via serde
- **When:** Simple binary encoding, caches, IPC within Rust. Easy to use (just serde derives)
- **Avoid when:** Need speed (rkyv/bitcode are faster), need cross-language (use prost/flatbuffers)

---

## Tabular Data & Analytics

### `polars`
- **What:** DataFrame library with lazy evaluation, query optimization, multi-threaded execution
- **When:** Complex analytics: joins, group-by, window functions, lazy pipelines, pivot tables. When you'd otherwise write 500+ lines of manual columnar processing
- **Avoid when:** Just reading/writing files, simple filtering, or you're building a library (heavy dependency: ~200+ transitive crates)
- **Weight:** Heavy. Accept this weight only when the analytical complexity justifies it

### `arrow` (arrow-rs)
- **What:** Apache Arrow in-memory columnar format with compute kernels
- **When:** Columnar data processing, interop with Arrow ecosystem (Spark, DuckDB, Flight), building your own query engine
- **Avoid when:** Don't need columnar — a Vec of structs is simpler

### `parquet` (arrow-rs parquet crate)
- **What:** Parquet file reader/writer
- **When:** Need to read/write Parquet files. Much lighter than pulling in polars just for file I/O
- **Pairs with:** `arrow` for in-memory processing after reading

### `csv`
- **What:** CSV reader/writer
- **When:** CSV files, obviously. Fast, zero-copy parsing available, serde integration
- **Note:** For large CSVs where you need analytics, read with `csv` → process with arrow or polars

### `datafusion`
- **What:** SQL query engine built on Arrow
- **When:** Need SQL over Arrow data, building analytics tools, custom query engines
- **Avoid when:** Overkill for simple processing — check if polars or raw arrow is enough

---

## Memory Management

### `bumpalo`
- **What:** Bump allocator — fast allocation, no individual deallocation
- **When:** Short-lived allocation bursts (parsing, request handling). Allocate many small objects, free them all at once. Much faster than global allocator for this pattern
- **Avoid when:** Objects need individual lifetimes

### `typed-arena`
- **What:** Typed arena allocator
- **When:** Many objects of the same type with the same lifetime (AST nodes, graph nodes)
- **Avoid when:** Mixed types (use bumpalo) or need deallocation

### `mimalloc` / `jemalloc`
- **What:** Alternative global allocators
- **When:** `mimalloc` — general purpose, good for multi-threaded allocation-heavy workloads. `jemalloc` — proven in production at scale (Firefox, Redis). Both can improve throughput 10-30% for allocation-heavy programs
- **How:** Set as global allocator, measure before/after

### `bytes`
- **What:** Reference-counted byte buffers with zero-copy slicing
- **When:** Network protocols, I/O pipelines. Shared ownership of byte slices without copying. Core building block of tokio ecosystem
- **Avoid when:** Single-owner bytes (just use Vec<u8>)

---

## Hashing

### `rustc-hash`
- **What:** FxHash — fast, non-cryptographic
- **When:** Hash maps/sets with integer or short string keys. Default choice for internal data structures
- **Avoid when:** User-controlled keys (HashDoS vulnerable)

### `ahash`
- **What:** Fast, DoS-resistant hash (used as HashMap default in newer Rust)
- **When:** Need speed + DoS resistance. Good middle ground
- **Note:** Already the default hasher for `HashMap` in recent Rust versions

### `xxhash-rust`
- **What:** XXHash — extremely fast non-cryptographic hash
- **When:** Checksums, content-addressable storage, deduplication
- **Avoid when:** Need cryptographic properties

### `blake3`
- **What:** Cryptographic hash, SIMD-accelerated, parallelizable
- **When:** Need a cryptographic hash that's also fast. File integrity, content hashing
- **Note:** Much faster than SHA-256 while being cryptographically secure

---

## Numerics & SIMD

### `std::simd` (nightly) / `packed_simd2`
- **What:** Portable SIMD
- **When:** Explicit vectorization of numeric kernels
- **Note:** Nightly-only for std::simd. Consider `pulp` for stable SIMD

### `ndarray`
- **What:** N-dimensional arrays with broadcasting, slicing
- **When:** Scientific computing, matrix operations, signal processing. Rust's NumPy equivalent
- **Avoid when:** Simple 1D processing (Vec is fine)

### `nalgebra`
- **What:** Linear algebra library
- **When:** Geometry, physics, graphics, robotics. Statically-sized matrices when dimensions are known at compile time
- **Avoid when:** Large dynamically-sized matrices (use ndarray or faer)

### `faer`
- **What:** High-performance linear algebra
- **When:** Dense matrix operations where you need LAPACK-level performance in pure Rust. Faster than nalgebra for large matrices
- **Avoid when:** Small fixed-size matrices (nalgebra is better optimized for those)

---

## I/O & Networking

### `tokio`
- **What:** Async runtime
- **When:** Network services, concurrent I/O, anything with many simultaneous I/O operations
- **Avoid when:** Pure CPU-bound work (use rayon), simple CLI tools (may be overkill)

### `mio`
- **What:** Low-level I/O event loop
- **When:** Building your own async runtime or need raw epoll/kqueue control
- **Avoid when:** 99% of cases — use tokio

### `memmap2`
- **What:** Memory-mapped files
- **When:** Large files you want to access without reading into memory. Random access patterns, shared memory IPC
- **Caution:** Unsafe if file is modified externally while mapped

### `io-uring` (tokio-uring)
- **What:** Linux io_uring interface
- **When:** Maximum I/O throughput on Linux. File I/O especially benefits
- **Avoid when:** Cross-platform needed, or tokio's regular I/O is fast enough

---

## String Processing

### `compact_str`
- **What:** String type that stores short strings inline (up to 24 bytes on 64-bit)
- **When:** Many short strings that would otherwise allocate. Measurable win when your profiler shows String allocation is a bottleneck
- **Avoid when:** Strings are typically long, or allocation isn't your bottleneck

### `smol_str`
- **What:** Immutable string with inline storage for small strings
- **When:** Interned-style strings, identifiers, keys. O(1) clone
- **Avoid when:** Need mutability

### `aho-corasick`
- **What:** Multi-pattern string matching
- **When:** Searching for many patterns simultaneously. Much faster than running regex N times
- **Note:** Used internally by the regex crate

### `regex`
- **What:** Regular expressions
- **When:** Pattern matching. Well-optimized, but for simple substring search, `str::contains` or `memchr` is faster
- **Avoid when:** Simple operations that str methods handle

### `memchr`
- **What:** SIMD-accelerated byte/character search
- **When:** Finding bytes in slices — faster than naive iteration. Used by many other crates internally
- **Note:** The `memmem` module does SIMD substring search

---

## Collections & Containers

### `arrayvec`
- **What:** Fixed-capacity Vec on the stack
- **When:** Known maximum size, want to avoid heap allocation. Great in no_std
- **Avoid when:** Size not known at compile time

### `heapless`
- **What:** Static-capacity data structures (Vec, String, HashMap, etc.)
- **When:** Embedded/no_std environments, real-time systems where heap allocation is forbidden
- **Avoid when:** Standard environments where alloc is fine

### `ecow`
- **What:** Economical clone-on-write smart pointer
- **When:** Data that's usually shared but occasionally mutated. Cheaper than Arc<Mutex<T>> for this pattern
- **Avoid when:** Data is always mutated (just own it)
