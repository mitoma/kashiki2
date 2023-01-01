use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::state::State;

mod camera;
mod font_vertex_buffer;
mod instances;
mod outline_bind_group;
mod overlap_bind_group;
mod rasterizer_pipeline;
mod screen_bind_group;
mod screen_texture;
mod screen_vertex_buffer;
mod state;
mod text;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Warn).expect("Could't initialize logger");
        } else {
            env_logger::init();
        }
    }

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    #[cfg(target_arch = "wasm32")]
    {
        // Winit prevents sizing with CSS, so we have to set
        // the size manually when on web.
        use winit::dpi::PhysicalSize;
        window.set_inner_size(PhysicalSize::new(450, 400));

        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("wasm-example")?;
                let canvas = web_sys::Element::from(window.canvas());
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
    }

    // State::new uses async code, so we're going to wait for it to finish
    let mut state = State::new(&window).await;

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                if !state.input(event) {
                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(physical_size) => {
                            state.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            // new_inner_size is &mut so w have to dereference it twice
                            state.resize(**new_inner_size);
                        }
                        _ => {}
                    }
                }
            }
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                state.update();
                match state.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if it's lost or outdated
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => state.redraw(),
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // We're ignoring timeouts
                    Err(wgpu::SurfaceError::Timeout) => log::warn!("Surface timeout"),
                }
            }
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                window.request_redraw();
            }
            _ => {}
        }
    });
}

pub(crate) enum SolarizedColor {
    BASE03,
    BASE02,
    BASE01,
    BASE00,
    BASE0,
    BASE1,
    BASE2,
    BASE3,
    YELLOW,
    ORANGE,
    RED,
    MAGENTA,
    VIOLET,
    BLUE,
    CYAN,
    GREEN,
}

impl SolarizedColor {
    pub(crate) fn get_color(&self) -> [f32; 3] {
        match self {
            SolarizedColor::BASE03 => [0.0030352699, 0.0241576303, 0.0368894450],
            SolarizedColor::BASE02 => [0.0065120910, 0.0368894450, 0.0544802807],
            SolarizedColor::BASE01 => [0.0975873619, 0.1559264660, 0.1778884083],
            SolarizedColor::BASE00 => [0.1301364899, 0.1980693042, 0.2269658893],
            SolarizedColor::BASE0 => [0.2269658893, 0.2961383164, 0.3049873710],
            SolarizedColor::BASE1 => [0.2917706966, 0.3564002514, 0.3564002514],
            SolarizedColor::BASE2 => [0.8549926877, 0.8069523573, 0.6653873324],
            SolarizedColor::BASE3 => [0.9822505713, 0.9215820432, 0.7681512833],
            SolarizedColor::YELLOW => [0.4620770514, 0.2501583695, 0.0000000000],
            SolarizedColor::ORANGE => [0.5972018838, 0.0703601092, 0.0080231922],
            SolarizedColor::RED => [0.7156936526, 0.0318960287, 0.0284260381],
            SolarizedColor::MAGENTA => [0.6514056921, 0.0368894450, 0.2232279778],
            SolarizedColor::VIOLET => [0.1499598026, 0.1651322246, 0.5520114899],
            SolarizedColor::BLUE => [0.0193823613, 0.2581829131, 0.6444797516],
            SolarizedColor::CYAN => [0.0231533647, 0.3564002514, 0.3139887452],
            SolarizedColor::GREEN => [0.2345506549, 0.3185468316, 0.0000000000],
        }
    }
}

#[cfg(test)]
mod test {

    // good color scheme.
    // https://ethanschoonover.com/solarized/
    const SCHEMES: [(&str, u32, u32, u32); 16] = [
        ("base03", 10, 43, 54),
        ("base02", 19, 54, 66),
        ("base01", 88, 110, 117),
        ("base00", 101, 123, 131),
        ("base0", 131, 148, 150),
        ("base1", 147, 161, 161),
        ("base2", 238, 232, 213),
        ("base3", 253, 246, 227),
        ("yellow", 181, 137, 0),
        ("orange", 203, 75, 22),
        ("red", 220, 50, 47),
        ("magenta", 211, 54, 130),
        ("violet", 108, 113, 196),
        ("blue", 38, 139, 210),
        ("cyan", 42, 161, 152),
        ("green", 133, 153, 0),
    ];

    #[test]
    fn generate_color_table_for_wgsl() {
        SCHEMES.iter().for_each(|scheme| {
            println!(
                "let {:10} = vec4<f32>({:.10}, {:.10}, {:.10}, 1.0);",
                scheme.0,
                linear_to_srgb(scheme.1),
                linear_to_srgb(scheme.2),
                linear_to_srgb(scheme.3)
            );
        });
    }

    #[test]
    fn generate_color_table_for_rust_enum() {
        println!("pub(crate) enum SolarizedColor {{");
        SCHEMES.iter().for_each(|scheme| {
            println!(
                "{:10}({:.10}, {:.10}, {:.10}),",
                scheme.0.to_uppercase(),
                linear_to_srgb(scheme.1),
                linear_to_srgb(scheme.2),
                linear_to_srgb(scheme.3)
            );
        });
        println!("}};");

        SCHEMES.iter().for_each(|scheme| {
            println!(
                "SolarizedColor::{:10} => [{:.10}, {:.10}, {:.10}],",
                scheme.0.to_uppercase(),
                linear_to_srgb(scheme.1),
                linear_to_srgb(scheme.2),
                linear_to_srgb(scheme.3)
            );
        });
        println!("}};");
    }

    // こちらの記事を参考に linear の RGB 情報を sRGB に変換
    // http://www.psy.ritsumei.ac.jp/~akitaoka/RGBtoXYZ_etal01.html
    // https://en.wikipedia.org/wiki/SRGB
    fn linear_to_srgb(value: u32) -> f32 {
        let value: f32 = value as f32 / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }
}
