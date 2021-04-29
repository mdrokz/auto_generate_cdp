# auto_generate_cdp
experimental crate to generate the Chrome Devtools Protocol.


# Usage

Cargo.toml
```
[build-dependencies]
auto_generate_cdp = "0.1.1"
```

build.rs

```
use auto_generate_cdp::init;

fn main() {
  init();
}

```

this will generate `protocol.rs` in your src folder which you can use in your crate
