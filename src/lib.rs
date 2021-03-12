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
    ///
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

impl<P, R> Backend<P, R>
where
    P: Platform,
    R: Renderer,
{
    pub fn io_mut(&mut self) -> &mut imgui::Io {
        self.context.io_mut()
    }

    pub fn handle_event(&mut self, window: &P::Window, ev: &P::Event) -> bool {
        self.platform.handle_event(&mut self.context, window, ev)
    }

    pub fn frame<'ui, 'w, 'd>(
        &'ui mut self,
        window: &'w P::Window,
        device: &'d mut R::Device,
    ) -> BackendUi<'ui, 'w, 'd, P, R> {
        self.platform.prepare_frame(self.context.io_mut(), window);
        let ui = self.context.frame();
        BackendUi {
            ui,
            platform: &mut self.platform,
            renderer: &mut self.renderer,
            window,
            device,
        }
    }
}

/// Wrapped [`Ui`](imgui::Ui) that can be rendered with backend
pub struct BackendUi<'ui, 'w, 'd, P, R>
where
    P: Platform,
    R: Renderer,
{
    pub ui: imgui::Ui<'ui>,
    pub platform: &'ui mut P,
    pub renderer: &'ui mut R,
    pub window: &'w P::Window,
    pub device: &'d mut R::Device,
}

impl<'ui, 'w, 'd, P, R> BackendUi<'ui, 'w, 'd, P, R>
where
    P: Platform,
    R: Renderer,
{
    /// Be sure to call this method to render
    pub fn render_with_backend(self) -> std::result::Result<(), R::Error> {
        self.platform.prepare_render(&self.ui, self.window);
        self.renderer.render(self.ui.render(), self.device)
    }
}

impl<'ui, 'w, 'd, P, R> std::ops::Deref for BackendUi<'ui, 'w, 'd, P, R>
where
    P: Platform,
    R: Renderer,
{
    type Target = imgui::Ui<'ui>;
    fn deref(&self) -> &Self::Target {
        &self.ui
    }
}

impl<'ui, 'w, 'd, P, R> std::ops::DerefMut for BackendUi<'ui, 'w, 'd, P, R>
where
    P: Platform,
    R: Renderer,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ui
    }
}
