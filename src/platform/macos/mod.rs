#![cfg(target_os = "macos")]

use CreationError;
use ContextError;
use GlAttributes;
use PixelFormat;
use PixelFormatRequirements;
use Robustness;

use objc::runtime::{BOOL, NO};

use cgl::{CGLEnable, kCGLCECrashOnRemovedFunctions, CGLSetParameter, kCGLCPSurfaceOpacity};

use cocoa::base::{id, nil};
use cocoa::foundation::NSAutoreleasePool;
use cocoa::appkit::{self, NSOpenGLContext, NSOpenGLPixelFormat};

use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use core_foundation::bundle::{CFBundleGetBundleWithIdentifier, CFBundleGetFunctionPointerForName};

use std::str::FromStr;
use std::ops::Deref;

use winit;
use winit::os::macos::WindowExt;
pub use winit::{MonitorId, NativeMonitorId, get_available_monitors, get_primary_monitor};
pub use self::headless::HeadlessContext;
pub use self::headless::PlatformSpecificHeadlessBuilderAttributes;

mod headless;
mod helpers;

pub struct Context {
    // NSOpenGLContext
    gl: IdRef,
    pixel_format: PixelFormat,
}

impl Context {

    pub fn new(
        window_builder: winit::WindowBuilder,
        events_loop: &winit::EventsLoop,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
    ) -> Result<(winit::Window, Self), CreationError>
    {
        let transparent = window_builder.window.transparent;
        let window = try!(window_builder.build(events_loop));

        if gl_attr.sharing.is_some() {
            unimplemented!()
        }

        match gl_attr.robustness {
            Robustness::RobustNoResetNotification |
            Robustness::RobustLoseContextOnReset => {
                return Err(CreationError::RobustnessNotSupported);
            }
            _ => (),
        }

        let view = window.get_nsview() as id;

        let attributes = try!(helpers::build_nsattributes(pf_reqs, gl_attr));
        unsafe {
            let pixel_format = IdRef::new(NSOpenGLPixelFormat::alloc(nil)
                .initWithAttributes_(&attributes));
            let pixel_format = match pixel_format.non_nil() {
                None => return Err(CreationError::NoAvailablePixelFormat),
                Some(pf) => pf,
            };

            // TODO: Add context sharing
            let gl_context = IdRef::new(NSOpenGLContext::alloc(nil)
                .initWithFormat_shareContext_(*pixel_format, nil));
            let gl_context = match gl_context.non_nil() {
                Some(gl_context) => gl_context,
                None => return Err(CreationError::NotSupported),
            };

            let pixel_format = {
                let get_attr = |attrib: appkit::NSOpenGLPixelFormatAttribute| -> i32 {
                    let mut value = 0;
                    NSOpenGLPixelFormat::getValues_forAttribute_forVirtualScreen_(
                        *pixel_format,
                        &mut value,
                        attrib,
                        NSOpenGLContext::currentVirtualScreen(*gl_context));
                    value
                };

                PixelFormat {
                    hardware_accelerated: get_attr(appkit::NSOpenGLPFAAccelerated) != 0,
                    color_bits: (get_attr(appkit::NSOpenGLPFAColorSize) - get_attr(appkit::NSOpenGLPFAAlphaSize)) as u8,
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
            gl_context.setValues_forParameter_(&value, appkit::NSOpenGLContextParameter::NSOpenGLCPSwapInterval);

            if transparent {
                let mut opacity = 0;
                CGLSetParameter(gl_context.CGLContextObj() as *mut _, kCGLCPSurfaceOpacity, &mut opacity);
            }

            CGLEnable(gl_context.CGLContextObj() as *mut _, kCGLCECrashOnRemovedFunctions);

            let context = Context { gl: gl_context, pixel_format: pixel_format };
            Ok((window, context))
        }
    }

    pub fn resize(&self, _width: u32, _height: u32) {
        unsafe { self.gl.update(); }
    }

    #[inline]
    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        let _: () = msg_send![*self.gl, update];
        self.gl.makeCurrentContext();
        Ok(())
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        unsafe {
            let current = NSOpenGLContext::currentContext(nil);
            if current != nil {
                let is_equal: BOOL = msg_send![current, isEqual:*self.gl];
                is_equal != NO
            } else {
                false
            }
        }
    }

    pub fn get_proc_address(&self, addr: &str) -> *const () {
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
            let pool = NSAutoreleasePool::new(nil);
            self.gl.flushBuffer();
            let _: () = msg_send![pool, release];
        }
        Ok(())
    }

    #[inline]
    pub fn get_api(&self) -> ::Api {
        ::Api::OpenGl
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        self.pixel_format.clone()
    }
}

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
        if self.0 == nil { None } else { Some(self) }
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
    fn deref<'a>(&'a self) -> &'a id {
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
