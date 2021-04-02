/*!
Rust-SDL2 + glow
*/

use {anyhow::*, glow::HasContext, sdl2::event::Event, std::time::Duration};

use imgui_backends::{helper::QuickStart, platform::ImGuiSdl2, renderer::ImGuiGlow};

type Backend = imgui_backends::Backend<ImGuiSdl2, ImGuiGlow>;

const TITLE: &'static str = "SDL2 + glow";
const W: u32 = 1280;
const H: u32 = 720;

/// SDL window with OpenGL context
pub struct SdlHandles {
    pub sdl: sdl2::Sdl,
    pub vid: sdl2::VideoSubsystem,
    pub win: sdl2::video::Window,
    pub gl: sdl2::video::GLContext,
}

impl SdlHandles {
    fn new() -> Result<Self> {
        let sdl = sdl2::init().map_err(Error::msg)?;
        let vid = sdl.video().map_err(Error::msg)?;

        // GlCore33
        let attr = vid.gl_attr();
        attr.set_context_profile(sdl2::video::GLProfile::Core);
        attr.set_context_version(3, 3);

        let win = vid
            .window(TITLE, W, H)
            .position_centered()
            .opengl()
            // .resizable()
            .build()
            .map_err(Error::msg)?;

        let gl = win.gl_create_context().unwrap();

        Ok(Self { sdl, vid, win, gl })
    }

    pub fn swap_window(&self) {
        self.win.gl_swap_window();
    }
}

fn main() -> Result<()> {
    env_logger::init();

    let mut handles = SdlHandles::new()?;
    let mut glow = unsafe {
        glow::Context::from_loader_function(|s| handles.vid.gl_get_proc_address(s) as *const _)
    };

    let mut backend = {
        let mut imgui = QuickStart {
            display_size: [W as f32, H as f32],
            fontsize: 13.0,
            hidpi_factor: 1.0,
        }
        .create_context();

        let platform = ImGuiSdl2::new(&mut imgui, &handles.win);
        let renderer = ImGuiGlow::new(&mut imgui, &glow)?;

        Backend {
            imgui,
            platform,
            renderer,
        }
    };

    let mut pump = handles.sdl.event_pump().map_err(Error::msg)?;

    'running: loop {
        let dt = Duration::from_nanos(1_000_000_000 / 30);

        for ev in pump.poll_iter() {
            match ev {
                Event::Quit { .. } => break 'running,
                _ => {}
            }

            backend.handle_event(&handles.win, &ev);
        }
        backend.update_delta_time(dt);

        unsafe {
            glow.clear(glow::COLOR_BUFFER_BIT);
        }

        let ui = backend.begin_frame(&handles.win);

        // use imgui here
        let mut b = true;
        ui.show_demo_window(&mut b);

        ui.end_frame(&mut handles.win, &mut glow)
            .map_err(Error::msg)?;

        // swap buffer
        handles.swap_window();

        // something like 30 FPS. do not use it for real applications
        std::thread::sleep(dt);
    }

    Ok(())
}
