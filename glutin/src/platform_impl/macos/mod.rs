#![cfg(target_os = "macos")]
#![allow(clippy::let_unit_value)]
use crate::{
    ContextError, CreationError, GlAttributes, PixelFormat, PixelFormatRequirements, Rect,
    Robustness,
};

use cgl::{kCGLCECrashOnRemovedFunctions, kCGLCPSurfaceOpacity, CGLEnable, CGLSetParameter};
use cocoa::appkit::{self, NSOpenGLContext, NSOpenGLPixelFormat};
use cocoa::base::{id, nil};
use cocoa::foundation::NSAutoreleasePool;
use core_foundation::base::TCFType;
use core_foundation::bundle::{CFBundleGetBundleWithIdentifier, CFBundleGetFunctionPointerForName};
use core_foundation::string::CFString;
use objc::runtime::{BOOL, NO};

use crate::platform::macos::WindowExtMacOS;

use winit::dpi;
use winit::event_loop::EventLoopWindowTarget;
use winit::window::{Window, WindowBuilder};

use std::ops::Deref;
use std::os::raw;
use std::str::FromStr;

mod helpers;

#[derive(Debug)]
pub enum Context {
    WindowedContext(WindowedContext),
    HeadlessContext(HeadlessContext),
}

#[derive(Debug)]
pub struct WindowedContext {
    // NSOpenGLContext
    context: IdRef,
    pixel_format: PixelFormat,
}

#[derive(Debug)]
pub struct HeadlessContext {
    context: IdRef,
}

impl Context {
    #[inline]
    pub fn new_windowed<T>(
        wb: WindowBuilder,
        el: &EventLoopWindowTarget<T>,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
    ) -> Result<(Window, Self), CreationError> {
        let transparent = wb.window.transparent;
        let win = wb.build(el)?;

        let share_ctx = gl_attr.sharing.map_or(nil, |c| *c.get_id());

        match gl_attr.robustness {
            Robustness::RobustNoResetNotification | Robustness::RobustLoseContextOnReset => {
                return Err(CreationError::RobustnessNotSupported);
            }
            _ => (),
        }

        let view = win.ns_view() as id;

        let gl_profile = helpers::get_gl_profile(gl_attr, pf_reqs)?;
        let attributes = helpers::build_nsattributes(pf_reqs, gl_profile)?;
        unsafe {
            let pixel_format =
                IdRef::new(NSOpenGLPixelFormat::alloc(nil).initWithAttributes_(&attributes));
            let pixel_format = match pixel_format.non_nil() {
                None => return Err(CreationError::NoAvailablePixelFormat),
                Some(pf) => pf,
            };

            let gl_context = IdRef::new(
                NSOpenGLContext::alloc(nil).initWithFormat_shareContext_(*pixel_format, share_ctx),
            );
            let gl_context = match gl_context.non_nil() {
                Some(gl_context) => gl_context,
                None => {
                    return Err(CreationError::NotSupported(
                        "could not open gl context".to_string(),
                    ));
                }
            };

            let pixel_format = {
                let get_attr = |attrib: appkit::NSOpenGLPixelFormatAttribute| -> i32 {
                    let mut value = 0;
                    NSOpenGLPixelFormat::getValues_forAttribute_forVirtualScreen_(
                        *pixel_format,
                        &mut value,
                        attrib,
                        NSOpenGLContext::currentVirtualScreen(*gl_context),
                    );
                    value
                };

                PixelFormat {
                    hardware_accelerated: get_attr(appkit::NSOpenGLPFAAccelerated) != 0,
                    color_bits: (get_attr(appkit::NSOpenGLPFAColorSize)
                        - get_attr(appkit::NSOpenGLPFAAlphaSize))
                        as u8,
                    alpha_bits: get_attr(appkit::NSOpenGLPFAAlphaSize) as u8,
                    depth_bits: get_attr(appkit::NSOpenGLPFADepthSize) as u8,
                    stencil_bits: get_attr(appkit::NSOpenGLPFAStencilSize) as u8,
                    stereoscopy: get_attr(appkit::NSOpenGLPFAStereo) != 0,
                    double_buffer: get_attr(appkit::NSOpenGLPFADoubleBuffer) != 0,
                    multisampling: if get_attr(appkit::NSOpenGLPFAMultisample) > 0 {
                        Some(get_attr(appkit::NSOpenGLPFASamples) as u16)
                    } else {
                        None
                    },
                    srgb: true,
                }
            };

            gl_context.setView_(view);
            let value = if gl_attr.vsync { 1 } else { 0 };
            gl_context.setValues_forParameter_(
                &value,
                appkit::NSOpenGLContextParameter::NSOpenGLCPSwapInterval,
            );

            if transparent {
                let opacity = 0;
                CGLSetParameter(
                    gl_context.CGLContextObj() as *mut _,
                    kCGLCPSurfaceOpacity,
                    &opacity,
                );
            }

            CGLEnable(gl_context.CGLContextObj() as *mut _, kCGLCECrashOnRemovedFunctions);

            let context = WindowedContext { context: gl_context, pixel_format };
            Ok((win, Context::WindowedContext(context)))
        }
    }

    #[inline]
    pub fn new_headless<T>(
        _el: &EventLoopWindowTarget<T>,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
        _size: dpi::PhysicalSize<u32>,
    ) -> Result<Self, CreationError> {
        let gl_profile = helpers::get_gl_profile(gl_attr, pf_reqs)?;
        let attributes = helpers::build_nsattributes(pf_reqs, gl_profile)?;
        let context = unsafe {
            let pixelformat = NSOpenGLPixelFormat::alloc(nil).initWithAttributes_(&attributes);
            if pixelformat == nil {
                return Err(CreationError::OsError(
                    "Could not create the pixel format".to_string(),
                ));
            }
            let context =
                NSOpenGLContext::alloc(nil).initWithFormat_shareContext_(pixelformat, nil);
            if context == nil {
                return Err(CreationError::OsError(
                    "Could not create the rendering context".to_string(),
                ));
            }

            IdRef::new(context)
        };

        let headless = HeadlessContext { context };

        Ok(Context::HeadlessContext(headless))
    }

    pub fn resize(&self, _width: u32, _height: u32) {
        match *self {
            Context::WindowedContext(ref c) => unsafe { c.context.update() },
            _ => unreachable!(),
        }
    }

    #[inline]
    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        match *self {
            Context::WindowedContext(ref c) => {
                let _: () = msg_send![*c.context, update];
                c.context.makeCurrentContext();
            }
            Context::HeadlessContext(ref c) => {
                let _: () = msg_send![*c.context, update];
                c.context.makeCurrentContext();
            }
        }
        Ok(())
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), ContextError> {
        if self.is_current() {
            match *self {
                Context::WindowedContext(ref c) => {
                    let _: () = msg_send![*c.context, update];
                    NSOpenGLContext::clearCurrentContext(nil);
                }
                Context::HeadlessContext(ref c) => {
                    let _: () = msg_send![*c.context, update];
                    NSOpenGLContext::clearCurrentContext(nil);
                }
            }
        }
        Ok(())
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        unsafe {
            let context = match *self {
                Context::WindowedContext(ref c) => *c.context,
                Context::HeadlessContext(ref c) => *c.context,
            };

            let pool = NSAutoreleasePool::new(nil);
            let current = NSOpenGLContext::currentContext(nil);
            let res = if current != nil {
                let is_equal: BOOL = msg_send![current, isEqual: context];
                is_equal != NO
            } else {
                false
            };
            let _: () = msg_send![pool, release];
            res
        }
    }

    pub fn get_proc_address(&self, addr: &str) -> *const core::ffi::c_void {
        let symbol_name: CFString = FromStr::from_str(addr).unwrap();
        let framework_name: CFString = FromStr::from_str("com.apple.opengl").unwrap();
        let framework =
            unsafe { CFBundleGetBundleWithIdentifier(framework_name.as_concrete_TypeRef()) };
        let symbol = unsafe {
            CFBundleGetFunctionPointerForName(framework, symbol_name.as_concrete_TypeRef())
        };
        symbol as *const _
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        unsafe {
            match *self {
                Context::WindowedContext(ref c) => {
                    let pool = NSAutoreleasePool::new(nil);
                    c.context.flushBuffer();
                    let _: () = msg_send![pool, release];
                }
                Context::HeadlessContext(_) => unreachable!(),
            }
        }
        Ok(())
    }

    #[inline]
    pub fn buffer_age(&self) -> u32 {
        0
    }

    #[inline]
    pub fn swap_buffers_with_damage(&self, _rects: &[Rect]) -> Result<(), ContextError> {
        Err(ContextError::OsError("buffer damage not suported".to_string()))
    }

    #[inline]
    pub fn swap_buffers_with_damage_supported(&self) -> bool {
        false
    }

    #[inline]
    pub fn get_api(&self) -> crate::Api {
        crate::Api::OpenGl
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        match *self {
            Context::WindowedContext(ref c) => c.pixel_format.clone(),
            Context::HeadlessContext(_) => unreachable!(),
        }
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> *mut raw::c_void {
        match self {
            Context::WindowedContext(c) => c.context.deref().CGLContextObj() as *mut _,
            Context::HeadlessContext(c) => c.context.deref().CGLContextObj() as *mut _,
        }
    }

    #[inline]
    fn get_id(&self) -> IdRef {
        match self {
            Context::WindowedContext(w) => w.context.clone(),
            Context::HeadlessContext(h) => h.context.clone(),
        }
    }
}

#[derive(Debug)]
struct IdRef(id);

impl IdRef {
    fn new(i: id) -> IdRef {
        IdRef(i)
    }

    #[allow(dead_code)]
    fn retain(i: id) -> IdRef {
        if i != nil {
            let _: id = unsafe { msg_send![i, retain] };
        }
        IdRef(i)
    }

    fn non_nil(self) -> Option<IdRef> {
        if self.0 == nil {
            None
        } else {
            Some(self)
        }
    }
}

impl Drop for IdRef {
    fn drop(&mut self) {
        if self.0 != nil {
            let _: () = unsafe { msg_send![self.0, release] };
        }
    }
}

impl Deref for IdRef {
    type Target = id;
    fn deref(&self) -> &id {
        &self.0
    }
}

impl Clone for IdRef {
    fn clone(&self) -> IdRef {
        if self.0 != nil {
            let _: id = unsafe { msg_send![self.0, retain] };
        }
        IdRef(self.0)
    }
}

unsafe impl Send for Context {}
unsafe impl Sync for Context {}
