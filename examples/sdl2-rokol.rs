/*
SDL2 + Rokol (graphics)
*/

use {
    anyhow::{Error, Result},
    imgui_backends::{helper::QuickStart, platform::ImGuiSdl2, renderer::ImGuiRokolGfx, Platform},
    rokol::gfx as rg,
    sdl2::event::Event,
    std::time::Duration,
};

type Backend = imgui_backends::Backend<ImGuiSdl2, ImGuiRokolGfx>;

const W: u32 = 1280;
const H: u32 = 720;

fn main() -> Result<()> {
    env_logger::init();

    let mut handles = rokol::glue::sdl::Init {
        title: "SDL2 + Rokol".to_string(),
        w: W,
        h: H,
        use_high_dpi: false,
        settings: Default::default(),
    }
    .init(|window_builder| {
        window_builder.position_centered();
    })
    .map_err(Error::msg)?;

    let mut backend = {
        let mut imgui = QuickStart {
            display_size: [W as f32, H as f32],
            fontsize: 13.0,
            hidpi_factor: 1.0,
        }
        .create_context();

        let platform = ImGuiSdl2::new(&mut imgui, &handles.win);
        let renderer = ImGuiRokolGfx::new(&mut imgui)?;

        Backend {
            imgui,
            platform,
            renderer,
        }
    };

    let mut pump = handles.sdl.event_pump().map_err(Error::msg)?;
    // clear screen with cornflower blue
    let pa = rg::PassAction::clear([100.0 / 255.0, 149.0 / 255.0, 237.0 / 255.0, 1.0]);

    'running: loop {
        for ev in pump.poll_iter() {
            match ev {
                Event::Quit { .. } => break 'running,
                _ => {}
            }

            backend.handle_event(&mut handles.win, &ev);
        }

        // something like 30 FPS. do not use it for real applications
        let dt = Duration::from_nanos(1_000_000_000 / 30);
        backend.update_delta_time(dt);

        // FIXME: Can it be cheaper? This is just clearing the screen.
        rg::begin_default_pass(&pa, 1280, 720);
        rg::end_pass();

        let mut dummy_device = ();
        let ui = backend.begin_frame(&handles.win);
        ui.show_demo_window(&mut true);
        ui.end_frame(&mut handles.win, &mut dummy_device)?;

        // swap buffer
        rg::commit();
        handles.swap_window();

        std::thread::sleep(dt);
    }

    Ok(())
}
