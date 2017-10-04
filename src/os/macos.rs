#![cfg(target_os = "macos")]

pub use cocoa::base::id;
pub use winit::os::macos::ActivationPolicy;
pub use winit::os::macos::MonitorIdExt;
pub use winit::os::macos::WindowBuilderExt;
pub use winit::os::macos::WindowExt;

use {Context, HeadlessContext};
use os::GlContextExt;

impl GlContextExt for Context {
    type Handle = id;

    #[inline]
    unsafe fn raw_handle(&self) -> Self::Handle {
        self.context.raw_handle()
    }
}

impl GlContextExt for HeadlessContext {
    type Handle = id;

    #[inline]
    unsafe fn raw_handle(&self) -> Self::Handle {
        self.context.raw_handle()
    }
}
