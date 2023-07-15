use glutin::context::PossiblyCurrentContext;
use glutin::surface::{
    GlSurface, ResizeableSurface, Surface, SurfaceAttributes, SurfaceAttributesBuilder,
    SurfaceTypeTrait, WindowSurface,
};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::num::NonZeroU32;
use winit::window::Window;

/// [`Window`] extensions for working with [`glutin`] surfaces.
pub trait GlWindow: HasWindowHandle + Sized {
    /// Build the surface attributes suitable to create a window surface.
    ///
    /// # Panics
    /// Panics if either window inner dimension is zero.
    ///
    /// # Example
    /// ```no_run
    /// use glutin_winit::GlWindow;
    /// # let winit_window: winit::window::Window = unimplemented!();
    ///
    /// let attrs = winit_window.build_surface_attributes(<_>::default());
    /// ```
    fn build_surface_attributes(
        self,
        builder: SurfaceAttributesBuilder<WindowSurface<Self>>,
    ) -> SurfaceAttributes<WindowSurface<Self>>;

    /// Resize the surface to the window inner size.
    ///
    /// No-op if either window size is zero.
    ///
    /// # Example
    /// ```no_run
    /// use glutin_winit::GlWindow;
    /// # use glutin::surface::{Surface, WindowSurface};
    /// # let winit_window: winit::window::Window = unimplemented!();
    /// # let (gl_surface, gl_context): (Surface<glutin::NoDisplay, WindowSurface<glutin::NoWindow>>, _) = unimplemented!();
    ///
    /// winit_window.resize_surface(&gl_surface, &gl_context);
    /// ```
    fn resize_surface<D: HasDisplayHandle>(
        &self,
        surface: &Surface<D, impl SurfaceTypeTrait + ResizeableSurface>,
        context: &PossiblyCurrentContext<D>,
    );
}

macro_rules! implement_glwindow {
    (<$($lt:lifetime)?> $ty:ty) => {
        impl<$($lt)?> GlWindow for $ty {
            fn build_surface_attributes(
                self,
                builder: SurfaceAttributesBuilder<WindowSurface<Self>>,
            ) -> SurfaceAttributes<WindowSurface<Self>> {
                let (w, h) = self.inner_size().non_zero().expect("invalid zero inner size");
                builder.build(self, w, h)
            }

            fn resize_surface<D: HasDisplayHandle>(
                &self,
                surface: &Surface<D, impl SurfaceTypeTrait + ResizeableSurface>,
                context: &PossiblyCurrentContext<D>,
            ) {
                if let Some((w, h)) = self.inner_size().non_zero() {
                    surface.resize(context, w, h)
                }
            }
        }
    };
}

implement_glwindow!(<> Window);
implement_glwindow!(<'a> &'a Window);
implement_glwindow!(<> std::rc::Rc<Window>);
implement_glwindow!(<> std::sync::Arc<Window>);

/// [`winit::dpi::PhysicalSize<u32>`] non-zero extensions.
trait NonZeroU32PhysicalSize {
    /// Converts to non-zero `(width, height)`.
    fn non_zero(self) -> Option<(NonZeroU32, NonZeroU32)>;
}
impl NonZeroU32PhysicalSize for winit::dpi::PhysicalSize<u32> {
    fn non_zero(self) -> Option<(NonZeroU32, NonZeroU32)> {
        let w = NonZeroU32::new(self.width)?;
        let h = NonZeroU32::new(self.height)?;
        Some((w, h))
    }
}
