/*!
`rokol::gfx` integration for `imugi-rs`
*/

use {
    anyhow::*,
    imgui::{im_str, BackendFlags},
    rokol::gfx::{self as rg, BakedResource},
    thiserror::Error,
};

use crate::{
    helper::{DrawParams, DrawParamsIterator},
    Renderer,
};

/// `mplus-1p-regular.ttf`
pub const JP_FONT: &[u8] = include_bytes!("../../assets/mplus-1p-regular.ttf");

/// Number of quadliterals
pub const N_QUADS: usize = 8192;

pub const FONT_TEXTUER_ID: usize = usize::MAX;

/// Size of a vertex in bytes
pub const VERT_SIZE: usize = 20;

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
            // NOTE: ImGUI uses 16 bits index
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
    pub fn new(imgui: &mut imgui::Context) -> Result<Self, ImGuiRendererError> {
        imgui.set_renderer_name(Some(im_str!(
            "imgui-rokol-renderer {}",
            env!("CARGO_PKG_VERSION")
        )));

        imgui
            .io_mut()
            .backend_flags
            .insert(BackendFlags::RENDERER_HAS_VTX_OFFSET);

        let font_texture = Self::load_font_texture(imgui.fonts())?;
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

    /// Create font texture with ID `FONT_TEXTURE_ID`
    fn load_font_texture(
        mut fonts: imgui::FontAtlasRefMut,
    ) -> Result<Texture2d, ImGuiRendererError> {
        let tex = {
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

            Texture2d { img, w, h }
        };

        // NOTE: we have to set the ID *AFTER* creating the font atlas texture
        fonts.tex_id = imgui::TextureId::from(FONT_TEXTUER_ID);

        Ok(tex)
    }

    fn lookup_texture(&self, tex_id: imgui::TextureId) -> Option<&Texture2d> {
        if tex_id.id() == FONT_TEXTUER_ID {
            // we didn't store the font texture in `textures`
            Some(&self.font_texture)
        } else if let Some(texture) = self.textures.get(tex_id) {
            Some(texture)
        } else {
            None
        }
    }
}

impl Renderer for ImGuiRokolGfx {
    type Device = ();
    type Error = anyhow::Error;
    fn render(
        &mut self,
        draw_data: &imgui::DrawData,
        _device: &mut Self::Device,
    ) -> std::result::Result<(), Self::Error> {
        self.before_render();
        for params in DrawParamsIterator::new(draw_data) {
            self.draw(&params)?;
        }
        self.after_render();
    }
}

impl ImGuiRokolGfx {
    fn before_render(&mut self) {
        self.binds.vertex_buffer_offsets[0] = 0;
        self.binds.index_buffer_offset = 0;

        rg::begin_default_pass(&rg::PassAction::LOAD, 1280, 720);
        self.shd.apply_pip();
    }

    fn after_render(&mut self) {
        rg::end_pass();
    }

    fn draw<'a>(
        &mut self,
        params: &'a DrawParams,
    ) -> std::result::Result<(), <Self as Renderer>::Error> {
        // on first draw call: set states
        if params.idx_offset == 0 {
            // 1. append buffers
            rg::append_buffer(self.binds.vertex_buffers[0], params.vtx_buffer);
            rg::append_buffer(self.binds.index_buffer, params.idx_buffer);

            // 2. set orthographic projection matrix
            let mat = crate::helper::ortho_mat_gl(
                // left, right
                params.display.left(),
                params.display.right(),
                // bottom, top
                params.display.up(),
                params.display.down(),
                // near, far
                0.0,
                1.0,
            );

            let bytes = unsafe {
                std::slice::from_raw_parts(
                    mat.as_ptr() as *const _,
                    std::mem::size_of::<[f32; 16]>(),
                )
            };
            self.shd.set_vs_uniform(0, bytes);
        }

        // 1. scissor
        // FIXME: crash happens
        // rg::scissor_f(
        //     params.scissor.left(),
        //     params.scissor.up(),
        //     params.scissor.width(),
        //     params.scissor.height(),
        // );

        // 2. set texture
        let tex = self
            .lookup_texture(params.tex_id)
            .ok_or_else(|| anyhow!("Bad texture id: {:?}", params.tex_id))?;
        self.binds.fs_images[0] = tex.img;

        // 3. draw
        rg::apply_bindings(&self.binds);
        rg::draw(params.idx_offset as u32, params.n_elems as u32, 1);
        Ok(())
    }
}
