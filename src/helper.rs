/*!
Helper

NOTE: Take care of the coordinate system: `Rect` thinks y axis goes up!
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
///
/// ```
/// // left, right, bottom, top, near, far
/// let mat = ortho_mat_gl(0.0, 1280.0, 0.0, 720.0, 0.0, 1.0);
/// ```
///
/// Note that they're in OpenGL coordinate system and the y axis goes up. Swap `bottom` and `top` if
/// you want your y axis to go down:
///
/// ```
/// // left, right, top, bottom, near, far
/// let mat = ortho_mat_gl(0.0, 1280.0, 720.0, 0.0, 0.0, 1.0);
/// ```
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
        -(2.0 / (far as f64 - near as f64)) as f32,
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

/// Rectangle. NOTE: Y axis goes up
///
/// # Coordinate system
/// ```md
///    (up)
///     y
///     ^
///     |
/// ----+---> x (right)
///     |
///     |
/// ```
#[derive(Debug, Clone)]
pub struct Rect {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
}

impl Rect {
    pub fn left(&self) -> f32 {
        self.left
    }

    /// NOTE: Y axies goes up
    pub fn top(&self) -> f32 {
        self.top
    }

    pub fn right(&self) -> f32 {
        self.right
    }

    /// NOTE: Y axies goes up
    pub fn bottom(&self) -> f32 {
        self.bottom
    }

    pub fn width(&self) -> f32 {
        // FIXME:
        (self.right - self.left).abs()
    }

    pub fn height(&self) -> f32 {
        // FIXME: somehow the sign changes every frame
        (self.top - self.bottom).abs()
    }
}

/// Context and parameters for making a draw call; more comfortable version of
/// [`imgui::DrawCmdParams`]
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

/// Iterator of [`DrawParams`]
pub struct DrawParamsIterator<'a> {
    // variables
    fb_width: f32,
    fb_height: f32,
    clip_off: [f32; 2],
    clip_scale: [f32; 2],
    display_rect: Rect,
    // iterators
    draw_lists: imgui::DrawListIterator<'a>,
    draw_list: Option<&'a imgui::DrawList>,
    draw_commands: Option<imgui::DrawCmdIterator<'a>>,
}

impl<'a> DrawParamsIterator<'a> {
    pub fn new(data: &'a imgui::DrawData) -> Self {
        // framebuffer size
        let fb_width = data.display_size[0] * data.framebuffer_scale[0];
        let fb_height = data.display_size[1] * data.framebuffer_scale[1];

        let clip_off = data.display_pos;
        let clip_scale = data.framebuffer_scale;

        let display_rect = Rect {
            left: data.display_pos[0],
            right: data.display_pos[0] + data.display_size[0],
            top: data.display_pos[1] + data.display_size[1],
            bottom: data.display_pos[1],
        };

        let mut draw_lists = data.draw_lists();

        if fb_width <= 0.0 || fb_height <= 0.0 {
            return Self {
                fb_width,
                fb_height,
                clip_off,
                clip_scale,
                display_rect,
                draw_lists,
                draw_list: None,
                draw_commands: None,
            };
        }

        let (draw_list, draw_commands) = match draw_lists.next() {
            Some(list) => (Some(list), Some(list.commands())),
            None => (None, None),
        };

        Self {
            fb_width,
            fb_height,
            clip_off,
            clip_scale,
            display_rect,
            draw_lists,
            draw_list,
            draw_commands,
        }
    }
}

impl<'a> Iterator for DrawParamsIterator<'a> {
    type Item = DrawParams<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let clip_off = self.clip_off;
        let clip_scale = self.clip_scale;
        let fb_width = self.fb_width;
        let fb_height = self.fb_height;
        let display_rect = self.display_rect.clone();

        // One step of this loop:
        // for draw_list in draw_data.draw_lists() {
        //     for cmd in draw_list.commands() {
        'next: loop {
            let cmd = loop {
                match self.draw_commands.as_mut()?.next() {
                    Some(cmd) => break cmd,
                    None => {
                        // go to next list
                        self.draw_list = self.draw_lists.next();
                        self.draw_commands = Some(self.draw_list?.commands());
                        continue;
                    }
                }
            };

            break match cmd {
                DrawCmd::Elements { count, cmd_params } => {
                    {
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
                            continue 'next;
                        }
                    }

                    // [left, up, right, bottom]
                    let [x, y, z, w] = cmd_params.clip_rect;
                    let scissor = Rect {
                        left: x * clip_scale[0],
                        top: fb_height - w * clip_scale[1],
                        right: (z - x) * clip_scale[0],
                        bottom: (w - y) * clip_scale[1],
                    };

                    Some(DrawParams {
                        display: display_rect.clone(),
                        vtx_buffer: self.draw_list?.vtx_buffer(),
                        vtx_offset: cmd_params.vtx_offset,
                        idx_buffer: self.draw_list?.idx_buffer(),
                        idx_offset: cmd_params.idx_offset,
                        n_elems: count,
                        tex_id: cmd_params.texture_id,
                        scissor,
                    })
                }
                DrawCmd::ResetRenderState => {
                    log::warn!("imgui-backends: ResetRenderState is not implemented");
                    None
                }
                DrawCmd::RawCallback { callback, raw_cmd } => unsafe {
                    log::warn!("imgui-backends: RawCallback is not implemented");
                    callback(self.draw_list?.raw(), raw_cmd);
                    None
                },
            };
        }
    }
}
