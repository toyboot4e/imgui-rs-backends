/*!
GPU resources
*/

use anyhow::*;
use glow::HasContext;
use std::{any::TypeId, marker::PhantomData, mem::size_of};

/// Max number of quadliterals
pub const N_QUADS: usize = 2048;

const VS_SRC: &'static str = include_str!("vs.glsl");
const FS_SRC: &'static str = include_str!("fs.glsl");

unsafe fn gen_shader_program(gl: &glow::Context, sources: &[(u32, &str)]) -> glow::Program {
    let program = gl.create_program().expect("Cannot create program");

    let mut shaders = Vec::with_capacity(sources.len());

    for (type_, src) in sources.iter() {
        let shader = gl.create_shader(*type_).expect("Cannot create shader");
        gl.shader_source(shader, &src);
        gl.compile_shader(shader);
        if !gl.get_shader_compile_status(shader) {
            panic!("{}", gl.get_shader_info_log(shader));
        }
        gl.attach_shader(program, shader);
        shaders.push(shader);
    }

    gl.link_program(program);
    if !gl.get_program_link_status(program) {
        panic!("{}", gl.get_program_info_log(program));
    }

    for shader in shaders {
        gl.detach_shader(program, shader);
        gl.delete_shader(shader);
    }

    program
}

unsafe fn alloc_buffer(gl: &glow::Context, type_: u32, capacity: usize) -> Result<glow::Buffer> {
    let buf = gl.create_buffer().map_err(Error::msg)?;
    gl.bind_buffer(type_, Some(buf));
    gl.buffer_data_size(type_, capacity as i32, glow::STREAM_DRAW);
    gl.bind_buffer(type_, None);
    Ok(buf)
}

struct Buffer<T> {
    // vertex/index buffer
    type_: u32,
    id: glow::Buffer,
    len_bytes: i32,
    capacity_bytes: i32,
    _marker: PhantomData<T>,
}

impl<T> Buffer<T> {
    pub fn new(gl: &glow::Context, type_: u32, len: usize) -> Result<Self> {
        assert!(type_ == glow::ARRAY_BUFFER || type_ == glow::ELEMENT_ARRAY_BUFFER);
        let capacity_bytes = size_of::<T>() * len;
        assert!(capacity_bytes < i32::MAX as usize);

        let id = unsafe { self::alloc_buffer(gl, type_, capacity_bytes)? };

        Ok(Self {
            type_,
            id,
            len_bytes: 0,
            capacity_bytes: capacity_bytes as i32,
            _marker: PhantomData,
        })
    }

    pub fn reset_offset(&mut self) {
        self.len_bytes = 0;
    }

    pub fn append(&mut self, gl: &glow::Context, data: &[T]) {
        let len_bytes = size_of::<T>() * data.len();
        let new_len_bytes = self.len_bytes + len_bytes as i32;
        assert!(new_len_bytes <= self.capacity_bytes);
        unsafe {
            let bytes: &[u8] = std::slice::from_raw_parts(data.as_ptr() as *const _, len_bytes);
            // FIXME:
            gl.buffer_sub_data_u8_slice(self.type_, self.len_bytes, bytes);
        }
        self.len_bytes = new_len_bytes;
    }
}

/// GPU resources
pub struct Resources {
    // pipeline
    vao: glow::VertexArray,
    program: glow::Program,
    // GPU buffers and texture slot
    vbuf: Buffer<imgui::DrawVert>,
    ibuf: Buffer<imgui::DrawIdx>,
    // TODO:
    // vbuf_cpu: Vec<imgui::DrawVert>,
    // ibuf_cpu: Vec<imgui::DrawVert>,
    /// We won't free this texture on drop; basically a weak reference
    tex: Option<glow::Texture>,
}

impl Resources {
    /// Allocates GPU resources
    pub fn new(gl: &glow::Context) -> Result<Self> {
        unsafe {
            let vao = gl
                .create_vertex_array()
                .expect("Cannot create vertex array");

            let program = self::gen_shader_program(
                gl,
                &[
                    (glow::VERTEX_SHADER, VS_SRC),
                    (glow::FRAGMENT_SHADER, FS_SRC),
                ],
            );

            let vbuf = Buffer::new(
                gl,
                glow::ARRAY_BUFFER,
                4 * N_QUADS * size_of::<imgui::DrawVert>(),
            )?;

            let ibuf = Buffer::new(
                gl,
                glow::ELEMENT_ARRAY_BUFFER,
                6 * N_QUADS * size_of::<imgui::DrawIdx>(),
            )?;

            Ok(Self {
                vao,
                program,
                vbuf,
                ibuf,
                tex: None,
            })
        }
    }

    pub unsafe fn free(&mut self, gl: &glow::Context) {
        gl.delete_program(self.program);
        gl.delete_vertex_array(self.vao);
        gl.delete_buffer(self.vbuf.id);
        gl.delete_buffer(self.ibuf.id);
    }
}

impl Resources {
    pub fn reset_buf_offsets(&mut self) {
        self.vbuf.reset_offset();
        self.ibuf.reset_offset();
    }

    pub fn set_texture(&mut self, tex: glow::Texture) {
        self.tex = Some(tex);
    }

    pub fn append_vbuf(&mut self, gl: &glow::Context, vbuf: &[imgui::DrawVert]) {
        self.vbuf.append(gl, vbuf);
    }

    pub fn append_ibuf(&mut self, gl: &glow::Context, ibuf: &[imgui::DrawIdx]) {
        self.ibuf.append(gl, ibuf);
    }
}

impl Resources {
    pub unsafe fn set_uniforms(&self, gl: &glow::Context, mat: [f32; 16]) {
        let location = gl
            // we must not add '\0' here -- glow does it
            .get_uniform_location(self.program, "transform")
            .expect("Unable to locate transform uniform");
        gl.uniform_matrix_4_f32_slice(Some(&location), false, &mat);
    }

    pub unsafe fn bind(&self, gl: &glow::Context) {
        // NOTE: The order is important.. bind buffers first and then setup VAO!
        gl.bind_vertex_array(Some(self.vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbuf.id));
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.ibuf.id));
        // TODO: disable dangling attributes
        self::set_vertex_attributes(gl);

        gl.use_program(Some(self.program));
        gl.bind_texture(glow::TEXTURE_2D, self.tex);

        // use alpha blending. use scissor test.
        gl.enable(glow::BLEND);
        gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        gl.enable(glow::SCISSOR_TEST);

        // TODO: not needed to set states?
        gl.disable(glow::DEPTH_TEST);
        gl.disable(glow::CULL_FACE);
        gl.disable(glow::STENCIL_TEST);
    }

    pub unsafe fn unbind(gl: &glow::Context) {
        gl.bind_vertex_array(None);
        gl.use_program(None);
        // TODO: disable attributes?
        gl.bind_texture(glow::TEXTURE_2D, None);
        gl.bind_buffer(glow::ARRAY_BUFFER, None);
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);
    }

    pub unsafe fn draw(&self, gl: &glow::Context, n_elems: i32, idx_offset: i32, vtx_offset: i32) {
        gl.draw_elements_base_vertex(
            // mode
            glow::TRIANGLES,
            // count
            n_elems,
            if std::any::TypeId::of::<imgui::DrawIdx>() == std::any::TypeId::of::<u16>() {
                glow::UNSIGNED_SHORT
            } else {
                glow::UNSIGNED_INT
            },
            idx_offset * size_of::<imgui::DrawIdx>() as i32,
            vtx_offset,
        );
    }
}

pub unsafe fn set_vertex_attributes(gl: &glow::Context) {
    let stride = size_of::<imgui::DrawVert>() as i32;

    // pos: [f32: 2]
    let index = 0;
    gl.enable_vertex_attrib_array(index);
    gl.vertex_attrib_pointer_f32(
        // index: nth component of a vertex
        index,
        // size: number of `data_type`
        2,
        // data_type
        glow::FLOAT,
        // normalized
        false,
        // stride: size of vertex
        stride,
        // offset: byte offset of this component in a vertex
        0,
    );
    gl.enable_vertex_attrib_array(index);

    // uv: [f32: 2]
    let index = 1;
    gl.enable_vertex_attrib_array(index);
    gl.vertex_attrib_pointer_f32(
        // index
        index,
        // size
        2,
        // data_type
        glow::FLOAT,
        // normalized
        false,
        // stride
        stride,
        // offset
        2 * size_of::<f32>() as i32,
    );

    // color: [u8: 2]
    let index = 2;
    gl.enable_vertex_attrib_array(index);
    gl.vertex_attrib_pointer_f32(
        index,
        // size
        4,
        // data_type
        glow::UNSIGNED_BYTE,
        // normalized
        true,
        // stride
        stride,
        // offset
        4 * size_of::<f32>() as i32,
    );

    // TODO:
    // gl.set_vertex_array(None);
}
