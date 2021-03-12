/*
SDL2 + Rust-FNA3D example
*/

use {
    anyhow::{Error, Result},
    fna3d::Color,
    imgui_backends::{helper::QuickStart, platform::ImGuiSdl2, renderer::ImGuiFna3d},
    sdl2::event::Event,
    std::time::Duration,
};

type Backend = imgui_backends::Backend<ImGuiSdl2, ImGuiFna3d>;

const W: u32 = 1280;
const H: u32 = 720;

/// Lifetime of the application
pub struct Init {
    pub sdl: sdl2::Sdl,
    pub vid: sdl2::VideoSubsystem,
    pub win: sdl2::video::Window,
    pub params: fna3d::PresentationParameters,
    pub device: fna3d::Device,
}

impl Init {
    /// Use it when calling [`fna3d::Device::swap_buffers`]
    pub fn raw_window(&self) -> *mut sdl2::sys::SDL_Window {
        self.win.raw()
    }

    /// Initializes the FNA3D device and the SDL2 window, wrapping them to an [`Init`] struct
    pub fn init(title: &str, size: (u32, u32)) -> Result<Init> {
        log::info!("FNA3D linked version: {}", fna3d::linked_version());
        fna3d::utils::hook_log_functions_default();

        let (sdl, vid, win) = {
            let flags = fna3d::prepare_window_attributes();

            let sdl = sdl2::init().map_err(Error::msg)?;
            let vid = sdl.video().map_err(Error::msg)?;
            let win = vid
                .window(title, size.0, size.1)
                .set_window_flags(flags.0)
                .position_centered()
                .build()
                .map_err(|e| e.to_string())
                .map_err(Error::msg)?;

            let size = fna3d::get_drawable_size(win.raw() as *mut _);
            log::info!("FNA3D drawable size: [{}, {}]", size.0, size.1);

            (sdl, vid, win)
        };

        let (params, device) = {
            let params = fna3d::utils::default_params_from_window_handle(win.raw() as *mut _);
            let do_debug = true;
            let device = fna3d::Device::from_params(params, do_debug);

            {
                let (max_tx, max_v_tx) = device.get_max_texture_slots();
                log::info!("device max textures: {}", max_tx);
                log::info!("device max vertex textures: {}", max_v_tx);
            }

            let vp = fna3d::Viewport {
                x: 0,
                y: 0,
                w: params.backBufferWidth as i32,
                h: params.backBufferHeight as i32,
                minDepth: 0.0,
                maxDepth: 1.0, // TODO: what's this
            };
            device.set_viewport(&vp);

            let rst = fna3d::RasterizerState::default();
            device.apply_rasterizer_state(&rst);

            let bst = fna3d::BlendState::alpha_blend();
            device.set_blend_state(&bst);

            (params, device)
        };

        Ok(Init {
            sdl,
            vid,
            win,
            params,
            device,
        })
    }
}

impl Init {
    pub fn create_imgui_backend(&mut self, mut icx: imgui::Context) -> Result<Backend> {
        let platform = ImGuiSdl2::new(&mut icx, &self.win);
        let renderer = ImGuiFna3d::init(&mut icx, &self.device)?;

        Ok(Backend {
            context: icx,
            platform,
            renderer,
        })
    }
}

pub fn main() -> Result<()> {
    env_logger::init();

    let title = "SDL2 + FNA3D";

    let mut init = Init::init(title, (W, H))?;

    let mut backend = {
        let icx = QuickStart {
            display_size: [W as f32, H as f32],
            fontsize: 13.0,
            hidpi_factor: 1.0,
        }
        .create_context();
        init.create_imgui_backend(icx)?
    };

    let mut pump = init.sdl.event_pump().map_err(Error::msg)?;

    'running: loop {
        let dt = Duration::from_nanos(1_000_000_000 / 30);

        for ev in pump.poll_iter() {
            match ev {
                Event::Quit { .. } => break 'running,
                _ => {}
            }

            backend.handle_event(&init.win, &ev);

            init.device.clear(
                fna3d::ClearOptions::TARGET,
                Color::rgb(120, 180, 140).to_vec4(),
                0.0, // depth
                0,   // stencil
            );

            let ui = backend.frame(&init.win, &mut init.device);

            let mut b = true;
            ui.show_demo_window(&mut b);

            // FIXME:
            ui.render_with_backend().unwrap();
            // ui.render_with_backend()?;

            init.device
                .swap_buffers(None, None, init.raw_window() as *mut _);

            // something like 30 FPS. do not use it for real applications
            std::thread::sleep(dt);
        }
    }

    Ok(())
}
