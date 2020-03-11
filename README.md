# glutin -  OpenGL, UTilities and INput
A low-level library for OpenGL context creation, written in pure Rust.

[![](https://meritbadge.herokuapp.com/glutin)](https://crates.io/crates/glutin)
[![Docs.rs](https://docs.rs/glutin/badge.svg)](https://docs.rs/glutin)

```toml
[dependencies]
glutin = "0.24"
```

## [Documentation](https://docs.rs/glutin)

## Contact Us

Join us in any of these:

[![Freenode](https://img.shields.io/badge/freenode.net-%23glutin-red.svg)](http://webchat.freenode.net?channels=%23glutin&uio=MTY9dHJ1ZSYyPXRydWUmND10cnVlJjExPTE4NSYxMj10cnVlJjE1PXRydWU7a)
[![Matrix](https://img.shields.io/badge/Matrix-%23Glutin%3Amatrix.org-blueviolet.svg)](https://matrix.to/#/#Glutin:matrix.org)
[![Gitter](https://badges.gitter.im/Join%20Chat.svg)](https://gitter.im/tomaka/glutin?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

## Usage Examples

Warning: these are examples for master. For the latest released version, 0.23, view [here.](https://github.com/rust-windowing/glutin/tree/f071c722f725143d80638f1c5c12a76d9d8e1be8)

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

See [the `android-rs-glue` repository](https://github.com/rust-windowing/android-rs-glue) for instructions.

### Emscripten with asmjs

Emscripten support has been deprecated in favor of platforms like stdweb. To get an OpenGL context on these platforms, please use crates like [glow](https://crates.io/crates/glow) instead.

### X11

The plan is that glutin tries to dynamically link-to and use Wayland w/EGL if possible. If it doesn't work, it will try Xlib w/GLX follow by Xlib w/EGL instead. This is work-in-progress.

### Wayland

Due to an issue with how Mesa and Wayland play together, all shared contexts must use the same events pool as each other.
