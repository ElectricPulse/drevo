# Getting started

Vizual requires nightly Rust, Tokio, a supported desktop environment, and a
GPU supported by Vello.

```sh
rustup toolchain install nightly
cargo new hello-drevo
cd hello-drevo
```

Add the dependencies:

```toml
[dependencies]
async-trait = "0.1"
color-eyre = "0.6"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
drevo = { git = "https://github.com/ElectricPulse/drevo" }
```

Use the complete
[`hello-world`](examples/src/bin/hello-world.rs) example as `src/main.rs`,
then run:

```sh
cargo +nightly run
```

[`drevo::run`](https://docs.rs/drevo/latest/drevo/fn.run.html) owns the
calling thread while Tokio continues running asynchronous widget work.
