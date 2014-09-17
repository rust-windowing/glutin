#[phase(plugin)]
extern crate gl_generator;

use gl_init;

#[cfg(not(target_os = "android"))]
mod gl {
    generate_gl_bindings!("gl", "core", "1.1", "struct")
}

#[cfg(target_os = "android")]
mod gl {
    pub use self::Gles1 as Gl;
    generate_gl_bindings!("gles1", "core", "1.1", "struct")
}

pub struct Context {
    gl: gl::Gl
}

pub fn load(window: &gl_init::Window) -> Context {
    let gl = gl::Gl::load_with(|symbol| window.get_proc_address(symbol));

    let version = {
        use std::c_str::CString;
        unsafe { CString::new(gl.GetString(gl::VERSION) as *const i8, false) }
    };

    println!("OpenGL version {}", version.as_str().unwrap());

    Context { gl: gl }
}

impl Context {
    #[cfg(not(target_os = "android"))]
    pub fn draw_frame(&self, color: (f32, f32, f32, f32)) {
        self.gl.ClearColor(color.0, color.1, color.2, color.3);
        self.gl.Clear(gl::COLOR_BUFFER_BIT);

        self.gl.Begin(gl::TRIANGLES);
        self.gl.Color3f(1.0, 0.0, 0.0);
        self.gl.Vertex2f(-0.5, -0.5);
        self.gl.Color3f(0.0, 1.0, 0.0);
        self.gl.Vertex2f(0.0, 0.5);
        self.gl.Color3f(0.0, 0.0, 1.0);
        self.gl.Vertex2f(0.5, -0.5);
        self.gl.End();
    }

    #[cfg(target_os = "android")]
    pub fn draw_frame(&self, color: (f32, f32, f32, f32)) {
        self.gl.ClearColor(color.0, color.1, color.2, color.3);
        self.gl.Clear(gl::COLOR_BUFFER_BIT);

        let vertex_data: [f32, ..15] = [
            -0.5, -0.5, 1.0, 0.0, 0.0,
            0.0, 0.5, 0.0, 1.0, 0.0,
            0.5, -0.5, 0.0, 0.0, 1.0
        ];

        self.gl.EnableClientState(gl::VERTEX_ARRAY);
        self.gl.EnableClientState(gl::COLOR_ARRAY);

        unsafe {
            use std::mem;
            self.gl.VertexPointer(2, gl::FLOAT, (mem::size_of::<f32>() * 5) as i32,
                mem::transmute(vertex_data.as_slice().as_ptr()));
            self.gl.ColorPointer(3, gl::FLOAT, (mem::size_of::<f32>() * 5) as i32,
                mem::transmute(vertex_data.as_slice().as_ptr().offset(2)));
        }

        self.gl.DrawArrays(gl::TRIANGLES, 0, 3);
        self.gl.DisableClientState(gl::VERTEX_ARRAY);
        self.gl.DisableClientState(gl::COLOR_ARRAY);
    }
}
