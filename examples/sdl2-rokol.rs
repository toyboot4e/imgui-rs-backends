/*
SDL2 + Rokol (graphics)
*/

use {
    anyhow::{Error, Result},
    imgui_backends::{
        helper::QuickStart, platform::ImGuiSdl2, renderer::ImGuiRokolGfx, Platform, Renderer,
    },
    rokol::gfx as rg,
    sdl2::event::Event,
    std::time::Duration,
};

type Backend = imgui_backends::Backend<ImGuiSdl2, ImGuiRokolGfx>;

const W: u32 = 1280;
const H: u32 = 720;

fn main() -> Result<()> {
    env_logger::init();

    let handles = rokol::glue::sdl::Init {
        name: "SDL2 + Rokol".to_string(),
        w: W,
        h: H,
        settings: Default::default(),
    }
    .init(|window_builder| {
        window_builder.position_centered();
    })
    .map_err(Error::msg)?;

    let mut backend = {
        let mut icx = QuickStart {
            display_size: [W as f32, H as f32],
            fontsize: 13.0,
            hidpi_factor: 1.0,
        }
        .create_context();

        let platform = ImGuiSdl2::new(&mut icx, &handles.win);
        let renderer = ImGuiRokolGfx::new(&mut icx)?;

        Backend {
            context: icx,
            platform,
            renderer,
        }
    };

    let mut pump = handles.sdl.event_pump().map_err(Error::msg)?;
    // clear screen with cornflower blue
    let pa = rg::PassAction::clear([100.0 / 255.0, 149.0 / 255.0, 237.0 / 255.0, 1.0]);

    'running: loop {
        let dt = Duration::from_nanos(1_000_000_000 / 30);

        for ev in pump.poll_iter() {
            match ev {
                Event::Quit { .. } => break 'running,
                _ => {}
            }

            backend
                .platform
                .handle_event(&mut backend.context, &handles.win, &ev);

            // FIXME: Can it be cheaper? This is just clearing the screen.
            rg::begin_default_pass(&pa, 1280, 720);
            rg::end_pass();

            // ----------
            let mut dummy_device = ();
            backend
                .platform
                .prepare_frame(backend.context.io_mut(), &handles.win);
            let ui = backend.context.frame();

            // use imgui here
            let mut b = true;
            ui.show_demo_window(&mut b);

            backend.platform.prepare_render(&ui, &handles.win);
            backend.renderer.render(ui.render(), &mut dummy_device)?;
            // ----------

            // swap buffer
            rg::commit();
            handles.swap_window();

            // something like 30 FPS. do not use it for real applications
            std::thread::sleep(dt);
        }
    }

    Ok(())
}
