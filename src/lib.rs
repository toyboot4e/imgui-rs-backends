/*!
Backend for [`imgui-rs`]

[`imgui-rs`]: https://github.com/Gekkio/imgui-rs

[`imgui-rs`] backends are made of platform + renderer. `imgui-backend` separates and combines them
so that any combination is allowed.
*/

pub mod helper;
pub mod platform;
pub mod renderer;

use imgui::{Context, Io, Ui};

/// Half of an `imgui-rs` backend
pub trait Platform {
    type Event;
    /// Dependency for setti
    type Window;
    /// Return if the event is captured by ImGUI
    fn handle_event(
        &mut self,
        imgui: &mut Context,
        window: &Self::Window,
        event: &Self::Event,
    ) -> bool;
    /// Sets up input state
    fn prepare_frame(&mut self, io: &mut Io, window: &Self::Window);
    /// TODO: docstring
    fn prepare_render(&mut self, ui: &Ui<'_>, window: &Self::Window);
}

/// Half of an `imgui-rs` backend. See also: [`helper::RendererImplUtil`]
pub trait Renderer {
    /// Rendering context
    type Device;
    type Error;
    /// Render
    fn render(
        &mut self,
        draw_data: &imgui::DrawData,
        device: &mut Self::Device,
    ) -> std::result::Result<(), Self::Error>;
}

/// `imgui-rs` backend
pub struct Backend<P, R>
where
    P: Platform,
    R: Renderer,
{
    pub context: imgui::Context,
    pub platform: P,
    pub renderer: R,
}
