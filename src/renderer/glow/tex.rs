//! Texture

use anyhow::*;
use glow::HasContext;

unsafe fn gen_texture(gl: &glow::Context, pixels: &[u8], w: u32, h: u32) -> Result<glow::Texture> {
    let tex = gl.create_texture().map_err(Error::msg)?;

    gl.bind_texture(glow::TEXTURE_2D, Some(tex));

    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);
    gl.tex_parameter_i32(
        glow::TEXTURE_2D,
        glow::TEXTURE_MIN_FILTER,
        glow::LINEAR as i32,
    );
    gl.tex_parameter_i32(
        glow::TEXTURE_2D,
        glow::TEXTURE_MAG_FILTER,
        glow::LINEAR as i32,
    );

    gl.tex_image_2d(
        glow::TEXTURE_2D,
        0,                 // level
        glow::RGBA as i32, // internal format
        w as i32,
        h as i32,
        0,          // border
        glow::RGBA, // format
        glow::UNSIGNED_BYTE,
        Some(pixels),
    );
    gl.generate_mipmap(glow::TEXTURE_2D);

    gl.bind_texture(glow::TEXTURE_2D, None);

    Ok(tex)
}

#[derive(Debug, Clone)]
pub struct TextureDrop {
    gl: *mut glow::Context,
    id: glow::Texture,
}

impl Drop for TextureDrop {
    fn drop(&mut self) {
        unsafe {
            let gl = &mut *self.gl;
            gl.delete_texture(self.id);
        }
    }
}

impl TextureDrop {
    pub fn new(gl: &glow::Context, pixels: &[u8], w: u32, h: u32) -> Result<Self> {
        let tex = unsafe { self::gen_texture(gl, pixels, w, h)? };

        Ok(Self {
            gl: gl as *const _ as *mut _,
            id: tex,
        })
    }

    pub fn id(&self) -> glow::Texture {
        self.id
    }
}

#[derive(Debug, Clone)]
pub struct Texture {
    own: TextureDrop,
}

impl Texture {
    pub fn new(gl: &glow::Context, pixels: &[u8], w: u32, h: u32) -> Result<Self> {
        let own = TextureDrop::new(gl, pixels, w, h)?;
        Ok(Self { own })
    }

    pub fn id(&self) -> glow::Texture {
        self.own.id
    }
}
