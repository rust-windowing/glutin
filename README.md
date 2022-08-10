# glutin -  OpenGL, UTilities and INput
A low-level library for OpenGL context creation, written in pure Rust.

[![](https://img.shields.io/crates/v/glutin.svg)](https://crates.io/crates/glutin)
[![Docs.rs](https://docs.rs/glutin/badge.svg)](https://docs.rs/glutin)

```toml
[dependencies]
glutin = "0.29.1"
```

## [Documentation](https://docs.rs/glutin)

## Contact Us

Join us in any of these:

[![Matrix](https://img.shields.io/badge/Matrix-%23rust--windowing%3Amatrix.org-blueviolet.svg)](https://matrix.to/#/#rust-windowing:matrix.org)
[![Libera.Chat](https://img.shields.io/badge/libera.chat-%23winit-red.svg)](https://web.libera.chat/#winit)

## Usage Examples

Warning: these are examples for master. For the latest released version you can
find them [here](https://github.com/rust-windowing/glutin/releases/tag/v0.29.1).

The examples use [gl_generator](https://crates.io/crates/gl_generator) to generate OpenGL bindings.

### Try it!

```bash
git clone https://github.com/rust-windowing/glutin
cd glutin
cargo run --example window
```

## Common issues

Please refer to [ISSUES.md.](ISSUES.md)

### Usage

Glutin is an OpenGL context creation library and doesn't directly provide OpenGL bindings for you.

For examples, please look [here.](https://github.com/rust-windowing/glutin/tree/master/glutin_examples)

Note that glutin aims at being a low-level brick in your rendering infrastructure. You are encouraged to write another layer of abstraction between glutin and your application.

Glutin is only officially supported on the latest stable version of the Rust compiler.

## Platform-specific notes

### Android

To compile the examples for android, you have to use the `cargo apk` utility.

See [`cargo-apk` in the `android-ndk-rs` repository](https://github.com/rust-windowing/android-ndk-rs/tree/master/cargo-apk) for instructions.

### X11

The plan is that glutin tries to dynamically link-to and use Wayland w/EGL if possible. If it doesn't work, it will try Xlib w/GLX follow by Xlib w/EGL instead. This is work-in-progress.

### Wayland

Due to an issue with how Mesa and Wayland play together, all shared contexts must use the same events pool as each other.
