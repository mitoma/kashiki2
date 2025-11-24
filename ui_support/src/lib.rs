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

use glam::Mat4;
use log::warn;
use pollster::block_on;
pub use render_state::RenderTargetResponse;
use text_instances::BorderType;
use ui::caret_char;

use std::sync::Arc;
#[cfg(target_arch = "wasm32")]
use std::sync::mpsc::{Receiver, Sender};

use camera::Camera;
use font_rasterizer::{
    color_theme::ColorTheme,
    context::WindowSize,
    glyph_instances::GlyphInstances,
    glyph_vertex_buffer::Direction,
    rasterizer_pipeline::Quarity,
    time::{ClockMode, increment_fixed_clock, set_clock_mode},
    vector_instances::VectorInstances,
};
use render_state::{RenderState, RenderTargetRequest};
use ui_context::UiContext;

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
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    icon::Icon,
    keyboard::{Key, NamedKey},
    monitor::Fullscreen,
    window::{ImeCapabilities, ImeEnableRequest, ImeRequestData, Window, WindowAttributes},
};

struct App {
    support: Option<SimpleStateSupport>,
    window: Option<Arc<Box<dyn Window>>>,
    attributes: Option<AppAttributes>,
    #[cfg(target_arch = "wasm32")]
    state_sender_receiver: (
        Sender<(RenderState, WindowSize, Flags)>,
        Receiver<(RenderState, WindowSize, Flags)>,
    ),
}

struct AppAttributes {
    render_rate_adjuster: RenderRateAdjuster,
    surface_configured: bool,
    state: RenderState,
    flags: Flags,
}

impl ApplicationHandler for App {
    fn can_create_surfaces(&mut self, event_loop: &dyn winit::event_loop::ActiveEventLoop) {
        let SimpleStateSupport {
            window_icon,
            window_title,
            window_size,
            callback,
            quarity,
            color_theme,
            flags,
            font_repository,
            performance_mode,
        } = self.support.take().expect("Support is not set");
        #[allow(unused_mut)]
        let mut window_attributes = WindowAttributes::default()
            .with_window_icon(window_icon.clone())
            .with_title(window_title.to_string())
            .with_surface_size(winit::dpi::LogicalSize {
                width: window_size.width,
                height: window_size.height,
            })
            .with_transparent(flags.contains(Flags::TRANCEPARENT))
            .with_decorations(!flags.contains(Flags::NO_TITLEBAR));

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesWeb;

            let target_canvas = web_sys::window()
                .and_then(|win| win.document())
                .and_then(|doc| doc.get_element_by_id("kashikishi-area"))
                .map(|canvas| canvas.unchecked_into());

            let window_attributes_web = WindowAttributesWeb::default().with_canvas(target_canvas);

            window_attributes =
                window_attributes.with_platform_attributes(Box::new(window_attributes_web));
        }

        self.attributes = match event_loop.create_window(window_attributes) {
            Ok(window) => {
                #[cfg(target_os = "macos")]
                {
                    // macOS では Option キーを Alt として扱う設定を有効にする
                    use winit::platform::macos::WindowExtMacOS;
                    window.set_option_as_alt(winit::platform::macos::OptionAsAlt::Both);
                }

                let req =
                    ImeEnableRequest::new(ImeCapabilities::default(), ImeRequestData::default())
                        .unwrap();
                let _ = window.request_ime_update(winit::window::ImeRequest::Enable(req));

                let window = Arc::new(window);
                self.window.replace(window.clone());

                #[cfg(not(target_arch = "wasm32"))]
                {
                    let state = block_on(RenderState::new(
                        RenderTargetRequest::Window {
                            window: window.clone(),
                        },
                        quarity,
                        color_theme,
                        callback,
                        font_repository,
                        performance_mode,
                    ));

                    // focus があるときは 120 FPS ぐらいまで出してもいいが focus が無い時は 5 FPS 程度にする。(GPU の負荷が高いので)
                    let render_rate_adjuster = RenderRateAdjuster::new(
                        120,
                        if flags.contains(Flags::SLEEP_WHEN_FOCUS_LOST) {
                            5
                        } else {
                            120
                        },
                    );
                    let surface_configured = false;

                    Some(AppAttributes {
                        render_rate_adjuster,
                        surface_configured,
                        state,
                        flags,
                    })
                }

                #[cfg(target_arch = "wasm32")]
                {
                    let sender = self.state_sender_receiver.0.clone();
                    let proxy = event_loop.create_proxy();

                    wasm_bindgen_futures::spawn_local(async move {
                        let state = RenderState::new(
                            RenderTargetRequest::Window { window },
                            quarity,
                            color_theme,
                            callback,
                            font_repository,
                            performance_mode,
                        )
                        .await;

                        sender.send((state, window_size, flags)).unwrap();
                        proxy.wake_up();
                    });

                    None
                }
            }
            Err(err) => {
                eprintln!("error creating window: {err}");
                event_loop.exit();
                return;
            }
        };
    }

    #[cfg(target_arch = "wasm32")]
    fn proxy_wake_up(&mut self, _event_loop: &dyn winit::event_loop::ActiveEventLoop) {
        let Some(window) = &self.window else {
            return;
        };

        if let Ok((state, window_size, flags)) = self.state_sender_receiver.1.recv() {
            use winit::dpi::PhysicalSize;

            let render_rate_adjuster = RenderRateAdjuster::new(120, 5);
            let surface_configured = false;

            self.attributes = Some(AppAttributes {
                render_rate_adjuster,
                surface_configured,
                state,
                flags,
            });
            let _ = window.request_surface_size(
                PhysicalSize::new(window_size.width, window_size.height).into(),
            );
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &dyn winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let Some(attributes) = &mut self.attributes else {
            return;
        };
        let Some(window) = &self.window else {
            return;
        };
        let self_window_id = window.id();
        if window_id != self_window_id {
            return;
        }
        let Some(window) = &mut self.window else {
            return;
        };

        let AppAttributes {
            render_rate_adjuster,
            surface_configured,
            state,
            flags,
        } = attributes;

        record_start_of_phase("state input");
        let input_result = state.input(&event);

        match input_result {
            InputResult::InputConsumed => {
                // state 内で処理が行われたので何もしない
            }
            InputResult::SendExit => {
                print_metrics_to_stdout();
                state.shutdown();
                event_loop.exit()
            }
            InputResult::ToggleFullScreen => {
                if flags.contains(Flags::FULL_SCREEN) {
                    match window.fullscreen() {
                        Some(_) => window.set_fullscreen(None),
                        None => window.set_fullscreen(Some(Fullscreen::Borderless(None))),
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
                state.context.set_global_direction(direction);
            }
            InputResult::ChangeWindowSize(window_size) => {
                state.resize(window_size);
            }
            InputResult::ChangeQuarity(quarity) => {
                state.change_quarity(quarity);
            }
            InputResult::Noop => {
                match event {
                    WindowEvent::CloseRequested => {
                        print_metrics_to_stdout();
                        state.shutdown();
                        event_loop.exit();
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
                        if flags.contains(Flags::EXIT_ON_ESC) {
                            print_metrics_to_stdout();
                            state.shutdown();
                            event_loop.exit();
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
                        if flags.contains(Flags::FULL_SCREEN) {
                            match window.fullscreen() {
                                Some(_) => window.set_fullscreen(None),
                                None => window.set_fullscreen(Some(Fullscreen::Borderless(None))),
                            }
                        }
                    }
                    WindowEvent::Focused(focused) => {
                        render_rate_adjuster.change_focus(focused);
                    }
                    WindowEvent::SurfaceResized(physical_size) => {
                        *surface_configured = true;
                        record_start_of_phase("state resize");
                        state.resize(WindowSize::new(physical_size.width, physical_size.height));
                    }
                    WindowEvent::ScaleFactorChanged { .. } => {
                        // TODO スケールファクタ変更時に何かする？
                    }
                    WindowEvent::RedrawRequested => {
                        if !*surface_configured {
                            return;
                        }
                        window.request_redraw();

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
                            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                                state.redraw()
                            }
                            // The system is out of memory, we should probably quit
                            Err(wgpu::SurfaceError::OutOfMemory) => {
                                print_metrics_to_stdout();
                                state.shutdown();
                                event_loop.exit();
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
                        while let Ok(action) = state.post_action_queue_receiver.try_recv() {
                            // この時の InputResult は処理不要のものを返す想定なのでハンドリングしない
                            let _ = state.action(action);
                        }
                    }
                    _ => {}
                }
            }
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
                    event_loop.exit()
                }
                InputResult::ToggleFullScreen => {
                    if flags.contains(Flags::FULL_SCREEN) {
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
                    state.context.set_global_direction(direction);
                }
                InputResult::ChangeWindowSize(window_size) => {
                    state.resize(window_size);
                }
                InputResult::ChangeQuarity(quarity) => state.change_quarity(quarity),
                InputResult::Noop => {}
            }
        }
        record_start_of_phase("wait for next event");
    }
}

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

    let _ = event_loop.run_app(App {
        support: Some(support),
        window: None,
        attributes: None,
        #[cfg(target_arch = "wasm32")]
        state_sender_receiver: {
            use std::sync::mpsc::channel;
            channel()
        },
    });
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
            state.context.set_global_direction(direction);
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
    fn init(&mut self, context: &UiContext);
    fn resize(&mut self, size: WindowSize);
    fn update(&mut self, context: &UiContext);
    fn input(&mut self, context: &UiContext, event: &WindowEvent) -> InputResult;
    fn action(&mut self, context: &UiContext, action: Action) -> InputResult;
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
    state.resize(support.window_size);

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
    state.resize(support.window_size);

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
pub fn register_default_caret(context: &UiContext) {
    context.register_svg(
        caret_char(text_buffer::caret::CaretType::Primary).to_string(),
        include_str!("../asset/caret_primary.svg").to_string(),
    );
    context.register_svg(
        caret_char(text_buffer::caret::CaretType::Mark).to_string(),
        include_str!("../asset/caret_mark.svg").to_string(),
    );
}

#[inline]
pub fn register_default_border(context: &UiContext) {
    context.register_svg(
        BorderType::Horizontal.to_key(),
        include_str!("../asset/border_horizontal.svg").to_string(),
    );
    context.register_svg(
        BorderType::Vertical.to_key(),
        include_str!("../asset/border_vertical.svg").to_string(),
    );
    context.register_svg(
        BorderType::TopLeft.to_key(),
        include_str!("../asset/border_top_left.svg").to_string(),
    );
    context.register_svg(
        BorderType::TopRight.to_key(),
        include_str!("../asset/border_top_right.svg").to_string(),
    );
    context.register_svg(
        BorderType::BottomLeft.to_key(),
        include_str!("../asset/border_bottom_left.svg").to_string(),
    );
    context.register_svg(
        BorderType::BottomRight.to_key(),
        include_str!("../asset/border_bottom_right.svg").to_string(),
    );
}

#[inline]
pub(crate) fn to_ndc_position(model: &dyn Model, camera: &Camera) -> (f32, f32) {
    let glam::Vec3 { x, y, z } = model.position();
    let position_vec = glam::Vec3 { x, y, z };

    let p = Mat4::from_translation(position_vec).mul_mat4(&Mat4::from_quat(model.rotation()));
    let view_projection_matrix = camera.build_view_projection_matrix();
    let calced_model_position = view_projection_matrix * p;
    let nw = calced_model_position.w_axis;
    let nw_x = nw.x / nw.w;
    let nw_y = nw.y / nw.w;
    (nw_x, nw_y)
}
