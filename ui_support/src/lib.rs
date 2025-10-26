pub mod action;
pub mod action_recorder;
pub mod camera;
mod easing_value;
pub mod layout_engine;
mod metrics_counter;
mod render_rate_adjuster;
mod render_state;
mod text_instances;
pub mod ui;
pub mod ui_context;

use log::warn;
pub use render_state::RenderTargetResponse;
use text_instances::BorderType;
use ui::caret_char;

use std::sync::Arc;

use camera::Camera;
use font_rasterizer::{
    color_theme::ColorTheme,
    context::{StateContext, WindowSize},
    glyph_instances::GlyphInstances,
    glyph_vertex_buffer::Direction,
    rasterizer_pipeline::Quarity,
    time::{ClockMode, increment_fixed_clock, set_clock_mode},
    vector_instances::VectorInstances,
};
use render_state::{RenderState, RenderTargetRequest};

use crate::{
    layout_engine::Model,
    metrics_counter::{print_metrics_to_stdout, record_start_of_phase},
    render_rate_adjuster::RenderRateAdjuster,
};

use bitflags::bitflags;
use font_collector::FontRepository;
use image::{DynamicImage, ImageBuffer, Rgba};
use web_time::Duration;

use stroke_parser::Action;
use winit::{
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::{Key, NamedKey},
    window::{Fullscreen, Icon, WindowBuilder},
};

bitflags! {
    pub struct Flags: u32 {
        const FULL_SCREEN  = 0b_0000_0001;
        const EXIT_ON_ESC  = 0b_0000_0010;
        const TRANCEPARENT = 0b_0000_0100;
        const NO_TITLEBAR  = 0b_0000_1000;
        // focus が無い時に省エネモードにするかは選択可能にする
        const SLEEP_WHEN_FOCUS_LOST = 0b_0001_0000;
        const DEFAULT      = Self::EXIT_ON_ESC.bits() | Self::FULL_SCREEN.bits() | Self::SLEEP_WHEN_FOCUS_LOST.bits();
    }
}

pub struct SimpleStateSupport {
    pub window_icon: Option<Icon>,
    pub window_title: String,
    pub window_size: WindowSize,
    pub callback: Box<dyn SimpleStateCallback>,
    pub quarity: Quarity,
    pub color_theme: ColorTheme,
    pub flags: Flags,
    pub font_repository: FontRepository,
    pub performance_mode: bool,
}

pub async fn run_support(support: SimpleStateSupport) {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Warn).expect("Could't initialize logger");
        } else {
            match env_logger::try_init() {
                Ok(_) => {},
                Err(_) => warn!("Logger is already initialized"),
            }
            use std::io::Write;
            let default_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                let mut file = std::fs::File::create("kashikishi.panic.log").expect("Could not create log file");
                writeln!(file, "{}", info).expect("Could not write to log file");
                default_hook(info);
            }));
        }
    }
    record_start_of_phase("initialize");

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_window_icon(support.window_icon)
        .with_title(support.window_title)
        .with_inner_size(winit::dpi::LogicalSize {
            width: support.window_size.width,
            height: support.window_size.height,
        })
        .with_transparent(support.flags.contains(Flags::TRANCEPARENT))
        .with_decorations(!support.flags.contains(Flags::NO_TITLEBAR))
        .build(&event_loop)
        .unwrap();
    window.set_ime_allowed(true);
    let window = Arc::new(window);

    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("kashikishi-area")?;
                let canvas = web_sys::Element::from(window.canvas()?);
                dst.append_child(&canvas.clone()).ok()?;
                let input = web_sys::Element::from(window.input()?);
                dst.append_child(&input).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");

        // Winit prevents sizing with CSS, so we have to set
        // the size manually when on web.
        use winit::dpi::PhysicalSize;
        let _ = window.request_inner_size(PhysicalSize::new(
            support.window_size.width,
            support.window_size.height,
        ));
    }

    record_start_of_phase("setup state");
    let mut state = RenderState::new(
        RenderTargetRequest::Window {
            window: window.clone(),
        },
        support.quarity,
        support.color_theme,
        support.callback,
        support.font_repository,
        support.performance_mode,
    )
    .await;

    // focus があるときは 120 FPS ぐらいまで出してもいいが focus が無い時は 5 FPS 程度にする。(GPU の負荷が高いので)
    let mut render_rate_adjuster = RenderRateAdjuster::new(
        120,
        if support.flags.contains(Flags::SLEEP_WHEN_FOCUS_LOST) {
            5
        } else {
            120
        },
    );
    let mut surface_configured = false;

    event_loop
        .run(move |event, control_flow| {
            match event {
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == window.id() => {
                    record_start_of_phase("state input");
                    let input_result = state.input(event);

                    match input_result {
                        InputResult::InputConsumed => {
                            // state 内で処理が行われたので何もしない
                        }
                        InputResult::SendExit => {
                            print_metrics_to_stdout();
                            state.shutdown();
                            control_flow.exit()
                        }
                        InputResult::ToggleFullScreen => {
                            if support.flags.contains(Flags::FULL_SCREEN) {
                                match window.fullscreen() {
                                    Some(_) => window.set_fullscreen(None),
                                    None => {
                                        window.set_fullscreen(Some(Fullscreen::Borderless(None)))
                                    }
                                }
                            }
                        }
                        InputResult::ToggleDecorations => {
                            window.set_decorations(!window.is_decorated());
                        }
                        InputResult::ChangeColorTheme(color_theme) => {
                            state.change_color_theme(color_theme);
                        }
                        InputResult::ChangeBackgroundImage(dynamic_image) => {
                            state.change_background_image(dynamic_image);
                        }
                        InputResult::ChangeFont(font_name) => {
                            state.change_font(font_name);
                        }
                        InputResult::ChangeGlobalDirection(direction) => {
                            state.context.global_direction = direction;
                        }
                        InputResult::ChangeWindowSize(window_size) => {
                            let _ = window.request_inner_size(winit::dpi::LogicalSize {
                                width: window_size.width,
                                height: window_size.height,
                            });
                        }
                        InputResult::ChangeQuarity(quarity) => {
                            state.change_quarity(quarity);
                        }
                        InputResult::Noop => {
                            match event {
                                WindowEvent::CloseRequested => {
                                    print_metrics_to_stdout();
                                    state.shutdown();
                                    control_flow.exit()
                                }
                                WindowEvent::KeyboardInput {
                                    event:
                                        KeyEvent {
                                            state: ElementState::Pressed,
                                            logical_key: Key::Named(NamedKey::Escape),
                                            ..
                                        },
                                    ..
                                } => {
                                    if support.flags.contains(Flags::EXIT_ON_ESC) {
                                        print_metrics_to_stdout();
                                        state.shutdown();
                                        control_flow.exit();
                                    }
                                }
                                WindowEvent::KeyboardInput {
                                    event:
                                        KeyEvent {
                                            state: ElementState::Pressed,
                                            logical_key: Key::Named(NamedKey::F11),
                                            ..
                                        },
                                    ..
                                } => {
                                    if support.flags.contains(Flags::FULL_SCREEN) {
                                        match window.fullscreen() {
                                            Some(_) => window.set_fullscreen(None),
                                            None => window
                                                .set_fullscreen(Some(Fullscreen::Borderless(None))),
                                        }
                                    }
                                }
                                WindowEvent::Focused(focused) => {
                                    render_rate_adjuster.change_focus(*focused);
                                }
                                WindowEvent::Resized(physical_size) => {
                                    surface_configured = true;
                                    record_start_of_phase("state resize");
                                    state.resize((*physical_size).into());
                                }
                                WindowEvent::ScaleFactorChanged { .. } => {
                                    // TODO スケールファクタ変更時に何かする？
                                }
                                WindowEvent::RedrawRequested => {
                                    if !surface_configured {
                                        return;
                                    }
                                    record_start_of_phase("state update");
                                    #[cfg(not(target_arch = "wasm32"))]
                                    {
                                        if let Some(idle_time) = render_rate_adjuster.idle_time() {
                                            std::thread::sleep(idle_time);
                                            return;
                                        }
                                    }
                                    state.update();
                                    record_start_of_phase("state render");
                                    match state.render() {
                                        Ok(_) => {}
                                        // Reconfigure the surface if it's lost or outdated
                                        Err(
                                            wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated,
                                        ) => state.redraw(),
                                        // The system is out of memory, we should probably quit
                                        Err(wgpu::SurfaceError::OutOfMemory) => {
                                            print_metrics_to_stdout();
                                            state.shutdown();
                                            control_flow.exit()
                                        }
                                        // We're ignoring timeouts
                                        Err(wgpu::SurfaceError::Timeout) => {
                                            log::warn!("Surface timeout")
                                        }
                                        Err(wgpu::SurfaceError::Other) => {
                                            log::warn!("Surface Other error")
                                        }
                                    }
                                    // 1 フレームごとに時計を更新する(時計のモードが StepByStep の場合のみ意味がある)
                                    increment_fixed_clock(Duration::ZERO);
                                    // 次のフレームの最初に action を処理するケースがある
                                    // 主なケースとしては、大量の文字を入力後にレイアウトを変更するケース。
                                    // この場合 render 後に post_action_queue_receiver にたまった action を処理するのが良い。
                                    while let Ok(action) =
                                        state.post_action_queue_receiver.try_recv()
                                    {
                                        // この時の InputResult は処理不要のものを返す想定なのでハンドリングしない
                                        let _ = state.action(action);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                Event::AboutToWait => {
                    window.request_redraw();
                }
                _ => {}
            }
            // input イベント後に連鎖して action が発生するケースがあるのでここで処理していく
            while let Ok(action) = state.action_queue_receiver.try_recv() {
                match state.action(action) {
                    InputResult::InputConsumed => {
                        // state 内で処理が行われたので何もしない
                    }
                    InputResult::SendExit => {
                        print_metrics_to_stdout();
                        state.shutdown();
                        control_flow.exit()
                    }
                    InputResult::ToggleFullScreen => {
                        if support.flags.contains(Flags::FULL_SCREEN) {
                            match window.fullscreen() {
                                Some(_) => window.set_fullscreen(None),
                                None => window.set_fullscreen(Some(Fullscreen::Borderless(None))),
                            }
                        }
                    }
                    InputResult::ChangeColorTheme(color_theme) => {
                        state.change_color_theme(color_theme);
                    }
                    InputResult::ChangeBackgroundImage(dynamic_image) => {
                        state.change_background_image(dynamic_image);
                    }
                    InputResult::ChangeFont(font_name) => {
                        state.change_font(font_name);
                    }
                    InputResult::ToggleDecorations => {
                        window.set_decorations(!window.is_decorated());
                    }
                    InputResult::ChangeGlobalDirection(direction) => {
                        state.context.global_direction = direction;
                    }
                    InputResult::ChangeWindowSize(window_size) => {
                        let _ = window.request_inner_size(winit::dpi::LogicalSize {
                            width: window_size.width,
                            height: window_size.height,
                        });
                    }
                    InputResult::ChangeQuarity(quarity) => state.change_quarity(quarity),
                    InputResult::Noop => {}
                }
            }
            record_start_of_phase("wait for next event");
        })
        .unwrap();
}

fn handle_action_result(input_result: InputResult, state: &mut RenderState) -> Option<InputResult> {
    match input_result {
        InputResult::ChangeColorTheme(color_theme) => {
            state.change_color_theme(color_theme);
            None
        }
        InputResult::ChangeBackgroundImage(dynamic_image) => {
            state.change_background_image(dynamic_image);
            None
        }
        InputResult::ChangeFont(font_name) => {
            state.change_font(font_name);
            None
        }
        InputResult::ChangeGlobalDirection(direction) => {
            state.context.global_direction = direction;
            None
        }
        InputResult::SendExit => Some(input_result),
        InputResult::ToggleFullScreen => Some(input_result),
        InputResult::ToggleDecorations => Some(input_result),
        InputResult::ChangeWindowSize(_) => Some(input_result),
        InputResult::ChangeQuarity(_) => Some(input_result),
        InputResult::InputConsumed => None,
        InputResult::Noop => None,
    }
}

#[derive(PartialEq)]
pub enum InputResult {
    InputConsumed,
    ToggleFullScreen,
    ToggleDecorations,
    ChangeColorTheme(ColorTheme),
    ChangeBackgroundImage(Option<DynamicImage>),
    ChangeGlobalDirection(Direction),
    ChangeWindowSize(WindowSize),
    ChangeFont(Option<String>),
    ChangeQuarity(Quarity),
    SendExit,
    Noop,
}

pub trait SimpleStateCallback {
    fn init(&mut self, context: &StateContext);
    fn resize(&mut self, size: WindowSize);
    fn update(&mut self, context: &StateContext);
    fn input(&mut self, context: &StateContext, event: &WindowEvent) -> InputResult;
    fn action(&mut self, context: &StateContext, action: Action) -> InputResult;
    fn render(&'_ mut self) -> RenderData<'_>;
    fn shutdown(&mut self);
}

pub struct RenderData<'a> {
    pub camera: &'a Camera,
    pub glyph_instances: Vec<&'a GlyphInstances>,
    pub vector_instances: Vec<&'a VectorInstances<String>>,
    pub glyph_instances_for_modal: Vec<&'a GlyphInstances>,
    pub vector_instances_for_modal: Vec<&'a VectorInstances<String>>,
}

pub async fn generate_images<F>(
    support: SimpleStateSupport,
    num_of_frame: u32,
    frame_gain: Duration,
    mut callback: F,
) where
    F: FnMut(ImageBuffer<Rgba<u8>, Vec<u8>>, u32),
    F: Send,
{
    set_clock_mode(ClockMode::Fixed);

    let mut state = RenderState::new(
        RenderTargetRequest::Image {
            window_size: support.window_size,
        },
        support.quarity,
        support.color_theme,
        support.callback,
        support.font_repository,
        support.performance_mode,
    )
    .await;

    let mut frame = 0;
    loop {
        if frame > num_of_frame {
            state.shutdown();
            break;
        }
        state.update();

        let image = if let RenderTargetResponse::Image(image) = state.render().unwrap() {
            image
        } else {
            panic!("image is not found")
        };
        callback(image, frame);
        increment_fixed_clock(frame_gain);
        frame += 1;
    }
}

pub async fn generate_image_iter(
    support: SimpleStateSupport,
    num_of_frame: u32,
    frame_gain: Duration,
) -> impl Iterator<Item = (ImageBuffer<Rgba<u8>, Vec<u8>>, u32)> {
    set_clock_mode(ClockMode::Fixed);

    let mut state = RenderState::new(
        RenderTargetRequest::Image {
            window_size: support.window_size,
        },
        support.quarity,
        support.color_theme,
        support.callback,
        support.font_repository,
        support.performance_mode,
    )
    .await;

    (0..num_of_frame).map(move |frame| {
        while let Ok(action) = state.action_queue_receiver.try_recv() {
            let _ = handle_action_result(state.action(action), &mut state);
        }

        state.update();
        let image = if let RenderTargetResponse::Image(image) = state.render().unwrap() {
            image
        } else {
            panic!("image is not found")
        };
        increment_fixed_clock(frame_gain);

        while let Ok(action) = state.post_action_queue_receiver.try_recv() {
            let _ = handle_action_result(state.action(action), &mut state);
        }

        (image, frame)
    })
}

#[inline]
pub fn register_default_caret(state_context: &StateContext) {
    state_context.register_svg(
        caret_char(text_buffer::caret::CaretType::Primary).to_string(),
        include_str!("../asset/caret_primary.svg").to_string(),
    );
    state_context.register_svg(
        caret_char(text_buffer::caret::CaretType::Mark).to_string(),
        include_str!("../asset/caret_mark.svg").to_string(),
    );
}

#[inline]
pub fn register_default_border(state_context: &StateContext) {
    state_context.register_svg(
        BorderType::Horizontal.to_key(),
        include_str!("../asset/border_horizontal.svg").to_string(),
    );
    state_context.register_svg(
        BorderType::Vertical.to_key(),
        include_str!("../asset/border_vertical.svg").to_string(),
    );
    state_context.register_svg(
        BorderType::TopLeft.to_key(),
        include_str!("../asset/border_top_left.svg").to_string(),
    );
    state_context.register_svg(
        BorderType::TopRight.to_key(),
        include_str!("../asset/border_top_right.svg").to_string(),
    );
    state_context.register_svg(
        BorderType::BottomLeft.to_key(),
        include_str!("../asset/border_bottom_left.svg").to_string(),
    );
    state_context.register_svg(
        BorderType::BottomRight.to_key(),
        include_str!("../asset/border_bottom_right.svg").to_string(),
    );
}

#[inline]
pub(crate) fn to_ndc_position(model: &dyn Model, camera: &Camera) -> (f32, f32) {
    let cgmath::Point3 { x, y, z } = model.position();
    let position_vec = cgmath::Vector3 { x, y, z };

    let p =
        cgmath::Matrix4::from_translation(position_vec) * cgmath::Matrix4::from(model.rotation());
    let view_projection_matrix = camera.build_view_projection_matrix();
    let calced_model_position = view_projection_matrix * p;
    let nw = calced_model_position.w;
    let nw_x = nw.x / nw.w;
    let nw_y = nw.y / nw.w;
    (nw_x, nw_y)
}
