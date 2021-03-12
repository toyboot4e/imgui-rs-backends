/*!
ImGUI renderer implementation in FNA3D based on [the example]

[the example]: https://github.com/Gekkio/imgui-rs/blob/master/imgui-gfx-renderer/src/lib.rs

* FIXME: It is a bad practice to use `raw_device` field because it may drop earlier than Device
* FIXME: Don't use batcher?
*/

use {
    anyhow::{Context, Error},
    imgui::{im_str, BackendFlags, DrawCmdParams, DrawData},
    std::{mem::size_of, rc::Rc},
    thiserror::Error,
};

use crate::{helper::RendererImplUtil, Renderer};

/// `SpriteEffect.fxb`
pub const SHADER: &[u8] = include_bytes!("fna3d/SpriteEffect.fxb");

/// `mplus-1p-regular.ttf`
pub const JP_FONT: &[u8] = include_bytes!("../../assets/mplus-1p-regular.ttf");

/// Number of quadliterals
const N_QUADS: usize = 2048;

/// Size of a vertex in bytes
const VERT_SIZE: usize = 20;

/// Size of an index in bytes
const INDEX_SIZE: usize = 2;

// TODO: extend and use this error
#[derive(Debug, Error)]
pub enum ImGuiRendererError {
    #[error("bad texture id")]
    BadTexture(imgui::TextureId),
}

/// Result<T, ImGuiRendererError>
pub type Result<T> = std::result::Result<T, ImGuiRendererError>;

/// GPU texture with size
#[derive(Debug)]
pub struct TextureData2d {
    pub raw: *mut fna3d::Texture,
    device: fna3d::Device,
    pub w: u32,
    pub h: u32,
}

impl Drop for TextureData2d {
    fn drop(&mut self) {
        self.device.add_dispose_texture(self.raw);
    }
}

/// Reference counted version of [`TextureData2d`]
#[derive(Debug)]
pub struct RcTexture2d {
    pub texture: Rc<TextureData2d>,
}

impl RcTexture2d {
    pub fn new(raw: *mut fna3d::Texture, device: fna3d::Device, w: u32, h: u32) -> Self {
        Self {
            texture: Rc::new(TextureData2d { raw, device, w, h }),
        }
    }
}

/// FNA3D ImGUI renderer
#[derive(Debug)]
pub struct ImGuiFna3d {
    textures: imgui::Textures<RcTexture2d>,
    font_texture: RcTexture2d,
    // TODO: remove batch?
    batch: Batch,
}

impl ImGuiFna3d {
    /// Add font before loading
    pub fn init(icx: &mut imgui::Context, device: &fna3d::Device) -> Result<Self> {
        icx.set_renderer_name(Some(im_str!(
            "imgui-fna3d-renderer {}",
            env!("CARGO_PKG_VERSION")
        )));

        icx.io_mut()
            .backend_flags
            .insert(BackendFlags::RENDERER_HAS_VTX_OFFSET);

        let font_texture = Self::load_font_texture(device, icx.fonts())?;

        Ok(ImGuiFna3d {
            textures: imgui::Textures::new(),
            font_texture,
            batch: Batch::new(device.clone()),
        })
    }

    /// Be warned that the font texture is  non-premultiplied alpha
    fn load_font_texture(
        device: &fna3d::Device,
        mut fonts: imgui::FontAtlasRefMut,
    ) -> Result<RcTexture2d> {
        let atlas_texture = fonts.build_rgba32_texture();
        let (pixels, w, h) = (
            atlas_texture.data,
            atlas_texture.width,
            atlas_texture.height,
        );

        let raw = {
            let fmt = fna3d::SurfaceFormat::Color;
            let gpu_texture = device.create_texture_2d(fmt, w, h, 1, false);
            device.set_texture_data_2d(gpu_texture, 0, 0, w, h, 0, pixels);
            gpu_texture
        };

        let font_texture = TextureData2d {
            raw,
            device: device.clone(),
            w,
            h,
        };

        // Note that we have to set the ID *AFTER* creating the font atlas texture
        fonts.tex_id = imgui::TextureId::from(usize::MAX);

        Ok(RcTexture2d {
            texture: Rc::new(font_texture),
        })
    }

    pub fn textures_mut(&mut self) -> &mut imgui::Textures<RcTexture2d> {
        &mut self.textures
    }

    /// Be warned that the font texture is  non-premultiplied alpha
    pub fn font_texture(&self) -> &TextureData2d {
        &self.font_texture.texture
    }
}

impl Renderer for ImGuiFna3d {
    type Device = fna3d::Device;
    type Error = anyhow::Error;

    fn render(
        &mut self,
        draw_data: &imgui::DrawData,
        device: &mut Self::Device,
    ) -> std::result::Result<(), Self::Error> {
        crate::helper::render(self, draw_data, device)
    }
}

impl RendererImplUtil for ImGuiFna3d {
    fn before_render(
        &mut self,
        device: &mut <Self as Renderer>::Device,
    ) -> std::result::Result<(), <Self as Renderer>::Error> {
        device.set_blend_state(&fna3d::BlendState::non_premultiplied());
        Ok(())
    }

    fn after_render(&mut self, _device: &mut <Self as Renderer>::Device) {
        //
    }

    fn set_proj_mat(&mut self, draw_data: &DrawData) {
        let mat = fna3d::mojo::orthographic_off_center(
            // left, right
            draw_data.display_pos[0],
            draw_data.display_pos[0] + draw_data.display_size[0],
            // bottom, top
            draw_data.display_pos[1] + draw_data.display_size[1],
            draw_data.display_pos[1],
            // near, far
            1.0,
            0.0,
        );

        unsafe {
            let name = "MatrixTransform";
            let name = std::ffi::CString::new(name).unwrap();
            if !fna3d::mojo::set_param(self.batch.effect_data, &name, &mat) {
                log::warn!("failed to set projection matrix in FNA3D ImGUI renderer");
            }
        }
    }

    fn set_draw_list(&mut self, draw_list: &imgui::DrawList, device: &<Self as Renderer>::Device) {
        self.batch.set_draw_list(draw_list, device);
    }

    fn draw(
        &mut self,
        device: &<Self as Renderer>::Device,
        scissors_rect: &[f32; 4],
        draw_params: &DrawCmdParams,
        n_elems: usize,
    ) -> std::result::Result<(), <Self as Renderer>::Error> {
        let DrawCmdParams {
            clip_rect,
            texture_id,
            vtx_offset,
            idx_offset,
        } = draw_params;

        let texture = if texture_id.id() == usize::MAX {
            &self.font_texture
        } else {
            self.textures
                .get(*texture_id)
                .ok_or_else(|| ImGuiRendererError::BadTexture(*texture_id))?
        };

        // FIXME:
        let scissors_rect = fna3d::Rect {
            x: f32::max(0.0, clip_rect[0]).floor() as i32,
            y: f32::max(0.0, clip_rect[1]).floor() as i32,
            w: (clip_rect[2] - clip_rect[0]).abs().ceil() as i32,
            h: (clip_rect[3] - clip_rect[1]).abs().ceil() as i32,
        };

        self.batch.prepare_draw(
            device,
            &scissors_rect,
            texture.texture.raw,
            *vtx_offset as u32,
        );

        let n_vertices = n_elems as u32 * 2 / 3; // n_verts : n_idx = 4 : 6
        let n_primitives = n_elems / 3;

        device.draw_indexed_primitives(
            fna3d::PrimitiveType::TriangleList,
            *vtx_offset as u32,
            0,
            n_vertices,
            *idx_offset as u32,
            n_primitives as u32,
            self.batch.ibuf.buf,
            fna3d::IndexElementSize::Bits16,
        );

        Ok(())
    }
}

// --------------------------------------------------------------------------------
// Batch

/// Buffer of GPU buffers
///
/// Drops internal buffers automatically.
#[derive(Debug)]
struct Batch {
    device: fna3d::Device,
    ibuf: GpuIndexBuffer,
    vbuf: GpuVertexBuffer,
    effect: *mut fna3d::Effect,
    effect_data: *mut fna3d::mojo::Effect,
}

impl Drop for Batch {
    fn drop(&mut self) {
        self.device.add_dispose_index_buffer(self.ibuf.buf);
        self.device.add_dispose_vertex_buffer(self.vbuf.buf);
        self.device.add_dispose_effect(self.effect);
    }
}

impl Batch {
    fn new(device: fna3d::Device) -> Self {
        let vbuf = GpuVertexBuffer::new(&device, 4 * N_QUADS); // four vertices per quad
        let ibuf = GpuIndexBuffer::new(&device, 6 * N_QUADS); // six indices per quad

        let (effect, effect_data) = fna3d::mojo::from_bytes(&device, SHADER).unwrap();

        Self {
            device,
            vbuf,
            ibuf,
            effect,
            effect_data,
        }
    }

    fn set_draw_list(&mut self, draw_list: &imgui::DrawList, device: &fna3d::Device) {
        self.vbuf.upload_vertices(&draw_list.vtx_buffer(), device);
        self.ibuf.upload_indices(&draw_list.idx_buffer(), device);
    }

    /// Sets up rendering pipeline before making a draw call
    fn prepare_draw(
        &mut self,
        device: &fna3d::Device,
        scissors_rect: &fna3d::Rect,
        texture: *mut fna3d::Texture,
        vtx_offset: u32,
    ) {
        device.set_scissor_rect(&scissors_rect);

        // apply effect
        let state_changes = fna3d::mojo::EffectStateChanges {
            render_state_change_count: 0,
            render_state_changes: std::ptr::null(),
            sampler_state_change_count: 0,
            sampler_state_changes: std::ptr::null(),
            vertex_sampler_state_change_count: 0,
            vertex_sampler_state_changes: std::ptr::null(),
        };
        let pass = 0;
        device.apply_effect(self.effect, pass, &state_changes);

        // set texture
        let sampler = fna3d::SamplerState::linear_wrap();
        let slot = 0;
        device.verify_sampler(slot, texture, &sampler);

        // apply vertex buffer binding
        let bind = fna3d::VertexBufferBinding {
            vertexBuffer: self.vbuf.buf,
            vertexDeclaration: VERT_DECL,
            vertexOffset: 0, // FIXME:
            instanceFrequency: 0,
        };
        device.apply_vertex_buffer_bindings(&[bind], true, vtx_offset);
    }
}

#[derive(Debug)]
struct GpuVertexBuffer {
    buf: *mut fna3d::Buffer,
    capacity_in_bytes: usize,
}

impl GpuVertexBuffer {
    fn new(device: &fna3d::Device, n_vertices: usize) -> Self {
        let len = VERT_SIZE * n_vertices;
        let buf = device.gen_vertex_buffer(true, fna3d::BufferUsage::None, len as u32);

        Self {
            buf,
            capacity_in_bytes: len,
        }
    }

    fn upload_vertices<T>(&mut self, data: &[T], device: &fna3d::Device) {
        // re-allocate if necessary
        // each index takes 20 bytes
        let len = VERT_SIZE * (data.len() + size_of::<T>()); // byte length
        if len > self.capacity_in_bytes {
            log::info!(
                "fna3d-imgui-rs: reallocate vertex buffer with byte length {}",
                len
            );
            device.add_dispose_vertex_buffer(self.buf);
            self.buf = device.gen_vertex_buffer(true, fna3d::BufferUsage::None, len as u32);
            self.capacity_in_bytes = len;
        }

        device.set_vertex_buffer_data(self.buf, 0, data, fna3d::SetDataOptions::None);
    }
}

#[derive(Debug)]
struct GpuIndexBuffer {
    buf: *mut fna3d::Buffer,
    capacity_in_bytes: usize,
}

impl GpuIndexBuffer {
    fn new(device: &fna3d::Device, n_indices: usize) -> Self {
        let len = INDEX_SIZE * n_indices;
        let buf = device.gen_index_buffer(true, fna3d::BufferUsage::None, len as u32);

        Self {
            buf,
            capacity_in_bytes: len,
        }
    }

    fn upload_indices<T>(&mut self, data: &[T], device: &fna3d::Device) {
        // reallocate if necessary
        // each index takes 2 bytes (16 bits)
        let len = INDEX_SIZE * (data.len() + size_of::<T>()); // byte length
        if len > self.capacity_in_bytes {
            log::info!(
                "fna3d-imgui-rs: re-allocating index buffer with byte length {}",
                len
            );
            device.add_dispose_index_buffer(self.buf);
            self.buf = device.gen_index_buffer(true, fna3d::BufferUsage::None, len as u32);
            self.capacity_in_bytes = len;
        }

        device.set_index_buffer_data(self.buf, 0, data, fna3d::SetDataOptions::None);
    }
}

/// Attributes of [`imgui::DrawVert`]
///
/// * pos: [f32; 2]
/// * uv: [f32; 2]
/// * col: [u8; 4]
const VERT_ELEMS: [fna3d::VertexElement; 3] = [
    fna3d::VertexElement {
        offset: 0,
        vertexElementFormat: fna3d::VertexElementFormat::Vector2 as u32,
        vertexElementUsage: fna3d::VertexElementUsage::Position as u32,
        usageIndex: 0,
    },
    fna3d::VertexElement {
        offset: 8,
        vertexElementFormat: fna3d::VertexElementFormat::Vector2 as u32,
        vertexElementUsage: fna3d::VertexElementUsage::TextureCoordinate as u32,
        usageIndex: 0,
    },
    fna3d::VertexElement {
        offset: 16,
        vertexElementFormat: fna3d::VertexElementFormat::Color as u32,
        vertexElementUsage: fna3d::VertexElementUsage::Color as u32,
        usageIndex: 0,
    },
];

const VERT_DECL: fna3d::VertexDeclaration = fna3d::VertexDeclaration {
    vertexStride: 20,
    elementCount: 3,
    elements: VERT_ELEMS.as_ptr() as *mut _,
};
