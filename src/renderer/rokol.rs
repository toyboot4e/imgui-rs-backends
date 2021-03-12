/*!
`rokol::gfx` integration for `imugi-rs`
*/

use {
    anyhow::{Context, Error},
    imgui::{im_str, BackendFlags, DrawCmdParams, DrawData},
    rokol::gfx::{self as rg, BakedResource},
    thiserror::Error,
};

use crate::{helper::RendererImplUtil, Renderer};

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

/// RAII
#[derive(Debug)]
pub struct Texture2d {
    pub img: rg::Image,
    pub w: u32,
    pub h: u32,
}

impl Drop for Texture2d {
    fn drop(&mut self) {
        rg::Image::destroy(self.img);
    }
}

/// RAII
#[derive(Debug)]
pub struct Shader {
    shd: rg::Shader,
    pip: rg::Pipeline,
}

impl std::ops::Drop for Shader {
    fn drop(&mut self) {
        rg::Shader::destroy(self.shd);
        rg::Pipeline::destroy(self.pip);
    }
}

impl Shader {
    pub fn new(shd: rg::Shader, pip: rg::Pipeline) -> Self {
        Self { shd, pip }
    }

    pub fn set_vs_uniform(&self, ix: usize, bytes: &[u8]) {
        rg::apply_uniforms(rg::ShaderStage::Vs, ix as u32, bytes);
    }

    pub fn set_fs_uniform(&self, ix: usize, bytes: &[u8]) {
        rg::apply_uniforms(rg::ShaderStage::Fs, ix as u32, bytes);
    }

    pub fn apply_pip(&self) {
        rg::apply_pipeline(self.pip);
    }
}

/// Memory layout of [`imgui::DrawVert`]
///
/// pub struct DrawVert {
///     pub pos: [f32; 2],
///     pub uv: [f32; 2],
///     pub col: [u8; 4],
/// }
fn layout() -> rg::LayoutDesc {
    let mut desc = rg::LayoutDesc::default();
    desc.attrs[0].format = rg::VertexFormat::Float2 as u32;
    desc.attrs[1].format = rg::VertexFormat::Float2 as u32;
    desc.attrs[2].format = rg::VertexFormat::UByte4N as u32;
    desc
}

/// Sets image type
macro_rules! img_type {
    ($name:expr,$ty:expr) => {
        rg::ShaderImageDesc {
            name: concat!($name, "\0").as_ptr() as *const _,
            image_type: $ty as u32,
            ..Default::default()
        }
    };
}

/// Single-value uniform block
macro_rules! ub {
    ($name:expr, $uniform_ty:expr, $size_ty:ty) => {{
        let mut block = rg::ShaderUniformBlockDesc::default();

        block.uniforms[0] = rg::ShaderUniformDesc {
            name: concat!($name, "\0").as_ptr() as *const _,
            type_: $uniform_ty as u32,
            ..Default::default()
        };
        block.size += std::mem::size_of::<$size_ty>() as u64;

        block
    }};
}

const ALPHA_BLEND: rg::BlendState = rg::BlendState {
    enabled: true,
    src_factor_rgb: rg::BlendFactor::SrcAlpha as u32,
    dst_factor_rgb: rg::BlendFactor::OneMinusSrcAlpha as u32,
    op_rgb: 0,
    src_factor_alpha: rg::BlendFactor::One as u32,
    dst_factor_alpha: rg::BlendFactor::Zero as u32,
    op_alpha: 0,
};

const VS: &'static str = concat!(include_str!("rokol/texture.vs"), '\0');
const FS: &'static str = concat!(include_str!("rokol/texture.fs"), '\0');

fn create_shader() -> Shader {
    log::trace!("creating imgui-rokol-gfx shader...");

    // // FIXME: why include_str! not working
    // let mut vs = std::fs::read_to_string("src/renderer/rokol/texture.vs").unwrap();
    // vs.push('\0');
    // let mut fs = std::fs::read_to_string("src/renderer/rokol/texture.fs").unwrap();
    // fs.push('\0');

    let shd = rg::Shader::create(&{
        let mut desc = unsafe { rokol::gfx::shader_desc(VS, FS) };
        // let mut desc = unsafe { rokol::gfx::shader_desc(&vs, &fs) };
        desc.fs.images[0] = img_type!("tex", rg::ImageType::Dim2);
        desc.vs.uniform_blocks[0] = ub!("transform", rg::UniformType::Mat4, [f32; 16]);
        desc
    });

    log::trace!("creating imgui-rokol-gfx pipeline...");
    let pip = rg::Pipeline::create(&{
        let mut desc = rg::PipelineDesc {
            shader: shd,
            index_type: rg::IndexType::UInt16 as u32,
            layout: self::layout(),
            cull_mode: rg::CullMode::None as u32,
            ..Default::default()
        };
        desc.colors[0].blend = ALPHA_BLEND;
        desc
    });

    Shader::new(shd, pip)
}

fn create_bindings() -> rg::Bindings {
    rg::Bindings {
        vertex_buffers: {
            let mut xs = [Default::default(); 8];
            xs[0] = rg::Buffer::create(&rg::vbuf_desc_dyn(
                VERT_SIZE * N_QUADS * 4,
                rg::ResourceUsage::Stream,
                "",
            ));
            xs
        },
        index_buffer: rg::Buffer::create(&rg::ibuf_desc_dyn(
            2 * N_QUADS * 6,
            rg::ResourceUsage::Stream,
            "",
        )),
        ..Default::default()
    }
}

#[derive(Debug)]
pub struct ImGuiRokolGfx {
    textures: imgui::Textures<Texture2d>,
    font_texture: Texture2d,
    shd: Shader,
    binds: rg::Bindings,
}

impl ImGuiRokolGfx {
    pub fn new(icx: &mut imgui::Context) -> Result<Self, ImGuiRendererError> {
        icx.set_renderer_name(Some(im_str!(
            "imgui-rokol-renderer {}",
            env!("CARGO_PKG_VERSION")
        )));

        icx.io_mut()
            .backend_flags
            .insert(BackendFlags::RENDERER_HAS_VTX_OFFSET);

        let font_texture = Self::load_font_texture(icx.fonts())?;
        let shd = self::create_shader();
        let mut binds = self::create_bindings();
        binds.fs_images[0] = font_texture.img;

        Ok(Self {
            textures: imgui::Textures::new(),
            font_texture,
            shd,
            binds,
        })
    }

    fn load_font_texture(
        mut fonts: imgui::FontAtlasRefMut,
    ) -> Result<Texture2d, ImGuiRendererError> {
        let atlas_texture = fonts.build_rgba32_texture();
        let (pixels, w, h) = (
            atlas_texture.data,
            atlas_texture.width,
            atlas_texture.height,
        );

        let img = rg::Image::create(&{
            let mut desc = rg::ImageDesc {
                type_: rg::ImageType::Dim2 as u32,
                // FIXME: Is immutable OK?
                usage: rg::ResourceUsage::Immutable as u32,
                width: w as i32,
                height: h as i32,
                ..Default::default()
            };
            desc.data.subimage[0][0] = pixels.as_ref().into();
            desc
        });

        Ok(Texture2d { img, w, h })
    }
}

impl Renderer for ImGuiRokolGfx {
    type Device = ();
    type Error = anyhow::Error;
    fn render(
        &mut self,
        draw_data: &imgui::DrawData,
        device: &mut Self::Device,
    ) -> std::result::Result<(), Self::Error> {
        crate::helper::render(self, draw_data, device)
    }
}

impl RendererImplUtil for ImGuiRokolGfx {
    fn before_render(
        &mut self,
        _device: &mut <Self as Renderer>::Device,
    ) -> std::result::Result<(), <Self as Renderer>::Error> {
        self.binds.vertex_buffer_offsets[0] = 0;
        self.binds.index_buffer_offset = 0;
        rg::begin_default_pass(&rg::PassAction::LOAD, 1280, 720);
        self.shd.apply_pip();
        Ok(())
    }

    fn after_render(&mut self, _device: &mut <Self as Renderer>::Device) {
        rg::end_pass();
    }

    fn set_proj_mat(&mut self, draw_data: &DrawData) {
        let mat = crate::helper::ortho_mat_gl(
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

        let bytes = unsafe {
            std::slice::from_raw_parts(mat.as_ptr() as *const _, std::mem::size_of::<[f32; 16]>())
        };
        self.shd.set_vs_uniform(0, bytes);
    }

    fn set_draw_list(&mut self, draw_list: &imgui::DrawList, _device: &<Self as Renderer>::Device) {
        // upload all vertices at once
        unsafe {
            rg::update_buffer(self.binds.vertex_buffers[0], draw_list.vtx_buffer());
            rg::update_buffer(self.binds.index_buffer, draw_list.idx_buffer());
        }
    }

    fn draw(
        &mut self,
        _device: &<Self as Renderer>::Device,
        params: &DrawCmdParams,
        n_elems: usize,
    ) -> std::result::Result<(), <Self as Renderer>::Error> {
        self.binds.vertex_buffer_offsets[0] = params.vtx_offset as i32;
        // self.binds.index_buffer_offset = params.idx_offset as i32;

        rg::apply_bindings(&self.binds);

        // rg::draw(draw_params.vtx_offset as u32, n_elems as u32, 1);
        rg::draw(0, n_elems as u32, 1);

        Ok(())
    }
}
