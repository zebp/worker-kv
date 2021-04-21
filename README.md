# worker-kv

[![Docs.rs][docs-badge]][docs-url]
[![Crates.io][crates-badge]][crates-url]
[![Unlicense][license-badge]][license-url]

[crates-badge]: https://img.shields.io/crates/v/worker-kv.svg
[crates-url]: https://crates.io/crates/worker-kv
[license-badge]: https://img.shields.io/badge/license-Unlicense-blue.svg
[license-url]: https://github.com/zebp/worker-kv/blob/master/LICENSE
[docs-badge]: https://img.shields.io/badge/docs.rs-rustdoc-green
[docs-url]: https://docs.rs/worker-kv/

Rust bindings to Cloudflare Worker [KV Stores](https://developers.cloudflare.com/workers/runtime-apis/kv) using [wasm-bindgen](https://docs.rs/wasm-bindgen) and [js-sys](https://docs.rs/js-sys).

## Example

```rust
let kv = KvStore::create("Example")?;

// Insert a new entry into the kv.
kv.put("example_key", "example_value")
    .metadata(vec![1, 2, 3, 4]) // Use some arbitrary serialiazable metadata
    .execute()
    .await?;

// NOTE: kv changes can take a minute to become visible to other workers.
// Get that same metadata.
let (value, metdata) = kv.get_with_metadata::<Vec<usize>>("example_key").await?;
```

## How do I use futures in WebAssembly?

There currently is not a way to use a [Future](https://doc.rust-lang.org/stable/std/future/trait.Future.html) natively from WebAssembly but with the [future_to_promise](https://docs.rs/wasm-bindgen-futures/0.4.22/wasm_bindgen_futures/fn.future_to_promise.html) function from [wasm_bindgen_futures](https://docs.rs/wasm_bindgen_futures) we can convert it to a standard JavaScript promise, which can be awaited in the regular JavaScript context.
