fn main() {
    #[cfg(all(egl_backend))]
    example::run();
}

#[cfg(all(egl_backend))]
mod example {
    use glutin::api::egl::display::Display;
    use glutin::config::ConfigTemplate;
    use glutin::context::{ContextApi, ContextAttributesBuilder};
    use glutin::display::{GetDisplayExtensions, GlDisplay};
    use glutin::prelude::GlConfig;
    use raw_window_handle::HasRawDisplayHandle;
    use winit::event_loop::EventLoop;

    pub fn run() {
        // We won't be displaying anything, but we will use winit to get
        // access to some sort of platform display.
        let event_loop = EventLoop::new().unwrap();

        // Create the display for the platform.
        let display = unsafe { Display::new(event_loop.raw_display_handle()) }.unwrap();

        if !display.extensions().contains("EGL_KHR_fence_sync") {
            eprintln!("EGL implementation does not support fence sync objects");
            return;
        }

        // Now we need a context to draw to create a sync object.
        let template = ConfigTemplate::default();
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

        let context_attributes =
            ContextAttributesBuilder::new().with_context_api(ContextApi::Gles(None)).build(None);
        let not_current = unsafe { display.create_context(&config, &context_attributes) }.unwrap();

        // Make the context current, since we are not rendering we can do a surfaceless
        // bind.
        let _context = not_current.make_current_surfaceless().unwrap();

        // Now a sync object can be created.
        let sync = display.create_sync(false).unwrap();

        // The sync object at this point is inserted into the command stream for the GL
        // context.
        //
        // However we aren't recording any commands so the fence would already be
        // signalled. Effecitvely it isn't useful to test the signalled value here.
        sync.is_signalled().unwrap();

        #[cfg(unix)]
        {
            if display.extensions().contains("EGL_ANDROID_native_fence_sync") {
                use std::os::unix::prelude::AsFd;

                println!("EGL Android native fence sync is supported");

                // Glutin also supports exporting a sync fence.
                // Firstly the sync must be a native fence.
                let sync = display.create_sync(true).unwrap();

                // An exported Sync FD can then be used in many ways, including:
                // - Send the Sync FD to another processe to synchronize rendering
                // - Import the Sync FD into another EGL Display
                // - Import the Sync FD into Vulkan using VK_KHR_external_fence_fd.
                let sync_fd = sync.export_sync_fd().expect("Export failed");

                // To show that an exported sync fd can be imported, we will reimport the sync
                // fd we just exported.
                let _imported_sync = display.import_sync(sync_fd.as_fd()).expect("Import failed");
            }
        }
    }
}
