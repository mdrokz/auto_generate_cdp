# auto_generate_cdp
experimental crate to generate the Chrome Devtools Protocol.


# Usage

Cargo.toml
```

[dependencies]
serde = {version = "1", features = ["derive"]}
serde_json = '1'

[build-dependencies]
auto_generate_cdp = "0.1.3"
```

build.rs

```
use auto_generate_cdp::init;

fn main() {
  init();
}

```

this will generate `protocol.rs` when you do `cargo check` or `cargo build` in your src folder which you can use in your crate
