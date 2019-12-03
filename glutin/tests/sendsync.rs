use glutin::*;

trait FailToCompileIfNotSendSync
where
    Self: Send + Sync,
{
}
impl<
        PBT: SupportsPBuffersTrait,
        WST: SupportsWindowSurfacesTrait,
        ST: SupportsSurfacelessTrait,
    > FailToCompileIfNotSendSync for SplitContext<ContextIsCurrent::No, PBT, WST, ST>
{
}
impl FailToCompileIfNotSendSync for WindowSurface<SurfaceInUse::No> {}
impl FailToCompileIfNotSendSync for RawWindowSurface<SurfaceInUse::No> {}
impl FailToCompileIfNotSendSync for PBuffer<SurfaceInUse::No> {}
impl<
        PBT: SupportsPBuffersTrait,
        WST: SupportsWindowSurfacesTrait,
        ST: SupportsSurfacelessTrait,
        SURFACE: Surface + Send + Sync,
    > FailToCompileIfNotSendSync for Context<ContextIsCurrent::No, PBT, WST, ST, SURFACE>
{
}

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]
impl FailToCompileIfNotSendSync for glutin::platform::unix::osmesa::OsMesaBuffer<SurfaceInUse::No> {}

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]
impl FailToCompileIfNotSendSync
    for glutin::platform::unix::osmesa::OsMesaContext<ContextIsCurrent::No, SurfaceInUse::No>
{
}

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]
impl FailToCompileIfNotSendSync
    for glutin::platform::unix::osmesa::SplitOsMesaContext<ContextIsCurrent::No>
{
}
