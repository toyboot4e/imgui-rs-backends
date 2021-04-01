/*!
Helper

TODO: Replace it with an iterator (DrawCalls::pull)
*/

use imgui::{FontConfig, FontSource};

use {
    imgui::{internal::RawWrapper, DrawCmd},
    thiserror::Error,
};

use crate::Renderer;

/// `mplus-1p-regular.ttf`
pub const JP_FONT: &[u8] = include_bytes!("../assets/mplus-1p-regular.ttf");

/// Named parameters for easily creating ImGUI context with fonts
#[derive(Debug, Clone, PartialEq)]
pub struct QuickStart {
    pub display_size: [f32; 2],
    pub fontsize: f32,
    pub hidpi_factor: f32,
}

impl QuickStart {
    /// Based on: <https://github.com/Gekkio/imgui-rs/blob/master/imgui-examples/examples/support/mod.rs>
    pub fn create_context(&self) -> imgui::Context {
        // ImGUI context
        let mut imgui = imgui::Context::create();

        // initial window settings
        imgui.io_mut().display_size = self.display_size;

        // initial font settings
        let font_size = (self.fontsize * self.hidpi_factor) as f32;
        imgui.fonts().add_font(&[
            FontSource::DefaultFontData {
                config: Some(FontConfig {
                    size_pixels: font_size,
                    ..FontConfig::default()
                }),
            },
            FontSource::TtfData {
                data: JP_FONT,
                size_pixels: font_size,
                config: Some(FontConfig {
                    rasterizer_multiply: 1.75,
                    glyph_ranges: imgui::FontGlyphRanges::japanese(),
                    ..FontConfig::default()
                }),
            },
        ]);
        imgui.io_mut().font_global_scale = (1.0 / self.hidpi_factor) as f32;

        imgui
    }
}

/// Creates an orthographic projection matrix for OpenGL
pub fn ortho_mat_gl(
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    near: f32,
    far: f32,
) -> [f32; 16] {
    [
        (2.0 / (right as f64 - left as f64)) as f32,
        0.0,
        0.0,
        0.0,
        // ---
        0.0,
        (2.0 / (top as f64 - bottom as f64)) as f32,
        0.0,
        0.0,
        // ---
        0.0,
        0.0,
        -(1.0 / (far as f64 - near as f64)) as f32,
        0.0,
        // ---
        -((right as f64 + left as f64) / (right as f64 - left as f64)) as f32,
        -((top as f64 + bottom as f64) / (top as f64 - bottom as f64)) as f32,
        (near as f64 / (near as f64 - far as f64)) as f32,
        1.0,
    ]
}

/// TODO: extend and use this error
#[derive(Debug, Error)]
pub enum ImGuiRendererError {
    #[error("bad texture id")]
    BadTexture(imgui::TextureId),
}

/// Rectangle
///
/// # Coordinate system
/// ```md
///     y
///     ^
///     |
/// ----+---> x
///     |
///     |
/// ```
#[derive(Debug, Clone)]
pub struct Rect {
    left: f32,
    up: f32,
    right: f32,
    down: f32,
}

impl Rect {
    pub fn left(&self) -> f32 {
        self.left
    }

    pub fn up(&self) -> f32 {
        self.up
    }

    pub fn right(&self) -> f32 {
        self.right
    }

    pub fn down(&self) -> f32 {
        self.down
    }

    pub fn width(&self) -> f32 {
        self.right - self.left
    }

    pub fn height(&self) -> f32 {
        self.down - self.up
    }
}

/// Context and parameters for a draw call; extentended [`imgui::DrawCmdParams`]
#[derive(Debug, Clone)]
pub struct DrawParams<'a> {
    /// Display [`Rect`]. Can be used for calculating orthographic projection matrix
    pub display: Rect,
    /// Vertex buffer for multiple draw calls, sliced with `vtx_offset` and `n_elems`
    pub vtx_buffer: &'a [imgui::DrawVert],
    /// Vertex offset for this draw call
    pub vtx_offset: usize,
    /// Index buffer for multiple draw calls, sliced with `vtx_offset` and `n_elems`
    pub idx_buffer: &'a [imgui::DrawIdx],
    /// Index offset for this draw call
    pub idx_offset: usize,
    /// Number of triangles for this draw call: `n_elems` = `vbuf_span.len` `4` = `ibuf.len` / `6`
    pub n_elems: usize,
    /// Texture ID
    pub tex_id: imgui::TextureId,
    /// Scissor rectangle
    pub scissor: Rect,
}

/// Implement [`Renderer`] by implementing sub procedures
pub trait RendererImplUtil: Renderer {
    /// Use pre-multiplied alpha on immediate-mode rendering API
    fn before_render(
        &mut self,
        _device: &mut <Self as Renderer>::Device,
    ) -> std::result::Result<(), <Self as Renderer>::Error> {
        Ok(())
    }
    /// Issue a draw call
    fn draw<'a>(
        &mut self,
        device: &mut <Self as Renderer>::Device,
        params: &'a DrawParams,
    ) -> std::result::Result<(), <Self as Renderer>::Error>;
    /// Revert the blending mode on immediate-mode rendering API
    fn after_render(&mut self, _device: &mut <Self as Renderer>::Device) {}
}

pub fn render<T: RendererImplUtil>(
    renderer: &mut T,
    draw_data: &imgui::DrawData,
    device: &mut <T as Renderer>::Device,
) -> std::result::Result<(), <T as Renderer>::Error> {
    renderer.before_render(device)?;
    let res = self::render_impl(renderer, draw_data, device);
    renderer.after_render(device);
    res
}

fn render_impl<T: RendererImplUtil>(
    renderer: &mut T,
    draw_data: &imgui::DrawData,
    device: &mut <T as Renderer>::Device,
) -> std::result::Result<(), <T as Renderer>::Error> {
    // framebuffer size
    let fb_width = draw_data.display_size[0] * draw_data.framebuffer_scale[0];
    let fb_height = draw_data.display_size[1] * draw_data.framebuffer_scale[1];

    if fb_width <= 0.0 || fb_height <= 0.0 {
        return Ok(());
    }

    let clip_off = draw_data.display_pos;
    let clip_scale = draw_data.framebuffer_scale;

    let display_rect = Rect {
        left: draw_data.display_pos[0],
        right: draw_data.display_pos[0] + draw_data.display_size[0],
        up: draw_data.display_pos[1] + draw_data.display_size[1],
        down: draw_data.display_pos[1],
    };

    for draw_list in draw_data.draw_lists() {
        for cmd in draw_list.commands() {
            match cmd {
                DrawCmd::Elements { count, cmd_params } => {
                    let clip_rect = &cmd_params.clip_rect;

                    // [left, up, right, down]
                    let clip_rect = [
                        (clip_rect[0] - clip_off[0]) * clip_scale[0],
                        (clip_rect[1] - clip_off[1]) * clip_scale[1],
                        (clip_rect[2] - clip_off[0]) * clip_scale[0],
                        (clip_rect[3] - clip_off[1]) * clip_scale[1],
                    ];

                    if clip_rect[0] >= fb_width
                        || clip_rect[1] >= fb_height
                        || clip_rect[2] < 0.0
                        || clip_rect[3] < 0.0
                    {
                        continue;
                    }

                    // [left, up, right, bottom]
                    let [x, y, z, w] = cmd_params.clip_rect;
                    let scissor = Rect {
                        left: x * clip_scale[0],
                        up: fb_height - w * clip_scale[1],
                        right: (z - x) * clip_scale[0],
                        down: (w - y) * clip_scale[1],
                    };

                    let params = DrawParams {
                        display: display_rect.clone(),
                        vtx_buffer: draw_list.vtx_buffer(),
                        vtx_offset: cmd_params.vtx_offset,
                        idx_buffer: draw_list.idx_buffer(),
                        idx_offset: cmd_params.idx_offset,
                        n_elems: count,
                        tex_id: cmd_params.texture_id,
                        scissor,
                    };

                    renderer.draw(device, &params)?;
                }
                DrawCmd::ResetRenderState => {
                    log::warn!("imgui-backends: ResetRenderState is not implemented");
                }
                DrawCmd::RawCallback { callback, raw_cmd } => unsafe {
                    log::warn!("imgui-backends: RawCallback is not implemented");
                    callback(draw_list.raw(), raw_cmd)
                },
            }
        }
    }

    Ok(())
}
