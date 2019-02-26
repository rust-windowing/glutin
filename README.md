# glutin -  OpenGL, UTilities and INput
[![Gitter](https://badges.gitter.im/Join%20Chat.svg)](https://gitter.im/tomaka/glutin?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

[![](https://meritbadge.herokuapp.com/glutin)](https://crates.io/crates/glutin)

[![Docs.rs](https://docs.rs/glutin/badge.svg)](https://docs.rs/glutin)

Alternative to GLFW in pure Rust.

[![Build Status](https://travis-ci.org/tomaka/glutin.png?branch=master)](https://travis-ci.org/tomaka/glutin)
[![Build status](https://ci.appveyor.com/api/projects/status/cv5xewg3uchb3854/branch/master?svg=true)](https://ci.appveyor.com/project/tomaka/glutin/branch/master)

```toml
[dependencies]
glutin = "*"
```

## [Documentation](https://docs.rs/glutin)

## Try it!

```bash
git clone https://github.com/tomaka/glutin
cd glutin
cargo run --example window
```

## Usage

Glutin is an OpenGL context creation library and doesn't directly provide OpenGL bindings for you.

```toml
[dependencies]
gl = "*"
```

```rust
extern crate gl;
extern crate glutin;

use glutin::dpi::*;
use glutin::ContextTrait;

fn main() {
    let mut el = glutin::EventsLoop::new();
    let wb = glutin::WindowBuilder::new()
        .with_title("Hello, world!")
        .with_dimensions(LogicalSize::new(1024.0, 768.0));
    let combined_context = glutin::ContextBuilder::new()
        .with_vsync(true)
        .build_combined(wb, &el)
        .unwrap();

    unsafe {
        combined_context.make_current().unwrap();
    }

    unsafe {
        gl::load_with(|symbol| combined_context.get_proc_address(symbol) as *const _);
        gl::ClearColor(0.0, 1.0, 0.0, 1.0);
    }

    let mut running = true;
    while running {
        el.poll_events(|event| {
            match event {
                glutin::Event::WindowEvent{ event, .. } => match event {
                    glutin::WindowEvent::CloseRequested => running = false,
                    glutin::WindowEvent::Resized(logical_size) => {
                        let dpi_factor = combined_context.get_hidpi_factor();
                        combined_context.resize(logical_size.to_physical(dpi_factor));
                    },
                    _ => ()
                },
                _ => ()
            }
        });

        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }

        combined_context.swap_buffers().unwrap();
    }
}
```

Note that glutin aims at being a low-level brick in your rendering infrastructure. You are encouraged to write another layer of abstraction between glutin and your application.

glutin is only officially supported on the latest stable version of the Rust compiler.

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

The plan is that glutin tries to dynamically link-to and use wayland if possible. If it doesn't work, it will try xlib instead. If it doesn't work, it will try libcaca. This is work-in-progress.
 
### Wayland

Due to an issue with how mesa and Wayland play together, all shared contexts must use the same events pool as each other.

## Common issues

Help! I'm receiving `NoAvailablePixelFormat`!

 - See: https://github.com/tomaka/glutin/issues/952#issuecomment-467228004

