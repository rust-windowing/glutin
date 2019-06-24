use crate::api::osmesa;
use crate::{
    Api, Context, ContextBuilderWrapper, ContextCurrentState, ContextError,
    CreationError, NotCurrent, PixelFormat, PossiblyCurrent,
    PossiblyCurrentContextCurrentState, Surface,
};
use std::marker::PhantomData;
use winit::dpi;

/// A unix-specific extension to the [`ContextBuilder`] which allows building
/// unix-specific osmesa contexts.
///
/// [`ContextBuilder`]: ../../struct.ContextBuilder.html
pub trait OsMesaContextExt {
    /// Builds an OsMesa context.
    ///
    /// Errors can occur if the OpenGL [`Context`] could not be created. This
    /// generally happens because the underlying platform doesn't support a
    /// requested feature.
    ///
    /// [`Context`]: struct.Context.html
    fn build_osmesa(self) -> Result<OsMesaContext<NotCurrent>, CreationError>
    where
        Self: Sized;
}

impl<'a, T: ContextCurrentState> OsMesaContextExt
    for OsMesaContextBuilder<'a, T>
{
    #[inline]
    fn build_osmesa(self) -> Result<OsMesaContext<NotCurrent>, CreationError>
    where
        Self: Sized,
    {
        let cb = self.map_sharing(|ctx| &ctx.context);
        osmesa::OsMesaContext::new(cb).map(|context| OsMesaContext {
            context,
            phantom: PhantomData,
        })
    }
}

pub type OsMesaContextBuilder<'a, CS: ContextCurrentState> =
    ContextBuilderWrapper<&'a OsMesaContext<CS>>;

pub struct OsMesaContext<CS: ContextCurrentState> {
    pub(crate) context: osmesa::OsMesaContext,
    pub(crate) phantom: PhantomData<CS>,
}

impl<CS: PossiblyCurrentContextCurrentState> OsMesaContext<CS> {
    /// Returns the address of an OpenGL function.
    pub fn get_proc_address(&self, addr: &str) -> *const () {
        self.context.get_proc_address(addr)
    }
}

impl<CS: ContextCurrentState> OsMesaContext<CS> {
    pub fn is_current(&self) -> bool {
        self.context.is_current()
    }

    pub fn get_api(&self) -> Api {
        self.context.get_api()
    }

    pub unsafe fn make_current_osmesa_buffer(
        self,
        buffer: &mut OsMesaBuffer,
    ) -> Result<OsMesaContext<PossiblyCurrent>, (Self, ContextError)> {
        match self.context.make_current_osmesa_buffer(buffer.inner_mut()) {
            Ok(()) => Ok(OsMesaContext {
                context: self.context,
                phantom: PhantomData,
            }),
            Err(err) => Err((self, err)),
        }
    }

    pub unsafe fn make_not_current(
        self,
    ) -> Result<OsMesaContext<NotCurrent>, (Self, ContextError)> {
        match self.context.make_not_current() {
            Ok(()) => Ok(OsMesaContext {
                context: self.context,
                phantom: PhantomData,
            }),
            Err(err) => Err((self, err)),
        }
    }

    pub unsafe fn treat_as_current<CS2: PossiblyCurrentContextCurrentState>(
        self,
    ) -> OsMesaContext<CS2> {
        OsMesaContext {
            context: self.context,
            phantom: PhantomData,
        }
    }

    pub unsafe fn treat_as_not_current(self) -> OsMesaContext<NotCurrent> {
        OsMesaContext {
            context: self.context,
            phantom: PhantomData,
        }
    }

    pub(crate) fn inner(&self) -> &osmesa::OsMesaContext {
        &self.context
    }
    pub(crate) fn inner_mut(&mut self) -> &mut osmesa::OsMesaContext {
        &mut self.context
    }
}

pub struct OsMesaBuffer {
    pub(crate) surface: osmesa::OsMesaBuffer,
}

impl Surface for OsMesaBuffer {
    type Inner = osmesa::OsMesaBuffer;

    fn inner(&self) -> &Self::Inner {
        &self.surface
    }
    fn inner_mut(&mut self) -> &mut Self::Inner {
        &mut self.surface
    }

    fn get_pixel_format(&self) -> PixelFormat {
        self.surface.get_pixel_format()
    }

    fn is_current(&self) -> bool {
        self.surface.is_current()
    }
}

impl OsMesaBuffer {
    pub fn new<CS: ContextCurrentState>(
        ctx: &OsMesaContext<CS>,
        size: dpi::PhysicalSize,
    ) -> Result<Self, CreationError> {
        let ctx = ctx.inner();
        osmesa::OsMesaBuffer::new(ctx, size)
            .map(|surface| OsMesaBuffer { surface })
    }
}
