#![cfg(target_os = "ios")]

//! iOS support
//!
//! # Building app
//! To build ios app you will need rustc built for this targets:
//!
//!  - armv7-apple-ios
//!  - armv7s-apple-ios
//!  - i386-apple-ios
//!  - aarch64-apple-ios
//!  - x86_64-apple-ios
//!
//! Then
//!
//! ```
//! cargo build --target=...
//! ```
//! The simplest way to integrate your app into xcode environment is to build it
//! as a static library. Wrap your main function and export it.
//!
//! ```rust, ignore
//! #[no_mangle]
//! pub extern fn start_glutin_app() {
//!     start_inner()
//! }
//!
//! fn start_inner() {
//!    ...
//! }
//! ```
//!
//! Compile project and then drag resulting .a into Xcode project. Add glutin.h
//! to xcode.
//!
//! ```c
//! void start_glutin_app();
//! ```
//!
//! Use start_glutin_app inside your xcode's main function.
//!
//!
//! # App lifecycle and events
//!
//! iOS environment is very different from other platforms and you must be very
//! careful with it's events. Familiarize yourself with [app lifecycle](https://developer.apple.com/library/ios/documentation/UIKit/Reference/UIApplicationDelegate_Protocol/).
//!
//!
//! This is how those event are represented in glutin:
//!
//!  - applicationDidBecomeActive is Focused(true)
//!  - applicationWillResignActive is Focused(false)
//!  - applicationDidEnterBackground is Suspended(true)
//!  - applicationWillEnterForeground is Suspended(false)
//!  - applicationWillTerminate is Destroyed
//!
//! Keep in mind that after Destroyed event is received every attempt to draw
//! with opengl will result in segfault.
//!
//! Also note that app will not receive Destroyed event if suspended, it will be
//! SIGKILL'ed

use crate::platform::ios::{WindowBuilderExtIOS, WindowExtIOS};
use crate::{
    Api, ContextError, CreationError, GlAttributes, GlRequest, PixelFormat,
    PixelFormatRequirements, Rect,
};

use glutin_gles2_sys as ffi;
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel, BOOL, NO, YES};
use winit::dpi;
use winit::event_loop::EventLoopWindowTarget;
use winit::window::WindowBuilder;

use std::ffi::CString;
use std::os::raw;

#[derive(Debug, PartialEq)]
enum ColorFormat {
    Rgba8888 = 0,
    Rgb565 = 1,
    Srgba8888 = 2,
}

impl ColorFormat {
    #[allow(non_upper_case_globals)]
    pub fn for_view(view: ffi::id) -> Self {
        let color_format: ffi::NSUInteger =
            unsafe { msg_send![view, drawableColorFormat] };
        match color_format {
            ffi::GLKViewDrawableColorFormatRGBA8888 => ColorFormat::Rgba8888,
            ffi::GLKViewDrawableColorFormatRGB565 => ColorFormat::Rgb565,
            ffi::GLKViewDrawableColorFormatSRGBA8888 => ColorFormat::Srgba8888,
            _ => unreachable!(),
        }
    }

    pub fn color_bits(&self) -> u8 {
        if *self == ColorFormat::Rgba8888 || *self == ColorFormat::Srgba8888 {
            8
        } else {
            16
        }
    }

    pub fn alpha_bits(&self) -> u8 {
        if *self == ColorFormat::Rgba8888 || *self == ColorFormat::Srgba8888 {
            8
        } else {
            0
        }
    }

    pub fn srgb(&self) -> bool {
        *self == ColorFormat::Srgba8888
    }
}

#[allow(non_upper_case_globals)]
fn depth_for_view(view: ffi::id) -> u8 {
    let depth_format: ffi::NSUInteger =
        unsafe { msg_send![view, drawableDepthFormat] };
    match depth_format {
        ffi::GLKViewDrawableDepthFormatNone => 0,
        ffi::GLKViewDrawableDepthFormat16 => 16,
        ffi::GLKViewDrawableDepthFormat24 => 24,
        _ => unreachable!(),
    }
}

#[allow(non_upper_case_globals)]
fn stencil_for_view(view: ffi::id) -> u8 {
    let stencil_format: ffi::NSUInteger =
        unsafe { msg_send![view, drawableStencilFormat] };
    match stencil_format {
        ffi::GLKViewDrawableStencilFormatNone => 0,
        ffi::GLKViewDrawableStencilFormat8 => 8,
        _ => unreachable!(),
    }
}

#[allow(non_upper_case_globals)]
fn multisampling_for_view(view: ffi::id) -> Option<u16> {
    let ms_format: ffi::NSUInteger =
        unsafe { msg_send![view, drawableMultisample] };
    match ms_format {
        ffi::GLKViewDrawableMultisampleNone => None,
        ffi::GLKViewDrawableMultisample4X => Some(4),
        _ => unreachable!(),
    }
}

#[derive(Debug)]
pub struct Context {
    eagl_context: ffi::id,
    view: ffi::id, // this will be invalid after the `EventLoop` is dropped
}

fn validate_version(version: u8) -> Result<ffi::NSUInteger, CreationError> {
    let version = version as ffi::NSUInteger;
    if version >= ffi::kEAGLRenderingAPIOpenGLES1
        && version <= ffi::kEAGLRenderingAPIOpenGLES3
    {
        Ok(version)
    } else {
        Err(CreationError::OsError(format!(
            "Specified OpenGL ES version ({:?}) is not availble on iOS. Only 1, 2, and 3 are valid options",
            version,
        )))
    }
}

impl Context {
    #[inline]
    pub fn new_windowed<T>(
        builder: WindowBuilder,
        el: &EventLoopWindowTarget<T>,
        _: &PixelFormatRequirements,
        gl_attrs: &GlAttributes<&Context>,
    ) -> Result<(winit::window::Window, Self), CreationError> {
        create_view_class();
        let view_class =
            Class::get("MainGLView").expect("Failed to get class `MainGLView`");
        let builder =
            builder.with_root_view_class(view_class as *const _ as *const _);
        if gl_attrs.sharing.is_some() {
            unimplemented!("Shared contexts are unimplemented on iOS.");
        }
        let version = match gl_attrs.version {
            GlRequest::Latest => ffi::kEAGLRenderingAPIOpenGLES3,
            GlRequest::Specific(api, (major, _minor)) => {
                if api == Api::OpenGlEs {
                    validate_version(major)?
                } else {
                    return Err(CreationError::OsError(format!(
                    "Specified API ({:?}) is not availble on iOS. Only `Api::OpenGlEs` can be used",
                    api,
                )));
                }
            }
            GlRequest::GlThenGles {
                opengles_version: (major, _minor),
                ..
            } => validate_version(major)?,
        };
        let win = builder.build(el)?;
        let context = unsafe {
            let eagl_context = Context::create_context(version)?;
            let view = win.ui_view() as ffi::id;
            let mut context = Context { eagl_context, view };
            context.init_context(&win);
            context
        };
        Ok((win, context))
    }

    #[inline]
    pub fn new_headless<T>(
        el: &EventLoopWindowTarget<T>,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
        size: dpi::PhysicalSize<u32>,
    ) -> Result<Self, CreationError> {
        let wb = winit::window::WindowBuilder::new()
            .with_visible(false)
            .with_inner_size(size);
        Self::new_windowed(wb, el, pf_reqs, gl_attr)
            .map(|(_window, context)| context)
    }

    unsafe fn create_context(
        mut version: ffi::NSUInteger,
    ) -> Result<ffi::id, CreationError> {
        let context_class = Class::get("EAGLContext")
            .expect("Failed to get class `EAGLContext`");
        let eagl_context: ffi::id = msg_send![context_class, alloc];
        let mut valid_context = ffi::nil;
        while valid_context == ffi::nil && version > 0 {
            valid_context = msg_send![eagl_context, initWithAPI: version];
            version -= 1;
        }
        if valid_context == ffi::nil {
            Err(CreationError::OsError(
                "Failed to create an OpenGL ES context with any version"
                    .to_string(),
            ))
        } else {
            Ok(eagl_context)
        }
    }

    unsafe fn init_context(&mut self, win: &winit::window::Window) {
        let dict_class = Class::get("NSDictionary")
            .expect("Failed to get class `NSDictionary`");
        let number_class =
            Class::get("NSNumber").expect("Failed to get class `NSNumber`");
        let draw_props: ffi::id = msg_send![dict_class, alloc];
        let draw_props: ffi::id = msg_send![draw_props,
            initWithObjects:
                vec![
                    msg_send![number_class, numberWithBool:NO],
                    ffi::kEAGLColorFormatRGB565,
                ].as_ptr()
            forKeys:
                vec![
                    ffi::kEAGLDrawablePropertyRetainedBacking,
                    ffi::kEAGLDrawablePropertyColorFormat,
                ].as_ptr()
            count: 2
        ];
        self.make_current().unwrap();

        let view = self.view;
        let scale_factor = win.scale_factor() as ffi::CGFloat;
        let _: () = msg_send![view, setContentScaleFactor: scale_factor];
        let layer: ffi::id = msg_send![view, layer];
        let _: () = msg_send![layer, setContentsScale: scale_factor];
        let _: () = msg_send![layer, setDrawableProperties: draw_props];

        let gl = ffi::gles::Gles2::load_with(|symbol| {
            self.get_proc_address(symbol) as *const raw::c_void
        });
        let mut color_render_buf: ffi::gles::types::GLuint = 0;
        let mut frame_buf: ffi::gles::types::GLuint = 0;
        gl.GenRenderbuffers(1, &mut color_render_buf);
        gl.BindRenderbuffer(ffi::gles::RENDERBUFFER, color_render_buf);

        let ok: BOOL = msg_send![self.eagl_context, renderbufferStorage:ffi::gles::RENDERBUFFER fromDrawable:layer];
        if ok != YES {
            panic!("EAGL: could not set renderbufferStorage");
        }

        gl.GenFramebuffers(1, &mut frame_buf);
        gl.BindFramebuffer(ffi::gles::FRAMEBUFFER, frame_buf);

        gl.FramebufferRenderbuffer(
            ffi::gles::FRAMEBUFFER,
            ffi::gles::COLOR_ATTACHMENT0,
            ffi::gles::RENDERBUFFER,
            color_render_buf,
        );

        let status = gl.CheckFramebufferStatus(ffi::gles::FRAMEBUFFER);
        if gl.CheckFramebufferStatus(ffi::gles::FRAMEBUFFER)
            != ffi::gles::FRAMEBUFFER_COMPLETE
        {
            panic!("framebuffer status: {:?}", status);
        }
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        unsafe {
            let res: BOOL = msg_send![
                self.eagl_context,
                presentRenderbuffer: ffi::gles::RENDERBUFFER
            ];
            if res == YES {
                Ok(())
            } else {
                Err(ContextError::IoError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "`EAGLContext presentRenderbuffer` failed",
                )))
            }
        }
    }

    #[inline]
    pub fn swap_buffers_with_damage(
        &self,
        rects: &[Rect],
    ) -> Result<(), ContextError> {
        Err(ContextError::OsError(
            "buffer damage not suported".to_string(),
        ))
    }

    #[inline]
    pub fn swap_buffers_with_damage_supported(&self) -> bool {
        false
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        let color_format = ColorFormat::for_view(self.view);
        PixelFormat {
            hardware_accelerated: true,
            color_bits: color_format.color_bits(),
            alpha_bits: color_format.alpha_bits(),
            depth_bits: depth_for_view(self.view),
            stencil_bits: stencil_for_view(self.view),
            stereoscopy: false,
            double_buffer: true,
            multisampling: multisampling_for_view(self.view),
            srgb: color_format.srgb(),
        }
    }

    #[inline]
    pub fn resize(&self, _width: u32, _height: u32) {
        // N/A
    }

    #[inline]
    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        let context_class = Class::get("EAGLContext")
            .expect("Failed to get class `EAGLContext`");
        let res: BOOL =
            msg_send![context_class, setCurrentContext: self.eagl_context];
        if res == YES {
            Ok(())
        } else {
            Err(ContextError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                "`EAGLContext setCurrentContext` failed",
            )))
        }
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), ContextError> {
        if !self.is_current() {
            return Ok(());
        }

        let context_class = Class::get("EAGLContext")
            .expect("Failed to get class `EAGLContext`");
        let res: BOOL = msg_send![context_class, setCurrentContext: ffi::nil];
        if res == YES {
            Ok(())
        } else {
            Err(ContextError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                "`EAGLContext setCurrentContext` failed",
            )))
        }
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        // TODO: This can likely be implemented using
        // `currentContext`/`getCurrentContext`
        true
    }

    #[inline]
    pub fn get_proc_address(
        &self,
        proc_name: &str,
    ) -> *const core::ffi::c_void {
        let proc_name_c = CString::new(proc_name)
            .expect("proc name contained interior nul byte");
        let path = b"/System/Library/Frameworks/OpenGLES.framework/OpenGLES\0";
        let addr = unsafe {
            let lib = ffi::dlopen(
                path.as_ptr() as *const raw::c_char,
                ffi::RTLD_LAZY | ffi::RTLD_GLOBAL,
            );
            ffi::dlsym(lib, proc_name_c.as_ptr()) as *const _
        };
        // debug!("proc {} -> {:?}", proc_name, addr);
        addr
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> *mut raw::c_void {
        self.eagl_context as *mut raw::c_void
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        Api::OpenGlEs
    }
}

fn create_view_class() {
    extern "C" fn init_with_frame(
        this: &Object,
        _: Sel,
        frame: ffi::CGRect,
    ) -> ffi::id {
        unsafe {
            let view: ffi::id =
                msg_send![super(this, class!(GLKView)), initWithFrame: frame];

            let mask = ffi::UIViewAutoresizingFlexibleWidth
                | ffi::UIViewAutoresizingFlexibleHeight;
            let _: () = msg_send![view, setAutoresizingMask: mask];
            let _: () = msg_send![view, setAutoresizesSubviews: YES];

            let layer: ffi::id = msg_send![view, layer];
            let _: () = msg_send![layer, setOpaque: YES];

            view
        }
    }

    extern "C" fn layer_class(_: &Class, _: Sel) -> *const Class {
        unsafe {
            std::mem::transmute(
                Class::get("CAEAGLLayer")
                    .expect("Failed to get class `CAEAGLLayer`"),
            )
        }
    }

    let superclass =
        Class::get("GLKView").expect("Failed to get class `GLKView`");
    let mut decl = ClassDecl::new("MainGLView", superclass)
        .expect("Failed to declare class `MainGLView`");
    unsafe {
        decl.add_method(
            sel!(initWithFrame:),
            init_with_frame
                as extern "C" fn(&Object, Sel, ffi::CGRect) -> ffi::id,
        );
        decl.add_class_method(
            sel!(layerClass),
            layer_class as extern "C" fn(&Class, Sel) -> *const Class,
        );
        decl.register();
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        let _: () = unsafe { msg_send![self.eagl_context, release] };
    }
}

unsafe impl Send for Context {}
unsafe impl Sync for Context {}
