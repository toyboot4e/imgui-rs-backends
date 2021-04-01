/*!
`glow` renderer for `imgui-rs`

NOTE: OpenGL is stateful and ImGuiGlow may not always work as expected. Please open an issue or send
PR then!
*/

use {anyhow::*, glow::HasContext, imgui::im_str};

use crate::{
    helper::{DrawParams, RendererImplUtil},
    Renderer,
};

mod res;
use res::*;

mod tex;
use tex::*;

/// `mplus-1p-regular.ttf`
pub const JP_FONT: &[u8] = include_bytes!("../../assets/mplus-1p-regular.ttf");

pub const FONT_TEXTUER_ID: usize = usize::MAX;

pub struct ImGuiGlow {
    textures: imgui::Textures<Texture>,
    font_texture: Texture,
    res: Resources,
    gl_ptr: *mut glow::Context,
}

impl Drop for ImGuiGlow {
    fn drop(&mut self) {
        unsafe {
            let gl = &mut *self.gl_ptr;
            self.res.free(gl);
        }
    }
}

impl ImGuiGlow {
    pub fn new(imgui: &mut imgui::Context, gl: &glow::Context) -> Result<Self> {
        imgui.set_renderer_name(Some(im_str!(
            "imgui-glow-renderer {}",
            env!("CARGO_PKG_VERSION")
        )));

        imgui
            .io_mut()
            .backend_flags
            .insert(imgui::BackendFlags::RENDERER_HAS_VTX_OFFSET);

        let font_texture = Self::load_font_texture(gl, imgui.fonts())?;

        let mut res = Resources::new(gl)?;
        res.set_texture(font_texture.id());

        Ok(Self {
            textures: imgui::Textures::new(),
            font_texture,
            res,
            gl_ptr: gl as *const _ as *mut _,
        })
    }

    /// Create font texture with ID `FONT_TEXTURE_ID`
    fn load_font_texture(gl: &glow::Context, mut fonts: imgui::FontAtlasRefMut) -> Result<Texture> {
        let tex = {
            let atlas_texture = fonts.build_rgba32_texture();
            let (pixels, w, h) = (
                atlas_texture.data,
                atlas_texture.width,
                atlas_texture.height,
            );
            Texture::new(gl, pixels, w, h)?
        };

        // NOTE: we have to set the ID *AFTER* creating the font atlas texture
        fonts.tex_id = imgui::TextureId::from(FONT_TEXTUER_ID);

        Ok(tex)
    }

    fn lookup_texture(&self, tex_id: imgui::TextureId) -> Option<&Texture> {
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

// implement Renderer by implementing RendererImplUtil
impl Renderer for ImGuiGlow {
    type Device = glow::Context;
    type Error = String;
    fn render(
        &mut self,
        draw_data: &imgui::DrawData,
        device: &mut Self::Device,
    ) -> std::result::Result<(), Self::Error> {
        crate::helper::render(self, draw_data, device)
    }
}

impl RendererImplUtil for ImGuiGlow {
    fn before_render(
        &mut self,
        gl: &mut <Self as Renderer>::Device,
    ) -> std::result::Result<(), <Self as Renderer>::Error> {
        self.res.reset_offsets();
        unsafe {
            self.res.bind(gl);
        }
        Ok(())
    }

    fn after_render(&mut self, gl: &mut <Self as Renderer>::Device) {
        unsafe {
            Resources::unbind(gl);
        }
    }

    fn draw<'a>(
        &mut self,
        gl: &mut <Self as Renderer>::Device,
        params: &'a DrawParams,
    ) -> std::result::Result<(), <Self as Renderer>::Error> {
        // on first draw call: set states
        if params.idx_offset == 0 {
            // 1. append buffers
            self.res.append_vbuf(gl, params.vtx_buffer);
            self.res.append_ibuf(gl, params.idx_buffer);

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

            unsafe {
                self.res.set_uniforms(gl, mat);
            }
        }

        unsafe {
            // 1. scissor
            gl.scissor(
                params.scissor.left() as i32,
                params.scissor.up() as i32,
                params.scissor.right() as i32,
                params.scissor.down() as i32,
            );

            // 2. set texture
            let tex = self
                .lookup_texture(params.tex_id)
                .ok_or_else(|| format!("Bad texture id: {:?}", params.tex_id))?;
            let tex_id = tex.id();
            self.res.set_texture(tex_id);

            // 3. draw
            log::trace!("{}", params.idx_offset);
            self.res.bind(gl);
            self.res.draw(
                gl,
                // FIXME:
                params.idx_offset as i32,
                params.n_elems as i32,
            );
        }

        Ok(())
    }
}
