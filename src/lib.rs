/*!
Framework for providing various backends for [`imgui-rs`]. See [`examples`] to get started.

[`imgui-rs`]: https://github.com/Gekkio/imgui-rs
[`examples`]: https://github.com/toyboot4e/imgui-rs-backends

# Example

Backend creation:

```no_run
use imgui_backends::{helper::QuickStart, platform::ImGuiSdl2, renderer::ImGuiGlow};
pub type Backend = imgui_backends::Backend<ImGuiSdl2, ImGuiGlow>;

let mut backend = {
    let mut imgui = QuickStart { /* omitted */ }
        .create_context();

    let platform = ImGuiSdl2::new(&mut imgui, &window);
    let renderer = ImGuiGlow::new(&mut imgui, &glow)?;

    Backend {
        imgui,
        platform,
        renderer,
    }
};
```

Backend usage:

```no_run
backend.update_delta_time(dt);

let ui = backend.begin_frame(&window);

// use imgui here

ui.end_frame(&mut window, &mut glow)
    .map_err(Error::msg)?;
```
*/

pub extern crate imgui;

pub mod helper;
pub mod platform;
pub mod renderer;

use imgui::{Context, Io, Ui};
use std::{
    ops::{Deref, DerefMut},
    time::Duration,
};

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
    fn prepare_render(&mut self, ui: &Ui<'_>, window: &Self::Window);
}

/// Half of an `imgui-rs` backend
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
#[derive(Debug)]
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

    pub fn update_delta_time(&mut self, dt: Duration) {
        self.imgui.io_mut().update_delta_time(dt);
    }

    /// TODO: begin frame with backbuffer size
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
