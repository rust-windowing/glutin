#![cfg(target_os = "macos")]

use CreationError;
use CreationError::OsError;
use ContextError;
use GlAttributes;
use GlContext;
use PixelFormat;
use PixelFormatRequirements;
use Robustness;
use WindowAttributes;
use os::macos::ActivationPolicy;

use objc::runtime::{BOOL, NO};

use cgl::{CGLEnable, kCGLCECrashOnRemovedFunctions};

use cocoa::base::{id, nil};
use cocoa::foundation::NSAutoreleasePool;
use cocoa::appkit::{self, NSOpenGLContext, NSOpenGLPixelFormat};

use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use core_foundation::bundle::{CFBundleGetBundleWithIdentifier, CFBundleGetFunctionPointerForName};

use std::str::FromStr;
use std::ops::Deref;

use libc;

use winit;
use winit::os::macos::WindowExt;
pub use winit::{MonitorId, NativeMonitorId, get_available_monitors, get_primary_monitor};
pub use self::headless::HeadlessContext;
pub use self::headless::PlatformSpecificHeadlessBuilderAttributes;

mod headless;
mod helpers;

#[derive(Clone, Default)]
pub struct PlatformSpecificWindowBuilderAttributes {
    pub activation_policy: ActivationPolicy,
}

pub struct Window {
    context: IdRef,
    pixel_format: PixelFormat,
    winit_window: winit::Window,
}

unsafe impl Send for Window {}
unsafe impl Sync for Window {}

impl Window {
    pub fn new(win_attribs: &WindowAttributes,
               pf_reqs: &PixelFormatRequirements,
               opengl: &GlAttributes<&Window>,
               _pl_attribs: &PlatformSpecificWindowBuilderAttributes,
               winit_builder: winit::WindowBuilder)
               -> Result<Window, CreationError>
    {
        if opengl.sharing.is_some() {
            unimplemented!()
        }

        // not implemented
        assert!(win_attribs.min_dimensions.is_none());
        assert!(win_attribs.max_dimensions.is_none());

        match opengl.robustness {
            Robustness::RobustNoResetNotification | Robustness::RobustLoseContextOnReset => {
                return Err(CreationError::RobustnessNotSupported);
            },
            _ => ()
        }

        let winit_window = winit_builder.build().unwrap();
        let view = winit_window.get_nsview() as id;
        let (context, pf) = match Window::create_context(view, pf_reqs, opengl) {
            Ok((context, pf)) => (context, pf),
            Err(e) => { return Err(OsError(format!("Couldn't create OpenGL context: {}", e))); },
        };

        let window = Window {
            context: context,
            pixel_format: pf,
            winit_window: winit_window,
        };

        Ok(window)
    }

    fn create_context(view: id, pf_reqs: &PixelFormatRequirements, opengl: &GlAttributes<&Window>)
                      -> Result<(IdRef, PixelFormat), CreationError>
    {
        let attributes = try!(helpers::build_nsattributes(pf_reqs, opengl));
        unsafe {
            let pixelformat = IdRef::new(NSOpenGLPixelFormat::alloc(nil).initWithAttributes_(&attributes));

            if let Some(pixelformat) = pixelformat.non_nil() {

                // TODO: Add context sharing
                let context = IdRef::new(NSOpenGLContext::alloc(nil).initWithFormat_shareContext_(*pixelformat, nil));

                if let Some(cxt) = context.non_nil() {
                    let pf = {
                        let get_attr = |attrib: appkit::NSOpenGLPixelFormatAttribute| -> i32 {
                            let mut value = 0;

                            NSOpenGLPixelFormat::getValues_forAttribute_forVirtualScreen_(
                                *pixelformat,
                                &mut value,
                                attrib,
                                NSOpenGLContext::currentVirtualScreen(*cxt));

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

                    cxt.setView_(view);
                    let value = if opengl.vsync { 1 } else { 0 };
                    cxt.setValues_forParameter_(&value, appkit::NSOpenGLContextParameter::NSOpenGLCPSwapInterval);

                    CGLEnable(cxt.CGLContextObj() as *mut _, kCGLCECrashOnRemovedFunctions);

                    Ok((cxt, pf))
                } else {
                    Err(CreationError::NotSupported)
                }
            } else {
                Err(CreationError::NoAvailablePixelFormat)
            }
        }
    }

    pub fn set_title(&self, title: &str) {
        self.winit_window.set_title(title)
    }

    pub fn show(&self) {
        self.winit_window.show()
    }

    pub fn hide(&self) {
        self.winit_window.hide()
    }

    pub fn get_position(&self) -> Option<(i32, i32)> {
        self.winit_window.get_position()
    }

    pub fn set_position(&self, x: i32, y: i32) {
        self.winit_window.set_position(x, y)
    }

    pub fn get_inner_size(&self) -> Option<(u32, u32)> {
        self.winit_window.get_inner_size()
    }

    pub fn get_inner_size_points(&self) -> Option<(u32, u32)> {
        self.winit_window.get_inner_size()
    }

    pub fn get_inner_size_pixels(&self) -> Option<(u32, u32)> {
        self.winit_window.get_inner_size().map(|(x, y)| {
            let hidpi = self.hidpi_factor();
            ((x as f32 * hidpi) as u32, (y as f32 * hidpi) as u32)
        })
    }

    pub fn get_outer_size(&self) -> Option<(u32, u32)> {
        self.winit_window.get_outer_size()
    }

    pub fn set_inner_size(&self, x: u32, y: u32) {
        self.winit_window.set_inner_size(x, y)
    }

    pub fn poll_events(&self) -> winit::PollEventsIterator {
        self.winit_window.poll_events()
    }

    pub fn wait_events(&self) -> winit::WaitEventsIterator {
        self.winit_window.wait_events()
    }

    pub unsafe fn platform_display(&self) -> *mut libc::c_void {
        self.winit_window.platform_display()
    }

    pub unsafe fn platform_window(&self) -> *mut libc::c_void {
        self.winit_window.platform_window()
    }

    pub fn create_window_proxy(&self) -> winit::WindowProxy {
        self.winit_window.create_window_proxy()
    }

    pub fn set_window_resize_callback(&mut self, callback: Option<fn(u32, u32)>) {
        self.winit_window.set_window_resize_callback(callback);
    }

    pub fn set_cursor(&self, cursor: winit::MouseCursor) {
        self.winit_window.set_cursor(cursor);
    }

    pub fn hidpi_factor(&self) -> f32 {
        self.winit_window.hidpi_factor()
    }

    pub fn set_cursor_position(&self, x: i32, y: i32) -> Result<(), ()> {
        self.winit_window.set_cursor_position(x, y)
    }

    pub fn set_cursor_state(&self, state: winit::CursorState) -> Result<(), String> {
        self.winit_window.set_cursor_state(state)
    }
}

impl GlContext for Window {
    #[inline]
    unsafe fn make_current(&self) -> Result<(), ContextError> {
        let _: () = msg_send![*self.context, update];
        self.context.makeCurrentContext();
        Ok(())
    }

    #[inline]
    fn is_current(&self) -> bool {
        unsafe {
            let current = NSOpenGLContext::currentContext(nil);
            if current != nil {
                let is_equal: BOOL = msg_send![current, isEqual:*self.context];
                is_equal != NO
            } else {
                false
            }
        }
    }

    fn get_proc_address(&self, addr: &str) -> *const () {
        let symbol_name: CFString = FromStr::from_str(addr).unwrap();
        let framework_name: CFString = FromStr::from_str("com.apple.opengl").unwrap();
        let framework = unsafe {
            CFBundleGetBundleWithIdentifier(framework_name.as_concrete_TypeRef())
        };
        let symbol = unsafe {
            CFBundleGetFunctionPointerForName(framework, symbol_name.as_concrete_TypeRef())
        };
        symbol as *const _
    }

    #[inline]
    fn swap_buffers(&self) -> Result<(), ContextError> {
        unsafe {
            let pool = NSAutoreleasePool::new(nil);
            self.context.flushBuffer();
            let _: () = msg_send![pool, release];
        }
        Ok(())
    }

    #[inline]
    fn get_api(&self) -> ::Api {
        ::Api::OpenGl
    }

    #[inline]
    fn get_pixel_format(&self) -> PixelFormat {
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
