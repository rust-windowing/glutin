fn main() {
    #[cfg(egl_backend)]
    example::run();
}

#[cfg(egl_backend)]
mod example {
    use std::fs::OpenOptions;
    use std::path::Path;

    use glutin::api::egl::device::Device;
    use glutin::api::egl::display::Display;
    use glutin::config::{ConfigSurfaceTypes, ConfigTemplate, ConfigTemplateBuilder};
    use glutin::context::{ContextApi, ContextAttributesBuilder};
    use glutin::prelude::*;
    use glutin_examples::{gl, Renderer};

    const IMG_PATH: &str = concat!(env!("OUT_DIR"), "/egl_device.png");

    pub fn run() {
        let devices = Device::query_devices().expect("Failed to query devices").collect::<Vec<_>>();

        for (index, device) in devices.iter().enumerate() {
            println!(
                "Device {}: Name: {} Vendor: {}",
                index,
                device.name().unwrap_or("UNKNOWN"),
                device.vendor().unwrap_or("UNKNOWN")
            );
        }

        let device = devices.first().expect("No available devices");

        // Create a display using the device.
        let display =
            unsafe { Display::with_device(device, None) }.expect("Failed to create display");

        let template = config_template();
        let config = unsafe { display.find_configs(template) }
            .unwrap()
            .reduce(
                |config, acc| {
                    if config.num_samples() > acc.num_samples() {
                        config
                    } else {
                        acc
                    }
                },
            )
            .expect("No available configs");

        println!("Picked a config with {} samples", config.num_samples());

        // Context creation.
        //
        // In particular, since we are doing offscreen rendering we have no raw window
        // handle to provide.
        let context_attributes = ContextAttributesBuilder::new().build(None);

        // Since glutin by default tries to create OpenGL core context, which may not be
        // present we should try gles.
        let fallback_context_attributes =
            ContextAttributesBuilder::new().with_context_api(ContextApi::Gles(None)).build(None);

        let not_current = unsafe {
            display.create_context(&config, &context_attributes).unwrap_or_else(|_| {
                display
                    .create_context(&config, &fallback_context_attributes)
                    .expect("failed to create context")
            })
        };

        // Make the context current for rendering
        let _context = not_current.make_current_surfaceless().unwrap();
        let renderer = Renderer::new(&display);

        // Create a framebuffer for offscreen rendering since we do not have a window.
        let mut framebuffer = 0;
        let mut renderbuffer = 0;
        unsafe {
            renderer.GenFramebuffers(1, &mut framebuffer);
            renderer.GenRenderbuffers(1, &mut renderbuffer);
            renderer.BindFramebuffer(gl::FRAMEBUFFER, framebuffer);
            renderer.BindRenderbuffer(gl::RENDERBUFFER, renderbuffer);
            renderer.RenderbufferStorage(gl::RENDERBUFFER, gl::RGBA, 1280, 720);
            renderer.FramebufferRenderbuffer(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::RENDERBUFFER,
                renderbuffer,
            );
        }

        renderer.resize(1280, 720);
        renderer.draw();

        let mut buffer = Vec::<u8>::with_capacity(1280 * 720 * 4);
        unsafe {
            // Wait for the previous commands to finish before reading from the framebuffer.
            renderer.Finish();
            // Download the framebuffer contents to the buffer.
            renderer.ReadPixels(
                0,
                0,
                1280,
                720,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                buffer.as_mut_ptr() as *mut _,
            );
            buffer.set_len(1280 * 720 * 4);
        }

        let path = Path::new(IMG_PATH);
        let file = OpenOptions::new().write(true).create(true).open(path).unwrap();

        let mut encoder = png::Encoder::new(file, 1280, 720);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_color(png::ColorType::Rgba);
        let mut png_writer = encoder.write_header().unwrap();

        png_writer.write_image_data(&buffer[..]).unwrap();
        png_writer.finish().unwrap();
        println!("Output rendered to: {}", path.display());

        unsafe {
            // Unbind the framebuffer and renderbuffer before deleting.
            renderer.BindFramebuffer(gl::DRAW_FRAMEBUFFER, 0);
            renderer.BindRenderbuffer(gl::RENDERBUFFER, 0);
            renderer.DeleteFramebuffers(1, &framebuffer);
            renderer.DeleteRenderbuffers(1, &renderbuffer);
        }
    }

    fn config_template() -> ConfigTemplate {
        ConfigTemplateBuilder::default()
            .with_alpha_size(8)
            // Offscreen rendering has no support window surface support.
            .with_surface_type(ConfigSurfaceTypes::empty())
            .build()
    }
}
