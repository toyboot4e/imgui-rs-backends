/*!
Rust-SDL2 platform integration for `imgui-rs`

This is very much based on [rust-imgui-sdl2]. Many thanks to them!

[rust-imgui-sdl2]: https://github.com/michaelfairley/rust-imgui-sdl2
*/

use {
    imgui::{ConfigFlags, Context, Key, MouseCursor},
    sdl2::{
        event::Event,
        keyboard::Scancode,
        mouse::{Cursor, SystemCursor},
        video::Window,
    },
};

use crate::Platform;

/// SDL2 platform for `imgui-rs`
pub struct ImGuiSdl2 {
    mouse_press: [bool; 5],
    ignore_mouse: bool,
    ignore_keyboard: bool,
    cursor: Option<MouseCursor>,
    sdl_cursor: Option<Cursor>,
}

struct Sdl2ClipboardBackend(sdl2::clipboard::ClipboardUtil);

impl imgui::ClipboardBackend for Sdl2ClipboardBackend {
    fn get(&mut self) -> Option<imgui::ImString> {
        if !self.0.has_clipboard_text() {
            return None;
        }

        self.0.clipboard_text().ok().map(imgui::ImString::new)
    }

    fn set(&mut self, value: &imgui::ImStr) {
        let _ = self.0.set_clipboard_text(value.to_str());
    }
}

impl ImGuiSdl2 {
    pub fn new(imgui: &mut Context, window: &Window) -> Self {
        let clipboard_util = window.subsystem().clipboard();
        imgui.set_clipboard_backend(Box::new(Sdl2ClipboardBackend(clipboard_util)));

        imgui.io_mut().key_map[Key::Tab as usize] = Scancode::Tab as u32;
        imgui.io_mut().key_map[Key::LeftArrow as usize] = Scancode::Left as u32;
        imgui.io_mut().key_map[Key::RightArrow as usize] = Scancode::Right as u32;
        imgui.io_mut().key_map[Key::UpArrow as usize] = Scancode::Up as u32;
        imgui.io_mut().key_map[Key::DownArrow as usize] = Scancode::Down as u32;
        imgui.io_mut().key_map[Key::PageUp as usize] = Scancode::PageUp as u32;
        imgui.io_mut().key_map[Key::PageDown as usize] = Scancode::PageDown as u32;
        imgui.io_mut().key_map[Key::Home as usize] = Scancode::Home as u32;
        imgui.io_mut().key_map[Key::End as usize] = Scancode::End as u32;
        imgui.io_mut().key_map[Key::Delete as usize] = Scancode::Delete as u32;
        imgui.io_mut().key_map[Key::Backspace as usize] = Scancode::Backspace as u32;
        imgui.io_mut().key_map[Key::Enter as usize] = Scancode::Return as u32;
        imgui.io_mut().key_map[Key::Escape as usize] = Scancode::Escape as u32;
        imgui.io_mut().key_map[Key::Space as usize] = Scancode::Space as u32;
        imgui.io_mut().key_map[Key::A as usize] = Scancode::A as u32;
        imgui.io_mut().key_map[Key::C as usize] = Scancode::C as u32;
        imgui.io_mut().key_map[Key::V as usize] = Scancode::V as u32;
        imgui.io_mut().key_map[Key::X as usize] = Scancode::X as u32;
        imgui.io_mut().key_map[Key::Y as usize] = Scancode::Y as u32;
        imgui.io_mut().key_map[Key::Z as usize] = Scancode::Z as u32;

        Self {
            mouse_press: [false; 5],
            ignore_keyboard: false,
            ignore_mouse: false,
            cursor: None,
            sdl_cursor: None,
        }
    }

    fn ignore_event(&self, event: &Event) -> bool {
        match *event {
            Event::KeyDown { .. }
            | Event::KeyUp { .. }
            | Event::TextEditing { .. }
            | Event::TextInput { .. } => self.ignore_keyboard,
            Event::MouseMotion { .. }
            | Event::MouseButtonDown { .. }
            | Event::MouseButtonUp { .. }
            | Event::MouseWheel { .. }
            | Event::FingerDown { .. }
            | Event::FingerUp { .. }
            | Event::FingerMotion { .. }
            | Event::DollarGesture { .. }
            | Event::DollarRecord { .. }
            | Event::MultiGesture { .. } => self.ignore_mouse,
            _ => false,
        }
    }
}

impl Platform for ImGuiSdl2 {
    type Event = sdl2::event::Event;
    type Window = sdl2::video::Window;

    fn handle_event(&mut self, imgui: &mut Context, _window:&Self::Window, event: &Self::Event) -> bool {
        use sdl2::keyboard;
        use sdl2::mouse::MouseButton;

        fn set_mod(imgui: &mut Context, keymod: keyboard::Mod) {
            let ctrl = keymod.intersects(keyboard::Mod::RCTRLMOD | keyboard::Mod::LCTRLMOD);
            let alt = keymod.intersects(keyboard::Mod::RALTMOD | keyboard::Mod::LALTMOD);
            let shift = keymod.intersects(keyboard::Mod::RSHIFTMOD | keyboard::Mod::LSHIFTMOD);
            let super_ = keymod.intersects(keyboard::Mod::RGUIMOD | keyboard::Mod::LGUIMOD);

            imgui.io_mut().key_ctrl = ctrl;
            imgui.io_mut().key_alt = alt;
            imgui.io_mut().key_shift = shift;
            imgui.io_mut().key_super = super_;
        }

        match *event {
            Event::MouseWheel { y, .. } => {
                imgui.io_mut().mouse_wheel = y as f32;
            }
            Event::MouseButtonDown { mouse_btn, .. } => {
                if mouse_btn != MouseButton::Unknown {
                    let index = match mouse_btn {
                        MouseButton::Left => 0,
                        MouseButton::Right => 1,
                        MouseButton::Middle => 2,
                        MouseButton::X1 => 3,
                        MouseButton::X2 => 4,
                        MouseButton::Unknown => unreachable!(),
                    };
                    self.mouse_press[index] = true;
                }
            }
            Event::TextInput { ref text, .. } => {
                for chr in text.chars() {
                    imgui.io_mut().add_input_character(chr);
                }
            }
            Event::KeyDown {
                scancode, keymod, ..
            } => {
                set_mod(imgui, keymod);
                if let Some(scancode) = scancode {
                    imgui.io_mut().keys_down[scancode as usize] = true;
                }
            }
            Event::KeyUp {
                scancode, keymod, ..
            } => {
                set_mod(imgui, keymod);
                if let Some(scancode) = scancode {
                    imgui.io_mut().keys_down[scancode as usize] = false;
                }
            }
            _ => {}
        }

        self.ignore_event(event)
    }

    fn prepare_frame(&mut self, io: &mut imgui::Io, window: &Self::Window) {
        // Here we're CHEATING. We don't have acecss to `EventPump` but we can get the mouse state
        // (though (x, y) values can't be seet to it).
        let (mut x, mut y) = (0, 0);
        let mouse_state: u32 = unsafe { sdl2::sys::SDL_GetMouseState(&mut x, &mut y) };
        let mouse_state = sdl2::mouse::MouseState::from_sdl_state(mouse_state);

        let mouse_util = window.subsystem().sdl().mouse();

        let (win_w, win_h) = window.size();
        let (draw_w, draw_h) = window.drawable_size();

        io.display_size = [win_w as f32, win_h as f32];
        io.display_framebuffer_scale = [
            (draw_w as f32) / (win_w as f32),
            (draw_h as f32) / (win_h as f32),
        ];

        // Merging the mousedown events we received into the current state prevents us from missing
        // clicks that happen faster than a frame
        io.mouse_down = [
            self.mouse_press[0] || mouse_state.left(),
            self.mouse_press[1] || mouse_state.right(),
            self.mouse_press[2] || mouse_state.middle(),
            self.mouse_press[3] || mouse_state.x1(),
            self.mouse_press[4] || mouse_state.x2(),
        ];
        self.mouse_press = [false; 5];

        let any_mouse_down = io.mouse_down.iter().any(|&b| b);
        mouse_util.capture(any_mouse_down);

        io.mouse_pos = [x as f32, y as f32];

        self.ignore_keyboard = io.want_capture_keyboard;
        self.ignore_mouse = io.want_capture_mouse;
    }

    fn prepare_render(&mut self, ui: &imgui::Ui, window: &Self::Window) {
        let io = ui.io();
        if io
            .config_flags
            .contains(ConfigFlags::NO_MOUSE_CURSOR_CHANGE)
        {
            return;
        }

        let mouse_util = window.subsystem().sdl().mouse();
        match ui.mouse_cursor() {
            Some(mouse_cursor) if !io.mouse_draw_cursor => {
                mouse_util.show_cursor(true);

                let sdl_cursor = match mouse_cursor {
                    MouseCursor::Arrow => SystemCursor::Arrow,
                    MouseCursor::TextInput => SystemCursor::IBeam,
                    MouseCursor::ResizeAll => SystemCursor::SizeAll,
                    MouseCursor::ResizeNS => SystemCursor::SizeNS,
                    MouseCursor::ResizeEW => SystemCursor::SizeWE,
                    MouseCursor::ResizeNESW => SystemCursor::SizeNESW,
                    MouseCursor::ResizeNWSE => SystemCursor::SizeNWSE,
                    MouseCursor::Hand => SystemCursor::Hand,
                    MouseCursor::NotAllowed => SystemCursor::No,
                };

                if self.cursor != Some(mouse_cursor) {
                    let sdl_cursor = Cursor::from_system(sdl_cursor).unwrap();
                    sdl_cursor.set();
                    self.cursor = Some(mouse_cursor);
                    self.sdl_cursor = Some(sdl_cursor);
                }
            }
            _ => {
                self.cursor = None;
                self.sdl_cursor = None;
                mouse_util.show_cursor(false);
            }
        }
    }
}
