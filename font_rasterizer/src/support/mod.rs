mod metrics_counter;
mod render_rate_adjuster;

use std::{collections::HashSet, iter, sync::Arc};

use crate::{
    camera::Camera,
    color_theme::ColorTheme,
    font_buffer::GlyphVertexBuffer,
    instances::GlyphInstances,
    rasterizer_pipeline::{Quarity, RasterizerPipeline},
    support::{
        metrics_counter::{print_metrics_to_stdout, record_start_of_phase},
        render_rate_adjuster::RenderRateAdjuster,
    },
    time::{increment_fixed_clock, set_clock_mode, ClockMode},
};

use bitflags::bitflags;
use font_collector::FontData;
use image::{ImageBuffer, Rgba};
use instant::Duration;

use wgpu::InstanceDescriptor;
use winit::{
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::{Key, NamedKey},
    window::{Fullscreen, Icon, Window, WindowBuilder},
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
    pub window_size: (u16, u16),
    pub callback: Box<dyn SimpleStateCallback>,
    pub quarity: Quarity,
    pub color_theme: ColorTheme,
    pub flags: Flags,
    pub font_binaries: Vec<FontData>,
}

pub async fn run_support(support: SimpleStateSupport) {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Warn).expect("Could't initialize logger");
        } else {
            env_logger::try_init().unwrap_or_default();
        }
    }
    record_start_of_phase("initialize");

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_window_icon(support.window_icon)
        .with_title(support.window_title)
        .with_inner_size(winit::dpi::LogicalSize {
            width: support.window_size.0,
            height: support.window_size.1,
        })
        .with_transparent(support.flags.contains(Flags::TRANCEPARENT))
        .with_decorations(!support.flags.contains(Flags::NO_TITLEBAR))
        .build(&event_loop)
        .unwrap();
    window.set_ime_allowed(true);
    let window = Arc::new(window);

    #[cfg(target_arch = "wasm32")]
    {
        // Winit prevents sizing with CSS, so we have to set
        // the size manually when on web.
        use winit::dpi::PhysicalSize;
        let _ = window.request_inner_size(PhysicalSize::new(800, 600));

        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("wasm-example")?;
                let canvas = web_sys::Element::from(window.canvas()?);
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
    }

    record_start_of_phase("setup state");
    let mut state = SimpleState::new(
        window.clone(),
        support.quarity,
        support.color_theme,
        support.callback,
        support.font_binaries,
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

    event_loop
        .run(move |event, control_flow| {
            match event {
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == window.id() => {
                    record_start_of_phase("state input");
                    match state.input(event) {
                        InputResult::InputConsumed => {
                            // state 内で処理が行われたので何もしない
                        }
                        InputResult::SendExit => {
                            print_metrics_to_stdout();
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
                        InputResult::ChangeColorTheme(color_theme) => {
                            state.change_color_theme(color_theme);
                        }
                        InputResult::Noop => {
                            match event {
                                WindowEvent::CloseRequested => {
                                    print_metrics_to_stdout();
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
                                    record_start_of_phase("state resize");
                                    state.resize(*physical_size);
                                }
                                WindowEvent::ScaleFactorChanged { .. } => {
                                    // TODO スケールファクタ変更時に何かする？
                                }
                                WindowEvent::RedrawRequested => {
                                    record_start_of_phase("state update");
                                    if render_rate_adjuster.skip() {
                                        return;
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
                                        Err(wgpu::SurfaceError::OutOfMemory) => control_flow.exit(),
                                        // We're ignoring timeouts
                                        Err(wgpu::SurfaceError::Timeout) => {
                                            log::warn!("Surface timeout")
                                        }
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
            record_start_of_phase("wait for next event");
        })
        .unwrap();
}

pub enum InputResult {
    InputConsumed,
    ToggleFullScreen,
    ChangeColorTheme(ColorTheme),
    SendExit,
    Noop,
}

pub trait SimpleStateCallback {
    fn init(
        &mut self,
        glyph_vertex_buffer: &mut GlyphVertexBuffer,
        color_theme: &ColorTheme,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    );
    fn resize(&mut self, width: u32, height: u32);
    fn update(
        &mut self,
        glyph_vertex_buffer: &mut GlyphVertexBuffer,
        color_theme: &ColorTheme,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    );
    fn input(&mut self, event: &WindowEvent) -> InputResult;
    fn render(&mut self) -> (&Camera, Vec<&GlyphInstances>);
}

pub struct SimpleState {
    quarity: Quarity,
    color_theme: ColorTheme,

    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,

    rasterizer_pipeline: RasterizerPipeline,
    glyph_vertex_buffer: GlyphVertexBuffer,

    simple_state_callback: Box<dyn SimpleStateCallback>,
}

impl SimpleState {
    pub async fn new(
        window: Arc<Window>,
        quarity: Quarity,
        color_theme: ColorTheme,
        mut simple_state_callback: Box<dyn SimpleStateCallback>,
        font_binaries: Vec<FontData>,
    ) -> Self {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(InstanceDescriptor::default());
        let surface = instance.create_surface(window).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web we'll have to disable some.
                    required_limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                },
                None, // Trace path
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let rasterizer_pipeline = RasterizerPipeline::new(
            &device,
            size.width,
            size.height,
            config.format,
            quarity,
            color_theme.background().into(),
        );
        let mut glyph_vertex_buffer = GlyphVertexBuffer::new(font_binaries);
        simple_state_callback.init(&mut glyph_vertex_buffer, &color_theme, &device, &queue);

        Self {
            color_theme,

            surface,
            device,
            queue,
            config,
            size,
            quarity,

            rasterizer_pipeline,

            glyph_vertex_buffer,
            simple_state_callback,
        }
    }

    pub fn redraw(&mut self) {
        self.resize(self.size)
    }

    fn change_color_theme(&mut self, color_theme: ColorTheme) {
        self.color_theme = color_theme;
        // カラーテーマ変更時にはパイプラインの色も同時に変更する
        self.rasterizer_pipeline.bg_color = self.color_theme.background().into();
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            self.simple_state_callback
                .resize(new_size.width, new_size.height);

            // サイズ変更時にはパイプラインを作り直す
            self.rasterizer_pipeline = RasterizerPipeline::new(
                &self.device,
                new_size.width,
                new_size.height,
                self.config.format,
                self.quarity,
                self.color_theme.background().into(),
            )
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> InputResult {
        self.simple_state_callback.input(event)
    }

    pub fn update(&mut self) {
        self.simple_state_callback.update(
            &mut self.glyph_vertex_buffer,
            &self.color_theme,
            &self.device,
            &self.queue,
        );
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // run all stage
        self.rasterizer_pipeline
            .overlap_bind_group
            .update_buffer(&self.queue);
        let screen_output = self.surface.get_current_texture()?;
        let screen_view = screen_output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let (camera, glyph_instances) = self.simple_state_callback.render();

        self.rasterizer_pipeline.run_all_stage(
            &mut encoder,
            &self.device,
            &self.queue,
            &self.glyph_vertex_buffer,
            (
                camera.build_view_projection_matrix().into(),
                camera.build_default_view_projection_matrix().into(),
            ),
            &glyph_instances,
            screen_view,
        );

        self.queue.submit(iter::once(encoder.finish()));
        screen_output.present();

        Ok(())
    }
}

pub struct ImageState {
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,

    surface_texture: wgpu::Texture,
    output_buffer: wgpu::Buffer,

    rasterizer_pipeline: RasterizerPipeline,
    glyph_vertex_buffer: GlyphVertexBuffer,

    color_theme: ColorTheme,

    simple_state_callback: Box<dyn SimpleStateCallback>,
}

impl ImageState {
    pub async fn new(
        image_size: (u32, u32),
        quarity: Quarity,
        color_theme: ColorTheme,
        mut simple_state_callback: Box<dyn SimpleStateCallback>,
        font_binaries: Vec<FontData>,
    ) -> Self {
        let size = winit::dpi::PhysicalSize {
            width: image_size.0,
            height: image_size.1,
        };

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web we'll have to disable some.
                    required_limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                },
                None, // Trace path
            )
            .await
            .unwrap();

        let surface_texture_desc = wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: image_size.0,
                height: image_size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            view_formats: &[],
            usage: wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING,
            label: None,
        };
        let surface_texture = device.create_texture(&surface_texture_desc);

        // create output buffer
        let u32_size = std::mem::size_of::<u32>() as u32;
        let output_buffer_size = (u32_size * image_size.0 * image_size.1) as wgpu::BufferAddress;
        let output_buffer_desc = wgpu::BufferDescriptor {
            size: output_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST
                // this tells wpgu that we want to read this buffer from the cpu
                | wgpu::BufferUsages::MAP_READ,
            label: Some("Output Buffer"),
            mapped_at_creation: false,
        };
        let output_buffer = device.create_buffer(&output_buffer_desc);

        let rasterizer_pipeline = RasterizerPipeline::new(
            &device,
            size.width,
            size.height,
            surface_texture.format(),
            quarity,
            color_theme.background().into(),
        );
        let mut glyph_vertex_buffer = GlyphVertexBuffer::new(font_binaries);
        simple_state_callback.init(&mut glyph_vertex_buffer, &color_theme, &device, &queue);
        simple_state_callback.resize(size.width, size.height);

        Self {
            device,
            queue,
            size,

            surface_texture,
            output_buffer,

            rasterizer_pipeline,

            glyph_vertex_buffer,
            simple_state_callback,

            color_theme,
        }
    }

    pub fn update(&mut self) {
        self.simple_state_callback.update(
            &mut self.glyph_vertex_buffer,
            &self.color_theme,
            &self.device,
            &self.queue,
        );
    }

    pub fn render(&mut self) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, wgpu::SurfaceError> {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // run all stage
        self.rasterizer_pipeline
            .overlap_bind_group
            .update_buffer(&self.queue);
        let screen_view = self
            .surface_texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let (camera, glyph_instances) = self.simple_state_callback.render();

        let chars = glyph_instances.iter().map(|i| i.c).collect::<HashSet<_>>();
        self.glyph_vertex_buffer
            .append_glyph(&self.device, &self.queue, chars)
            .unwrap();

        self.rasterizer_pipeline.run_all_stage(
            &mut encoder,
            &self.device,
            &self.queue,
            &self.glyph_vertex_buffer,
            (
                camera.build_view_projection_matrix().into(),
                camera.build_default_view_projection_matrix().into(),
            ),
            &glyph_instances,
            screen_view,
        );

        let u32_size = std::mem::size_of::<u32>() as u32;
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &self.surface_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::ImageCopyBuffer {
                buffer: &self.output_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(u32_size * self.size.width),
                    rows_per_image: Some(self.size.height),
                },
            },
            wgpu::Extent3d {
                width: self.size.width,
                height: self.size.height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(Some(encoder.finish()));

        let buffer = {
            let buffer_slice = self.output_buffer.slice(..);

            let (tx, rx) = std::sync::mpsc::channel();

            // NOTE: We have to create the mapping THEN device.poll() before await
            // the future. Otherwise the application will freeze.
            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                tx.send(result).unwrap();
            });
            self.device.poll(wgpu::Maintain::Wait);
            rx.recv().unwrap().unwrap();

            let data = buffer_slice.get_mapped_range();
            let raw_data = data.to_vec();

            ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(self.size.width, self.size.height, raw_data)
                .unwrap()
        };
        self.output_buffer.unmap();

        Ok(buffer)
    }
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
    let mut state = ImageState::new(
        (support.window_size.0 as u32, support.window_size.1 as u32),
        support.quarity,
        support.color_theme,
        support.callback,
        support.font_binaries,
    )
    .await;

    let mut frame = 0;
    loop {
        if frame > num_of_frame {
            break;
        }
        state.update();
        callback(state.render().unwrap(), frame);
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
    let mut state = ImageState::new(
        (support.window_size.0 as u32, support.window_size.1 as u32),
        support.quarity,
        support.color_theme,
        support.callback,
        support.font_binaries,
    )
    .await;

    (0..num_of_frame).map(move |frame| {
        state.update();
        let image = state.render().unwrap();

        increment_fixed_clock(frame_gain);
        (image, frame)
    })
}
