/*!
Backend for [`imgui-rs`]

[`imgui-rs`]: https://github.com/Gekkio/imgui-rs

[`imgui-rs`] backends are made of platform and renderer. `imgui-backend` separates and combines them
so that any combination is allowed.

See [`examples`] to get started.

[`examples`]: https://github.com/toyboot4e/imgui-rs-backends
*/

pub mod helper;
pub mod platform;
pub mod renderer;

use imgui::{Context, Io, Ui};
use std::ops::{Deref, DerefMut};

/// Half of an `imgui-rs` backend
pub trait Platform {
    type Event;
    /// Dependency
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

/// `imgui-rs` backend = `imgui::Context` + `Platform` + `Renderer`
pub struct Backend<P, R>
where
    P: Platform,
    R: Renderer,
{
    pub imgui: imgui::Context,
    pub platform: P,
    pub renderer: R,
}

impl<P, R> Backend<P, R>
where
    P: Platform,
    R: Renderer,
{
    pub fn handle_event(&mut self, window: &P::Window, event: &P::Event) {
        self.platform.handle_event(&mut self.imgui, window, event);
    }

    /// TODO: set dt?
    pub fn begin_frame<'a>(&'a mut self, window: &P::Window) -> BackendUi<'a, P, R> {
        self.platform.prepare_frame(self.imgui.io_mut(), window);
        BackendUi {
            ui: self.imgui.frame(),
            platform: &mut self.platform,
            renderer: &mut self.renderer,
        }
    }
}

pub struct BackendUi<'a, P, R>
where
    P: Platform,
    R: Renderer,
{
    ui: imgui::Ui<'a>,
    platform: &'a mut P,
    renderer: &'a mut R,
}

impl<'a, P, R> Deref for BackendUi<'a, P, R>
where
    P: Platform,
    R: Renderer,
{
    type Target = imgui::Ui<'a>;
    fn deref(&self) -> &Self::Target {
        &self.ui
    }
}

impl<'a, P, R> DerefMut for BackendUi<'a, P, R>
where
    P: Platform,
    R: Renderer,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ui
    }
}

impl<'a, P, R> AsRef<imgui::Ui<'a>> for BackendUi<'a, P, R>
where
    P: Platform,
    R: Renderer,
{
    fn as_ref(&self) -> &imgui::Ui<'a> {
        &self.ui
    }
}

impl<'a, P, R> AsMut<imgui::Ui<'a>> for BackendUi<'a, P, R>
where
    P: Platform,
    R: Renderer,
{
    fn as_mut(&mut self) -> &mut imgui::Ui<'a> {
        &mut self.ui
    }
}

impl<'a, P, R> BackendUi<'a, P, R>
where
    P: Platform,
    R: Renderer,
{
    pub fn end_frame(self, window: &mut P::Window, device: &mut R::Device) -> Result<(), R::Error> {
        self.platform.prepare_render(&self.ui, window);
        self.renderer.render(self.ui.render(), device)?;
        Ok(())
    }
}
