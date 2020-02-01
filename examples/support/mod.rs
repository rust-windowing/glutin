#![allow(dead_code)]

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]
use glutin::platform::unix::osmesa::{OsMesaBuffer, OsMesaContext, OsMesaContextBuilder};

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]
use glutin_interface::{NativeDisplay, RawDisplay, Seal};

use glutin::config::{Api, Config, ConfigsFinder, Version};
use glutin::context::{Context, ContextBuilder};
use glutin::surface::{PBuffer, Surface};
use winit::event_loop::EventLoop;
use winit_types::dpi::PhysicalSize;
use winit_types::error::{Error, ErrorType};

use std::ffi::CStr;
use std::os::raw;
use std::path::Path;

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]
pub unsafe fn load_egl_sym(lib: &libloading::Library, name: &str) -> *const raw::c_void {
    type FnEglGetProcAddress = unsafe extern "C" fn(*mut raw::c_char) -> *mut raw::c_void;

    let name = std::ffi::CString::new(name.as_bytes()).unwrap();

    let name = name.as_bytes_with_nul();
    match lib.get::<*const raw::c_void>(name) {
        Err(_) => {
            let egl_get_proc_address_fn: libloading::Symbol<FnEglGetProcAddress> =
                lib.get(b"eglGetProcAddress\0").unwrap();
            (egl_get_proc_address_fn)(name.as_ptr() as *mut raw::c_char) as *const _
        }
        Ok(sym) => {
            assert!(!(*sym).is_null());
            *sym
        }
    }
}

pub mod gl {
    pub use self::Gles2 as Gl;
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));
}

pub struct Gl {
    pub gl: gl::Gl,
}

impl Gl {
    pub fn load<F>(loadfn: F) -> Self
    where
        F: FnMut(&'static str) -> *const raw::c_void,
    {
        let gl = gl::Gl::load_with(loadfn);

        let version = unsafe {
            let data = CStr::from_ptr(gl.GetString(gl::VERSION) as *const _)
                .to_bytes()
                .to_vec();
            String::from_utf8(data).unwrap()
        };

        println!("OpenGL version {}", version);

        unsafe {
            let vs = gl.CreateShader(gl::VERTEX_SHADER);
            gl.ShaderSource(
                vs,
                1,
                [VS_SRC.as_ptr() as *const _].as_ptr(),
                std::ptr::null(),
            );
            gl.CompileShader(vs);

            let fs = gl.CreateShader(gl::FRAGMENT_SHADER);
            gl.ShaderSource(
                fs,
                1,
                [FS_SRC.as_ptr() as *const _].as_ptr(),
                std::ptr::null(),
            );
            gl.CompileShader(fs);

            let program = gl.CreateProgram();
            gl.AttachShader(program, vs);
            gl.AttachShader(program, fs);
            gl.LinkProgram(program);
            gl.UseProgram(program);

            let mut vb = 0;
            gl.GenBuffers(1, &mut vb);
            gl.BindBuffer(gl::ARRAY_BUFFER, vb);
            gl.BufferData(
                gl::ARRAY_BUFFER,
                (VERTEX_DATA.len() * std::mem::size_of::<f32>()) as gl::types::GLsizeiptr,
                VERTEX_DATA.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );

            if gl.BindVertexArray.is_loaded() {
                let mut vao = 0;
                gl.GenVertexArrays(1, &mut vao);
                gl.BindVertexArray(vao);
            }

            let pos_attrib = gl.GetAttribLocation(program, b"position\0".as_ptr() as *const _);
            let color_attrib = gl.GetAttribLocation(program, b"color\0".as_ptr() as *const _);
            gl.VertexAttribPointer(
                pos_attrib as gl::types::GLuint,
                2,
                gl::FLOAT,
                0,
                5 * std::mem::size_of::<f32>() as gl::types::GLsizei,
                std::ptr::null(),
            );
            gl.VertexAttribPointer(
                color_attrib as gl::types::GLuint,
                3,
                gl::FLOAT,
                0,
                5 * std::mem::size_of::<f32>() as gl::types::GLsizei,
                (2 * std::mem::size_of::<f32>()) as *const () as *const _,
            );
            gl.EnableVertexAttribArray(pos_attrib as gl::types::GLuint);
            gl.EnableVertexAttribArray(color_attrib as gl::types::GLuint);
        }

        Gl { gl }
    }

    pub fn draw_frame(&self, color: [f32; 4]) {
        unsafe {
            self.gl.ClearColor(color[0], color[1], color[2], color[3]);
            self.gl.Clear(gl::COLOR_BUFFER_BIT);
            self.gl.DrawArrays(gl::TRIANGLES, 0, 3);
        }
    }

    pub fn make_renderbuf(&self, size: PhysicalSize<u32>) -> gl::types::GLuint {
        unsafe {
            let mut render_buf = 0;
            self.gl.GenRenderbuffers(1, &mut render_buf);
            self.gl.BindRenderbuffer(gl::RENDERBUFFER, render_buf);
            self.gl.RenderbufferStorage(
                gl::RENDERBUFFER,
                gl::RGB8,
                size.width as _,
                size.height as _,
            );
            render_buf
        }
    }

    pub fn make_framebuffer(&self, render_buf: gl::types::GLuint) -> gl::types::GLuint {
        unsafe {
            let mut fb = 0;
            self.gl.GenFramebuffers(1, &mut fb);
            self.gl.BindFramebuffer(gl::FRAMEBUFFER, fb);
            self.gl.BindRenderbuffer(gl::RENDERBUFFER, render_buf);
            self.gl.FramebufferRenderbuffer(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::RENDERBUFFER,
                render_buf,
            );

            fb
        }
    }

    pub fn export_to_file(&self, size: PhysicalSize<u32>, path: &Path) {
        println!("Exporting to file {}", path.display());

        let mut pixels: Vec<gl::types::GLubyte> = vec![];
        pixels.resize(3 * size.width as usize * size.height as usize, 0);
        unsafe {
            // You probably want the format here to match to the T, else you
            // can easily confuse Mesa.
            //
            // For example, if you use a 30bpp config (with alpha or without)
            // you should instead pass RGBA and GL_UNSIGNED_INT_2_10_10_10_REV.
            //
            // If you don't pass RGBA, you will get a black image. If you don't
            // pass GL_UNSIGNED_INT_2_10_10_10_REV or you don't pass both you
            // will get the wrong colors.
            self.gl.ReadPixels(
                0,
                0,
                size.width as _,
                size.height as _,
                gl::RGB,
                gl::UNSIGNED_BYTE,
                pixels.as_mut_ptr() as *mut _,
            );
        }

        let mut pixels_flipped: Vec<gl::types::GLubyte> = vec![];
        for v in (0..size.height as _).rev() {
            let s = 3 * v as usize * size.width as usize;
            let o = 3 * size.width as usize;
            pixels_flipped.extend_from_slice(&pixels[s..(s + o)]);
        }

        image::save_buffer(
            path,
            &pixels_flipped,
            size.width as u32,
            size.height as u32,
            image::RGB(8),
        )
        .unwrap();
    }
}

#[rustfmt::skip]
static VERTEX_DATA: [f32; 15] = [
    -0.5, -0.5,  1.0,  0.0,  0.0,
     0.0,  0.5,  0.0,  1.0,  0.0,
     0.5, -0.5,  0.0,  0.0,  1.0,
];

const VS_SRC: &'static [u8] = b"
#version 100
precision mediump float;

attribute vec2 position;
attribute vec3 color;

varying vec3 v_color;

void main() {
    gl_Position = vec4(position, 0.0, 1.0);
    v_color = color;
}
\0";

const FS_SRC: &'static [u8] = b"
#version 100
precision mediump float;

varying vec3 v_color;

void main() {
    gl_FragColor = vec4(v_color, 1.0);
}
\0";

pub enum HeadlessBackend {
    Surfaceless(Context),
    PBuffer(Context, Surface<PBuffer>),
    OsMesa(OsMesaContext, OsMesaBuffer),
}

impl HeadlessBackend {
    pub unsafe fn new<T>(
        el: &EventLoop<T>,
        size: PhysicalSize<u32>,
        must_support_windows: bool,
    ) -> Result<(Self, PhysicalSize<u32>, Option<Config>), Error> {
        let mut errs = winit_types::make_error!(ErrorType::NotSupported(
            "None of the backends seem to work!".to_string()
        ));

        match ConfigsFinder::new()
            .with_must_support_surfaceless(true)
            .with_must_support_windows(must_support_windows)
            .with_gl((Api::OpenGl, Version(3, 0)))
            .find(&**el)
        {
            Ok(mut confs) => {
                let conf = confs.drain(..1).next().unwrap();
                println!("Surfaceless configeration chosen: {:#?}", conf);
                let ctx = ContextBuilder::new().build(&conf).unwrap();
                return Ok((HeadlessBackend::Surfaceless(ctx), size.clone(), Some(conf)));
            }
            Err(err) => errs.append(err),
        }

        match ConfigsFinder::new()
            .with_must_support_pbuffers(true)
            .with_must_support_windows(must_support_windows)
            .with_gl((Api::OpenGl, Version(3, 0)))
            .find(&**el)
        {
            Ok(mut confs) => {
                let conf = confs.drain(..1).next().unwrap();
                println!("PBuffer configeration chosen: {:#?}", conf);

                let ctx = ContextBuilder::new().build(&conf).unwrap();
                let surf = Surface::new_pbuffer(&conf, size, true).unwrap();
                let size = surf.size().unwrap();

                return Ok((HeadlessBackend::PBuffer(ctx, surf), size, Some(conf)));
            }
            Err(err) => errs.append(err),
        }

        if must_support_windows {
            return Err(errs);
        }

        #[cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd",
        ))]
        {
            struct EglMesaSurfaceless;
            impl NativeDisplay for EglMesaSurfaceless {
                fn raw_display(&self) -> RawDisplay {
                    RawDisplay::EglMesaSurfaceless {
                        _non_exhaustive_do_not_use: Seal,
                    }
                }
            }

            match ConfigsFinder::new()
                .with_must_support_surfaceless(true)
                .with_must_support_windows(must_support_windows)
                .with_gl((Api::OpenGl, Version(3, 0)))
                .find(&EglMesaSurfaceless)
            {
                Ok(mut confs) => {
                    let conf = confs.drain(..1).next().unwrap();
                    println!("EGL Mesa Surfaceless configeration chosen: {:#?}", conf);
                    let ctx = ContextBuilder::new().build(&conf).unwrap();
                    return Ok((HeadlessBackend::Surfaceless(ctx), size.clone(), Some(conf)));
                }
                Err(err) => errs.append(err),
            }

            match ConfigsFinder::new()
                .with_must_support_pbuffers(true)
                .with_must_support_windows(must_support_windows)
                .with_gl((Api::OpenGl, Version(3, 0)))
                .find(&EglMesaSurfaceless)
            {
                Ok(mut confs) => {
                    let conf = confs.drain(..1).next().unwrap();
                    println!(
                        "Egl Mesa Surfaceless PBuffer configeration chosen: {:#?}",
                        conf
                    );

                    let ctx = ContextBuilder::new().build(&conf).unwrap();
                    let surf = Surface::new_pbuffer(&conf, size, true).unwrap();
                    let size = surf.size().unwrap();

                    return Ok((HeadlessBackend::PBuffer(ctx, surf), size, Some(conf)));
                }
                Err(err) => errs.append(err),
            }
        }

        #[cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd",
        ))]
        match OsMesaContextBuilder::new().build(Version(3, 0)) {
            Ok(ctx) => match OsMesaBuffer::new(size) {
                Ok(buf) => {
                    println!("OsMesa chosen");
                    return Ok((HeadlessBackend::OsMesa(ctx, buf), size.clone(), None));
                }
                Err(err) => errs.append(err),
            },
            Err(err) => errs.append(err),
        }

        return Err(errs);
    }

    pub unsafe fn make_current(&self) -> Result<(), Error> {
        match self {
            HeadlessBackend::Surfaceless(ctx) => ctx.make_current_surfaceless(),
            HeadlessBackend::PBuffer(ctx, surf) => ctx.make_current(surf),
            HeadlessBackend::OsMesa(ctx, buf) => ctx.make_current(buf),
        }
    }

    pub fn load_symbols(&self) -> Result<Gl, Error> {
        unsafe {
            self.make_current()?;
        }
        let gl = match self {
            HeadlessBackend::Surfaceless(ctx) | HeadlessBackend::PBuffer(ctx, _) => {
                Gl::load(|s| ctx.get_proc_address(s).unwrap())
            }
            HeadlessBackend::OsMesa(ctx, _) => Gl::load(|s| ctx.get_proc_address(s).unwrap()),
        };
        Ok(gl)
    }

    pub fn context(&self) -> &Context {
        match self {
            HeadlessBackend::Surfaceless(ctx) | HeadlessBackend::PBuffer(ctx, _) => ctx,
            _ => panic!(),
        }
    }
}
