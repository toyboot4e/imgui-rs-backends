/*!
Helper
*/

use imgui::{FontConfig, FontSource};

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
