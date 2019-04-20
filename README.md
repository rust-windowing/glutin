# glutin -  OpenGL, UTilities and INput
Alternative to GLFW in pure Rust.

[![](https://meritbadge.herokuapp.com/glutin)](https://crates.io/crates/glutin)
[![Docs.rs](https://docs.rs/glutin/badge.svg)](https://docs.rs/glutin)
[![Build Status](https://travis-ci.org/tomaka/glutin.png?branch=master)](https://travis-ci.org/tomaka/glutin)
[![Build status](https://ci.appveyor.com/api/projects/status/cv5xewg3uchb3854/branch/master?svg=true)](https://ci.appveyor.com/project/tomaka/glutin/branch/master)

```toml
[dependencies]
glutin = "*"
```

## [Documentation](https://docs.rs/glutin)
## [0.21.0 Migration guide](https://gentz.rocks/posts/glutin-v0-21-0-migration-guide/)

## Contact Us

Join us in any of these:

[![Freenode](https://img.shields.io/badge/freenode.net-%23glutin-red.svg)](http://webchat.freenode.net?channels=%23glutin&uio=MTY9dHJ1ZSYyPXRydWUmND10cnVlJjExPTE4NSYxMj10cnVlJjE1PXRydWU7a)
[![Matrix](https://img.shields.io/badge/Matrix-%23Glutin%3Amatrix.org-blueviolet.svg)](https://matrix.to/#/#Glutin:matrix.org)
[![Gitter](https://badges.gitter.im/Join%20Chat.svg)](https://gitter.im/tomaka/glutin?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

## Usage Examples

Warning: these are examples for master. For the latest released version, 0.21, view [here.](https://github.com/rust-windowing/glutin/tree/2e816ae2654ba80eb3e201d0ce51d238cc105226)

### Try it!

```bash
git clone https://github.com/tomaka/glutin
cd glutin
cargo run --example window
```

### Usage

Glutin is an OpenGL context creation library and doesn't directly provide OpenGL bindings for you.

For examples, please look [here.](https://github.com/tomaka/glutin/tree/master/glutin_examples)

Note that glutin aims at being a low-level brick in your rendering infrastructure. You are encouraged to write another layer of abstraction between glutin and your application.

Glutin is only officially supported on the latest stable version of the Rust compiler.

## Platform-specific notes

### Android

To compile the examples for android, you have to use the `cargo apk` utility.

See [the `android-rs-glue` repository](https://github.com/tomaka/android-rs-glue) for instructions.

### Emscripten with asmjs

In order to use glutin with emscripten, start by compiling your code with `--target=asmjs-unknown-emscripten`.

Then create an HTML document that contains this:

```html
<canvas id="canvas"></canvas>
<script type="text/javascript">
var Module = {
    canvas: document.getElementById('canvas')
};
</script>
<script type="text/javascript" src="target/asmjs-unknown-emscripten/debug/..." async></script>
```

*Note: adjust the `src` element of the script to point to the .js file that was produced by the compilation.*

The `Module` object is the link between emscripten and the HTML page.
See also [this documentation](https://kripken.github.io/emscripten-site/docs/api_reference/module.html).

### X11

The plan is that glutin tries to dynamically link-to and use wayland if possible. If it doesn't work, it will try xlib instead. This is work-in-progress.

### Wayland

Due to an issue with how mesa and Wayland play together, all shared contexts must use the same events pool as each other.

## Common issues

Help! I'm receiving `NoAvailablePixelFormat`!

 - See: https://github.com/tomaka/glutin/issues/952#issuecomment-467228004

