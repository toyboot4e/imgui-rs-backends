/*!
Backend for [`imgui-rs`]

[`imgui-rs`]: https://github.com/Gekkio/imgui-rs

[`imgui-rs`] backends are made of input handler + renderer. `imgui-backend` separates them and allows
any combination of them.
*/

pub mod helper;
pub mod input_handler;
pub mod renderer;

use imgui::{Context, Io, Ui};

/// Half of an `imgui-rs` backend
pub trait InputHandler {
    type Event;
    /// Dependency for setti
    type Window;
    /// Return if the event is captured by ImGUI
    fn handle_event(&mut self, imgui: &mut Context, event: &Self::Event) -> bool;
    /// Sets up input state
    fn prepare_frame(&mut self, io: &mut Io, window: &Self::Window);
    ///
    fn prepare_render(&mut self, ui: &Ui<'_>, window: &Self::Window);
}

/// Half of an `imgui-rs` backend
pub trait Renderer {
    /// Rendering context
    type Device;
    /// Return type
    type Result;
    /// Render
    fn render(&mut self, draw_data: &imgui::DrawData, device: &mut Self::Device) -> Self::Result;
}

/// `imgui-rs` backend made of [`InputHandler`] and [`Renderer`]
pub struct Backend<I, R>
where
    I: InputHandler,
    R: Renderer,
{
    pub context: imgui::Context,
    pub input_handler: I,
    pub renderer: R,
}

impl<I, R> Backend<I, R>
where
    I: InputHandler,
    R: Renderer,
{
    pub fn io_mut(&mut self) -> &mut imgui::Io {
        self.context.io_mut()
    }

    pub fn handle_event(&mut self, ev: &I::Event) -> bool {
        self.input_handler.handle_event(&mut self.context, ev)
    }

    pub fn frame<'ui, 'w, 'd>(
        &'ui mut self,
        window: &'w I::Window,
        device: &'d mut R::Device,
    ) -> BackendUi<'ui, 'w, 'd, I, R> {
        self.input_handler
            .prepare_frame(self.context.io_mut(), window);
        let ui = self.context.frame();
        BackendUi {
            ui,
            input_handler: &mut self.input_handler,
            renderer: &mut self.renderer,
            window,
            device,
        }
    }

    // pub fn frame(&mut self, window: &I::Window) -> imgui::Ui {
    //     self.input_handler
    //         .prepare_frame(self.context.io_mut(), window);
    //     self.context.frame()
    // }

    // pub fn render(
    //     &mut self,
    //     ui: imgui::Ui,
    //     window: &I::Window,
    //     device: &mut R::Device,
    // ) -> R::Result {
    //     self.input_handler.prepare_render(&ui, window);
    //     self.renderer.render(ui.render(), device)
    // }
}

pub struct BackendUi<'ui, 'w, 'd, I, R>
where
    I: InputHandler,
    R: Renderer,
{
    pub ui: imgui::Ui<'ui>,
    pub input_handler: &'ui mut I,
    pub renderer: &'ui mut R,
    pub window: &'w I::Window,
    pub device: &'d mut R::Device,
}

impl<'ui, 'w, 'd, I, R> BackendUi<'ui, 'w, 'd, I, R>
where
    I: InputHandler,
    R: Renderer,
{
    /// Be sure to call this method to render
    pub fn render_with_backend(self) -> R::Result {
        self.input_handler.prepare_render(&self.ui, self.window);
        self.renderer.render(self.ui.render(), self.device)
    }
}

impl<'ui, 'w, 'd, I, R> std::ops::Deref for BackendUi<'ui, 'w, 'd, I, R>
where
    I: InputHandler,
    R: Renderer,
{
    type Target = imgui::Ui<'ui>;
    fn deref(&self) -> &Self::Target {
        &self.ui
    }
}

impl<'ui, 'w, 'd, I, R> std::ops::DerefMut for BackendUi<'ui, 'w, 'd, I, R>
where
    I: InputHandler,
    R: Renderer,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ui
    }
}
