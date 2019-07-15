pub use glutin::platform::unix::osmesa::*;

use glutin::api::osmesa;
use glutin::{
    Api, ContextBuilderWrapper, ContextError, ContextIsCurrent,
    ContextIsCurrentTrait, ContextIsCurrentYesTrait, CreationError,
    PixelFormat, Surface, SurfaceInUse, SurfaceInUseTrait,
};
use std::marker::PhantomData;
use glutin::dpi;

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
    fn build_osmesa(
        self,
    ) -> Result<SplitOsMesaContext<ContextIsCurrent::No>, CreationError>
    where
        Self: Sized;
}

impl<'a, T: ContextIsCurrentTrait> OsMesaContextExt
    for OsMesaContextBuilder<'a, T>
{
    #[inline]
    fn build_osmesa(
        self,
    ) -> Result<SplitOsMesaContext<ContextIsCurrent::No>, CreationError>
    where
        Self: Sized,
    {
        let cb = self.map_sharing(|ctx| &ctx.context);
        osmesa::OsMesaContext::new(cb).map(|context| SplitOsMesaContext {
            context,
            phantom: PhantomData,
        })
    }
}

pub type OsMesaContextBuilder<'a, IC> =
    ContextBuilderWrapper<&'a SplitOsMesaContext<IC>>;

#[derive(Debug)]
pub struct SplitOsMesaContext<IC: ContextIsCurrentTrait> {
    pub(crate) context: osmesa::OsMesaContext,
    pub(crate) phantom: PhantomData<IC>,
}

impl<IC: ContextIsCurrentYesTrait> SplitOsMesaContext<IC> {
    /// Returns the address of an OpenGL function.
    pub fn get_proc_address(&self, addr: &str) -> *const () {
        self.context.get_proc_address(addr)
    }
}

impl<IC: ContextIsCurrentTrait> SplitOsMesaContext<IC> {
    pub fn is_current(&self) -> bool {
        self.context.is_current()
    }

    pub fn get_api(&self) -> Api {
        self.context.get_api()
    }

    pub unsafe fn make_current_osmesa_buffer<IU: SurfaceInUseTrait>(
        self,
        mut buffer: OsMesaBuffer<IU>,
    ) -> Result<
        (
            SplitOsMesaContext<ContextIsCurrent::Possibly>,
            OsMesaBuffer<SurfaceInUse::Possibly>,
        ),
        (
            SplitOsMesaContext<ContextIsCurrent::Possibly>,
            OsMesaBuffer<SurfaceInUse::Possibly>,
            ContextError,
        ),
    > {
        match self.context.make_current_osmesa_buffer(buffer.inner()) {
            Ok(()) => Ok((self.treat_as_current(), buffer.treat_as_current())),
            Err(err) => {
                Err((self.treat_as_current(), buffer.treat_as_current(), err))
            }
        }
    }

    pub unsafe fn make_not_current(
        self,
    ) -> Result<SplitOsMesaContext<ContextIsCurrent::No>, (Self, ContextError)>
    {
        match self.context.make_not_current() {
            Ok(()) => Ok(SplitOsMesaContext {
                context: self.context,
                phantom: PhantomData,
            }),
            Err(err) => Err((self, err)),
        }
    }

    pub unsafe fn treat_as_current<IC2: ContextIsCurrentYesTrait>(
        self,
    ) -> SplitOsMesaContext<IC2> {
        SplitOsMesaContext {
            context: self.context,
            phantom: PhantomData,
        }
    }

    pub unsafe fn treat_as_not_current(
        self,
    ) -> SplitOsMesaContext<ContextIsCurrent::No> {
        SplitOsMesaContext {
            context: self.context,
            phantom: PhantomData,
        }
    }

    pub(crate) fn inner(&self) -> &osmesa::OsMesaContext {
        &self.context
    }

    pub fn unify_with<IU: SurfaceInUseTrait>(
        self,
        buffer: OsMesaBuffer<IU>,
    ) -> OsMesaContext<IC, IU> {
        OsMesaContext {
            context: self,
            buffer,
        }
    }
}

#[derive(Debug)]
pub struct OsMesaBuffer<IU: SurfaceInUseTrait> {
    pub(crate) buffer: osmesa::OsMesaBuffer,
    pub(crate) phantom: PhantomData<IU>,
}

impl<IU: SurfaceInUseTrait> Surface for OsMesaBuffer<IU> {
    type Inner = osmesa::OsMesaBuffer;
    type NotInUseType = OsMesaBuffer<SurfaceInUse::No>;
    type PossiblyInUseType = OsMesaBuffer<SurfaceInUse::Possibly>;

    fn inner(&self) -> &Self::Inner {
        &self.buffer
    }

    fn get_pixel_format(&self) -> PixelFormat {
        self.buffer.get_pixel_format()
    }

    fn is_current(&self) -> bool {
        panic!("This cannot be implemented with OsMesa.")
    }

    unsafe fn treat_as_not_current(self) -> Self::NotInUseType {
        OsMesaBuffer {
            buffer: self.buffer,
            phantom: PhantomData,
        }
    }

    unsafe fn treat_as_current(self) -> Self::PossiblyInUseType {
        OsMesaBuffer {
            buffer: self.buffer,
            phantom: PhantomData,
        }
    }

    unsafe fn make_not_current(self) -> Result<Self::NotInUseType, (Self::PossiblyInUseType, ContextError)> {
        panic!("This cannot be implemented with OsMesa.")
    }
}

impl<IU: SurfaceInUseTrait> OsMesaBuffer<IU> {
    pub fn new<IC: ContextIsCurrentTrait>(
        ctx: &SplitOsMesaContext<IC>,
        size: dpi::PhysicalSize,
    ) -> Result<OsMesaBuffer<SurfaceInUse::No>, CreationError> {
        let ctx = ctx.inner();
        osmesa::OsMesaBuffer::new(ctx, size).map(|buffer| OsMesaBuffer {
            buffer,
            phantom: PhantomData,
        })
    }
}

#[derive(Debug)]
pub struct OsMesaContext<
    IC: ContextIsCurrentTrait,
    IU: SurfaceInUseTrait,
> {
    pub(crate) context: SplitOsMesaContext<IC>,
    pub(crate) buffer: OsMesaBuffer<IU>,
}

impl<IC: ContextIsCurrentTrait, IU: SurfaceInUseTrait> OsMesaContext<IC, IU> {
    pub unsafe fn make_current(
        self,
    ) -> Result<
        OsMesaContext<
            ContextIsCurrent::Possibly,
            SurfaceInUse::Possibly,
        >,
        (
            OsMesaContext<
                ContextIsCurrent::Possibly,
                SurfaceInUse::Possibly,
            >,
            ContextError,
        ),
    > {
        match self.context.make_current_osmesa_buffer(self.buffer) {
            Ok((context, buffer)) => Ok(OsMesaContext { context, buffer }),
            Err((context, buffer, err)) => {
                Err((OsMesaContext { context, buffer }, err))
            }
        }
    }
}

impl<IC: ContextIsCurrentTrait, IU: SurfaceInUseTrait> OsMesaContext<IC, IU> {
    pub unsafe fn make_current_osmesa_buffer<IU2: SurfaceInUseTrait>(
        self,
        mut buffer: OsMesaBuffer<IU2>,
    ) -> Result<
        (
        OsMesaContext<
            ContextIsCurrent::Possibly,
            SurfaceInUse::Possibly,
        >,
        OsMesaBuffer<SurfaceInUse::No>,
        ),
        (
            OsMesaContext<
                ContextIsCurrent::Possibly,
                SurfaceInUse::Possibly,
            >,
        OsMesaBuffer<SurfaceInUse::Possibly>,
            ContextError,
        ),
    > {
        match self.context.make_current_osmesa_buffer(buffer) {
            Ok((context, nbuffer)) => Ok((OsMesaContext { context, buffer: nbuffer }, Surface::treat_as_not_current(self.buffer))),
            Err((context, nbuffer, err)) => {
                Err((OsMesaContext { context, buffer: Surface::treat_as_current(self.buffer) }, nbuffer, err))
            }
        }
    }
}
