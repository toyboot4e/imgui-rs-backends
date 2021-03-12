/*!
Helper
*/

use imgui::{FontConfig, FontSource};

use {
    imgui::{internal::RawWrapper, DrawCmd, DrawCmdParams, DrawData},
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
        let mut icx = imgui::Context::create();

        // initial window settings
        icx.io_mut().display_size = self.display_size;

        // initial font settings
        let font_size = (self.fontsize * self.hidpi_factor) as f32;
        icx.fonts().add_font(&[
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
        icx.io_mut().font_global_scale = (1.0 / self.hidpi_factor) as f32;

        icx
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

/// Implement [`Renderer`] by implementing sub procedures
pub trait RendererImplUtil: Renderer {
    /// Use pre-multiplied alpha on immediate-mode rendering API
    fn before_render(
        &mut self,
        _device: &mut <Self as Renderer>::Device,
    ) -> std::result::Result<(), <Self as Renderer>::Error> {
        Ok(())
    }
    /// Revert the blending mode on immediate-mode rendering API
    fn after_render(&mut self, _device: &mut <Self as Renderer>::Device) {}
    fn set_proj_mat(&mut self, draw_data: &DrawData);
    fn set_draw_list(&mut self, draw_list: &imgui::DrawList, device: &<Self as Renderer>::Device);
    fn draw(
        &mut self,
        device: &<Self as Renderer>::Device,
        // clip_rect: &[f32; 4],
        draw_params: &DrawCmdParams,
        n_elems: usize,
    ) -> std::result::Result<(), <Self as Renderer>::Error>;
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
    let fb_width = draw_data.display_size[0] * draw_data.framebuffer_scale[0];
    let fb_height = draw_data.display_size[1] * draw_data.framebuffer_scale[1];

    if fb_width <= 0.0 || fb_height <= 0.0 {
        return Ok(());
    }

    renderer.set_proj_mat(&draw_data);

    let clip_off = draw_data.display_pos;
    let clip_scale = draw_data.framebuffer_scale;

    for draw_list in draw_data.draw_lists() {
        renderer.set_draw_list(draw_list, device);

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

                    // FIXME: this clipping is not always correct?
                    if clip_rect[0] >= fb_width
                        || clip_rect[1] >= fb_height
                        || clip_rect[2] < 0.0
                        || clip_rect[3] < 0.0
                    {
                        // skip
                    } else {
                        // renderer.draw(device, &clip_rect, &cmd_params, count)?;
                        renderer.draw(device, &cmd_params, count)?;
                    }
                }
                DrawCmd::ResetRenderState => {
                    log::warn!("imgui-backends fna3d: ResetRenderState not implemented");
                }
                DrawCmd::RawCallback { callback, raw_cmd } => unsafe {
                    callback(draw_list.raw(), raw_cmd)
                },
            }
        }
    }

    Ok(())
}
