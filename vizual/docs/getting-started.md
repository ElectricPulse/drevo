# Getting started

Vizual requires nightly Rust, Tokio, a supported desktop environment, and a
GPU supported by Vello.

```sh
rustup toolchain install nightly
cargo new hello-vizual
cd hello-vizual
```

Add the dependencies:

```toml
[dependencies]
color-eyre = "0.6"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
vizual = { git = "https://github.com/ElectricPulse/vizual" }
```

Use the complete
[`hello-world`](examples/src/bin/hello-world.rs) example as `src/main.rs`,
then run:

```sh
cargo +nightly run
```

[`vizual::run`](https://docs.rs/vizual/latest/vizual/fn.run.html) owns the
calling thread while Tokio continues running asynchronous widget work.
