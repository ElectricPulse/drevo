# Getting started

Drevo requires Rust, Tokio, a supported desktop environment, and a GPU
supported by Vello.

## Run the image gallery

The [`image-gallery`](../examples/image-gallery.rs) example is included in the
Drevo repository. Build and run it in release mode from the repository root:

```sh
cargo run -p drevo --release --example image-gallery
```

It displays a fixed icon card and a draggable icon. Drag the latter without
letting it leave the gallery or overlap the fixed card.

## Add Drevo to an application

```sh
cargo new hello-drevo
cd hello-drevo
```

Add the dependencies:

```toml
[dependencies]
async-trait = "0.1"
color-eyre = "0.6"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
drevo = "0.1.7"
```

Use the [minimal application](../../README.md#quick-start) as a starting point
for `src/main.rs`.

[`drevo::run`](https://docs.rs/drevo/latest/drevo/fn.run.html) owns the
calling thread while Tokio continues running asynchronous widget work.
