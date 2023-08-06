# glutin -  OpenGL, UTilities, and INput

A low-level library for OpenGL context creation.

[![](https://img.shields.io/crates/v/glutin.svg)](https://crates.io/crates/glutin)
[![Docs.rs](https://docs.rs/glutin/badge.svg)](https://docs.rs/glutin)

```toml
[dependencies]
glutin = "0.30.10"
```

## [Documentation](https://docs.rs/glutin)

## Contact Us

Join us in any of these:

[![Matrix](https://img.shields.io/badge/Matrix-%23winit%3Amatrix.org-blueviolet.svg)](https://matrix.to/#/#winit:matrix.org)
[![Libera.Chat](https://img.shields.io/badge/libera.chat-%23winit-red.svg)](https://web.libera.chat/#winit)

## Usage Examples

**Warning:** These are examples for `master`. You can find examples for
the latest _released version_ [here](https://github.com/rust-windowing/glutin/releases/tag/v0.30.9).

The examples use [`gl_generator`](https://crates.io/crates/gl_generator) to
generate OpenGL bindings.

### Try it!

```bash
git clone https://github.com/rust-windowing/glutin
cd glutin
cargo run --example window
```

### Usage

Glutin is an OpenGL context creation library, and doesn't directly provide
OpenGL bindings for you.

For examples, please look [here](https://github.com/rust-windowing/glutin/tree/master/glutin_examples).

Note that glutin aims at being a low-level brick in your rendering
infrastructure. You are encouraged to write another layer of abstraction
between glutin and your application.

The minimum Rust version target by glutin is `1.60.0`.

## Platform-specific notes

### Android

Be sure to handle Android's lifecycle correctly when using a `winit` window
by only creating a GL surface after `winit` raises `Event::Resumed`, and
destroy it again upon receiving `Event::Suspended`. See this in action in the
[`android.rs` example](./glutin_examples/examples/android.rs).

To compile and run the Android example on your device,
install [`cargo-apk`](https://crates.io/crates/cargo-apk)
and start the app using:

```console
$ cargo apk r -p glutin_examples --example android
```
