# auto_generate_cdp
[![Docs](https://docs.rs/auto_generate_cdp/badge.svg)](https://docs.rs/auto_generate_cdp)
[![Crates.io](https://img.shields.io/crates/v/auto_generate_cdp.svg?maxAge=2592000)](https://crates.io/crates/auto_generate_cdp)

An experimental crate to generate the Chrome Devtools Protocol.

[![Contributors](https://img.shields.io/github/contributors/mdrokz/auto_generate_cdp.svg)](https://github.com/mdrokz/auto_generate_cdp/graphs/contributors)

# Usage

Add the following to your `Cargo.toml`:

```toml
[dependencies]
serde = {version = "1", features = ["derive"]}
serde_json = '1'

[build-dependencies]
auto_generate_cdp = {version = "0.3.4",default-features = false}
```

To generate the protocol, add the following to your `build/build.rs` script.

```rust
use auto_generate_cdp::init;

fn main() {
  init();
}
```

This will generate `protocol.rs` in your `$OUT_DIR` folder when you run `$ cargo check` or `$ cargo build`. Use like:


```rust
// src/protocol.rs

include!(concat!(env!("OUT_DIR"), "/protocol.rs"));

```

```rust
// src/main.rs

mod protocol;

fn main() {
  // protocol module contains the definitions now
}
```
