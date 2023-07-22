use std::{collections::HashSet, iter};

use crate::{
    camera::Camera,
    font_buffer::GlyphVertexBuffer,
    instances::GlyphInstances,
    rasterizer_pipeline::{Quarity, RasterizerPipeline},
};
use bitflags::bitflags;
use wgpu::InstanceDescriptor;
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Fullscreen, Window, WindowBuilder},
};

bitflags! {
    pub struct Flags: u32 {
        const FULL_SCREEN  = 0b_0000_0001;
        const EXIT_ON_ESC  = 0b_0000_0010;
        const TRANCEPARENT = 0b_0000_0100;
        const NO_TITLEBAR  = 0b_0000_1000;
        const DEFAULT      = Self::EXIT_ON_ESC.bits() | Self::FULL_SCREEN.bits();
    }
}

pub struct SimpleStateSupport {
    pub window_title: String,
    pub window_size: (u16, u16),
    pub callback: Box<dyn SimpleStateCallback>,
    pub quarity: Quarity,
    pub bg_color: wgpu::Color,
    pub flags: Flags,
    pub font_binaries: Vec<Vec<u8>>,
}

pub async fn run_support(support: SimpleStateSupport) {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Warn).expect("Could't initialize logger");
        } else {
            env_logger::init();
        }
    }

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
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

    #[cfg(target_arch = "wasm32")]
    {
        // Winit prevents sizing with CSS, so we have to set
        // the size manually when on web.
        use winit::dpi::PhysicalSize;
        window.set_inner_size(PhysicalSize::new(800, 600));

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
    let mut state = SimpleState::new(
        &window,
        support.quarity,
        support.bg_color,
        support.callback,
        support.font_binaries,
    )
    .await;

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                match state.input(event) {
                    InputResult::InputConsumed => {}
                    InputResult::SendExit => *control_flow = ControlFlow::Exit,
                    InputResult::Noop => {
                        match event {
                            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                            WindowEvent::KeyboardInput {
                                input:
                                    KeyboardInput {
                                        state: ElementState::Pressed,
                                        virtual_keycode: Some(VirtualKeyCode::Escape),
                                        ..
                                    },
                                ..
                            } => {
                                if support.flags.contains(Flags::EXIT_ON_ESC) {
                                    *control_flow = ControlFlow::Exit
                                }
                            }
                            WindowEvent::KeyboardInput {
                                input:
                                    KeyboardInput {
                                        state: ElementState::Pressed,
                                        virtual_keycode: Some(VirtualKeyCode::F11),
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

pub enum InputResult {
    InputConsumed,
    SendExit,
    Noop,
}

pub trait SimpleStateCallback {
    fn init(
        &mut self,
        glyph_vertex_buffer: &mut GlyphVertexBuffer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    );
    fn resize(&mut self, width: u32, height: u32);
    fn update(
        &mut self,
        glyph_vertex_buffer: &mut GlyphVertexBuffer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    );
    fn input(&mut self, event: &WindowEvent) -> InputResult;
    fn render(&mut self) -> (&Camera, Vec<&GlyphInstances>);
}

pub struct SimpleState {
    quarity: Quarity,
    bg_color: wgpu::Color,

    surface: wgpu::Surface,
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
        window: &Window,
        quarity: Quarity,
        bg_color: wgpu::Color,
        mut simple_state_callback: Box<dyn SimpleStateCallback>,
        font_binaries: Vec<Vec<u8>>,
    ) -> Self {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(InstanceDescriptor::default());
        let surface = unsafe { instance.create_surface(window).unwrap() };
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
                    features: wgpu::Features::empty(),
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web we'll have to disable some.
                    limits: if cfg!(target_arch = "wasm32") {
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
        };
        surface.configure(&device, &config);

        let rasterizer_pipeline = RasterizerPipeline::new(
            &device,
            size.width,
            size.height,
            config.format,
            quarity,
            bg_color,
        );
        let mut glyph_vertex_buffer = GlyphVertexBuffer::new(font_binaries);
        simple_state_callback.init(&mut glyph_vertex_buffer, &device, &queue);

        Self {
            bg_color,

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
                self.bg_color,
            )
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> InputResult {
        self.simple_state_callback.input(event)
    }

    pub fn update(&mut self) {
        self.simple_state_callback
            .update(&mut self.glyph_vertex_buffer, &self.device, &self.queue);
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

        let chars = glyph_instances.iter().map(|i| i.c).collect::<HashSet<_>>();
        self.glyph_vertex_buffer
            .append_glyph(&self.device, &self.queue, chars)
            .unwrap();

        self.rasterizer_pipeline.run_all_stage(
            &mut encoder,
            &self.device,
            &self.queue,
            &self.glyph_vertex_buffer,
            camera.build_view_projection_matrix().into(),
            &glyph_instances,
            screen_view,
        );

        self.queue.submit(iter::once(encoder.finish()));
        screen_output.present();

        Ok(())
    }
}
