use super::*;

pub trait SurfaceOrNothing {
    type NotInUseType: SurfaceOrNothing;
    type PossiblyInUseType: SurfaceOrNothing;

    unsafe fn treat_as_not_current(self) -> Self::NotInUseType;

    unsafe fn treat_as_current(self) -> Self::PossiblyInUseType;
    unsafe fn make_not_current(self) -> Result<Self::NotInUseType, (Self::PossiblyInUseType, ContextError)>;
}

impl SurfaceOrNothing for () {
    type NotInUseType = ();
    type PossiblyInUseType = ();

    unsafe fn treat_as_not_current(self) -> Self::NotInUseType {
        ()
    }

    unsafe fn treat_as_current(self) -> Self::PossiblyInUseType {
        ()
    }

    unsafe fn make_not_current(self) -> Result<Self::NotInUseType, (Self::PossiblyInUseType, ContextError)> {Ok(()) }
}
impl<T: Surface> SurfaceOrNothing for T {
    type NotInUseType = <T as Surface>::NotInUseType;
    type PossiblyInUseType = <T as Surface>::PossiblyInUseType;

    unsafe fn treat_as_not_current(self) -> Self::NotInUseType {
        self.treat_as_not_current()
    }

    unsafe fn treat_as_current(self) -> Self::PossiblyInUseType {
        self.treat_as_current()
    }

    unsafe fn make_not_current(self) -> Result<Self::NotInUseType, (Self::PossiblyInUseType, ContextError)> { self.make_not_current() }
}

#[derive(Debug)]
pub struct Context<
    IC: ContextIsCurrentTrait,
    PBT: SupportsPBuffersTrait,
    WST: SupportsWindowSurfacesTrait,
    ST: SupportsSurfacelessTrait,
    SURFACE: SurfaceOrNothing,
> {
    pub(crate) context: SplitContext<IC, PBT, WST, ST>,
    pub(crate) surface: SURFACE,
}

impl<
        IC: ContextIsCurrentTrait,
        PBT: SupportsPBuffersTrait,
        WST: SupportsWindowSurfacesTrait,
        ST: SupportsSurfacelessTrait,
        SURFACE: SurfaceOrNothing,
    > Context<IC, PBT, WST, ST, SURFACE>
{
    pub unsafe fn make_not_current(
        self,
    ) -> Result<
        Context<
            ContextIsCurrent::No,
            PBT,
            WST,
            ST,
            SURFACE::NotInUseType,
        >,
        (
            Context<
                ContextIsCurrent::PossiblyAndSurfaceBound,
                PBT,
                WST,
                ST,
                SURFACE::PossiblyInUseType,
            >,
            ContextError,
        ),
    > {
        match self.context.make_not_current() {
            Ok(context) => match self.surface.make_not_current() {
                Err((surface, err)) => Err((Context { context: context.treat_as_current(), surface }, err)),
                Ok(surface) => Ok(Context { context, surface })
            }
            Err((context, err)) => Err((Context { context, surface: SurfaceOrNothing::treat_as_current(self.surface) }, err)),
        }
    }

    pub unsafe fn split(self) -> (SplitContext<IC, PBT, WST, ST>, SURFACE) {
        (self.context, self.surface)
    }
}

impl<
        IC: ContextIsCurrentTrait,
        WST: SupportsWindowSurfacesTrait,
        ST: SupportsSurfacelessTrait,
        IU: SurfaceInUseTrait,
    > Context<IC, SupportsPBuffers::Yes, WST, ST, PBuffer<IU>>
{
    pub unsafe fn make_current(
        self,
    ) -> Result<
        Context<
            ContextIsCurrent::Possibly,
            SupportsPBuffers::Yes,
            WST,
            ST,
            PBuffer<SurfaceInUse::Possibly>,
        >,
        (
            Context<
                ContextIsCurrent::PossiblyAndSurfaceBound,
                SupportsPBuffers::Yes,
                WST,
                ST,
                PBuffer<SurfaceInUse::Possibly>,
            >,
            ContextError,
        ),
    > {
        match self.context.make_current_pbuffer(self.surface) {
            Ok((context, surface)) => Ok(Context { context, surface }),
            Err((context, surface, err)) => {
                Err((Context { context, surface }, err))
            }
        }
    }
}

impl<
        IC: ContextIsCurrentTrait,
        PBT: SupportsPBuffersTrait,
        ST: SupportsSurfacelessTrait,
        IU: SurfaceInUseTrait,
        W,
    >
    Context<
        IC,
        PBT,
        SupportsWindowSurfaces::Yes,
        ST,
        WindowSurfaceWrapper<W, IU>,
    >
{
    pub unsafe fn make_current(
        self,
    ) -> Result<
        Context<
            ContextIsCurrent::PossiblyAndSurfaceBound,
            PBT,
            SupportsWindowSurfaces::Yes,
            ST,
            WindowSurfaceWrapper<W, SurfaceInUse::Possibly>,
        >,
        (
            Context<
                ContextIsCurrent::PossiblyAndSurfaceBound,
                PBT,
                SupportsWindowSurfaces::Yes,
                ST,
                WindowSurfaceWrapper<W, SurfaceInUse::Possibly>,
            >,
            ContextError,
        ),
    > {
        match self.context.make_current_window(self.surface) {
            Ok((context, surface)) => Ok(Context { context, surface }),
            Err((context, surface, err)) => {
                Err((Context { context, surface }, err))
            }
        }
    }
}

impl<
        IC: ContextIsCurrentTrait,
        PBT: SupportsPBuffersTrait,
        WST: SupportsWindowSurfacesTrait,
    >
    Context<
        IC,
        PBT,
        WST,
        SupportsSurfaceless::Yes,
        (),
    >
{
    pub unsafe fn make_current(
        self,
    ) -> Result<
        Context<
            ContextIsCurrent::Possibly,
            PBT,
            WST,
            SupportsSurfaceless::Yes,
            (),
        >,
        (
            Context<
                ContextIsCurrent::PossiblyAndSurfaceBound,
                PBT,
                WST,
                SupportsSurfaceless::Yes,
                (),
            >,
            ContextError,
        ),
    > {
        match self.context.make_current_surfaceless() {
            Ok(context) => Ok(Context { context, surface: () }),
            Err((context, err)) => {
                Err((Context { context, surface: () }, err))
            }
        }
    }
}

impl<
        IC: ContextIsCurrentTrait,
        PBT: SupportsPBuffersTrait,
        WST: SupportsWindowSurfacesTrait,
        SURFACE: SurfaceOrNothing,
    > Context<IC, PBT, WST, SupportsSurfaceless::Yes, SURFACE>
{
    pub unsafe fn make_current_surfaceless(
        self,
    ) -> Result<
        Context<
            ContextIsCurrent::Possibly,
            PBT,
            WST,
            SupportsSurfaceless::Yes,
            SURFACE::NotInUseType,
        >,
        (
            Context<
                ContextIsCurrent::PossiblyAndSurfaceBound,
                PBT,
                WST,
                SupportsSurfaceless::Yes,
                SURFACE,
            >,
            ContextError,
        ),
    > {
        match self.context.make_current_surfaceless() {
            Ok(context) => Ok(Context {
                context,
                surface: self.surface.treat_as_not_current(),
            }),
            Err((context, err)) => Err((
                Context {
                    context,
                    surface: self.surface,
                },
                err,
            )),
        }
    }
}

impl<
        IC: ContextIsCurrentTrait,
        PBT: SupportsPBuffersTrait,
        ST: SupportsSurfacelessTrait,
        SURFACE: SurfaceOrNothing,
    > Context<IC, PBT, SupportsWindowSurfaces::Yes, ST, SURFACE>
{
    pub unsafe fn make_current_window<W, IU: SurfaceInUseTrait>(
        self,
        mut surface: WindowSurfaceWrapper<W, IU>,
    ) -> Result<
        (
            Context<
                ContextIsCurrent::PossiblyAndSurfaceBound,
                PBT,
                SupportsWindowSurfaces::Yes,
                ST,
                WindowSurfaceWrapper<W, SurfaceInUse::Possibly>,
            >,
            SURFACE::NotInUseType,
        ),
        (
            Context<
                ContextIsCurrent::PossiblyAndSurfaceBound,
                PBT,
                SupportsWindowSurfaces::Yes,
                ST,
                SURFACE,
            >,
            WindowSurfaceWrapper<W, SurfaceInUse::Possibly>,
            ContextError,
        ),
    > {
        match self.context.make_current_window(surface) {
            Ok((context, nsurface)) => Ok((
                Context {
                    context,
                    surface: Surface::treat_as_current(nsurface),
                },
                SurfaceOrNothing::treat_as_not_current(self.surface),
            )),
            Err((context, nsurface, err)) => Err((
                Context {
                    context,
                    surface: self.surface,
                },
                Surface::treat_as_current(nsurface),
                err,
            )),
        }
    }
}

impl<
        IC: ContextIsCurrentTrait,
        WST: SupportsWindowSurfacesTrait,
        ST: SupportsSurfacelessTrait,
        SURFACE: SurfaceOrNothing,
    > Context<IC, SupportsPBuffers::Yes, WST, ST, SURFACE>
{
    pub unsafe fn make_current_pbuffer<IU: SurfaceInUseTrait>(
        self,
        mut pbuffer: PBuffer<IU>,
    ) -> Result<
        (
            Context<
                ContextIsCurrent::Possibly,
                SupportsPBuffers::Yes,
                WST,
                ST,
                PBuffer<SurfaceInUse::Possibly>,
            >,
            SURFACE::NotInUseType,
        ),
        (
            Context<
                ContextIsCurrent::PossiblyAndSurfaceBound,
                SupportsPBuffers::Yes,
                WST,
                ST,
                SURFACE,
            >,
            PBuffer<SurfaceInUse::Possibly>,
            ContextError,
        ),
    > {
        match self.context.make_current_pbuffer(pbuffer) {
            Ok((context, pbuffer)) => Ok((
                Context {
                    context,
                    surface: Surface::treat_as_current(pbuffer),
                },
                SurfaceOrNothing::treat_as_not_current(self.surface),
            )),
            Err((context, pbuffer, err)) => Err((
                Context {
                    context,
                    surface: self.surface,
                },
                Surface::treat_as_current(pbuffer),
                err,
            )),
        }
    }
}
