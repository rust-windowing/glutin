#![cfg(target_os = "macos")]

use WindowBuilder;

pub use winit::os::macos::ActivationPolicy;

/// Additional methods on `WindowBuilder` that are specific to MacOS.
pub trait WindowBuilderExt<'a> {
    fn with_activation_policy(mut self, activation_policy: ActivationPolicy) -> WindowBuilder<'a>;
}

impl<'a> WindowBuilderExt<'a> for WindowBuilder<'a> {
    /// Sets the activation policy for the window being built
    #[inline]
    fn with_activation_policy(mut self, activation_policy: ActivationPolicy) -> WindowBuilder<'a> {
        use winit::os::macos::WindowBuilderExt;

        self.winit_builder = self.winit_builder.with_activation_policy(activation_policy);
        self
    }
}
