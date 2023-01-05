use std::collections::HashSet;
use std::iter;

use log::{debug, info};
use winit::{event::*, window::Window};

use crate::{
    camera::{Camera, CameraController, CameraOperation, EasingPoint3},
    color_theme::ColorMode,
    font_vertex_buffer::FontVertexBuffer,
    instances::Instances,
    rasterizer_pipeline::{Quarity, RasterizerPipeline},
    text::SingleLineText,
};

pub(crate) struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,

    quarity: Quarity,

    camera: Camera,
    camera_controller: CameraController,

    rasterizer_pipeline: RasterizerPipeline,

    font_vertex_buffer: FontVertexBuffer,

    instances: Vec<Instances>,
}

impl State {
    pub(crate) async fn new(window: &Window) -> Self {
        let color_mode = ColorMode::SolarizedLight;

        // テストデータ
        let sample_text = include_str!("../data/memo.md").to_string();
        // フォント情報の読み込みを動的にしたり切り替えるのはいずれやる必要あり
        let chars = sample_text.chars().collect::<HashSet<_>>();
        let chars = chars.iter().map(|c| *c..=*c).collect::<Vec<_>>();

        // ここから本来の処理
        let quarity = Quarity::VeryHigh;

        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(window) };
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

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_supported_formats(&adapter)[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
        };
        surface.configure(&device, &config);

        // Camera
        let camera = Camera::new(
            EasingPoint3::new(0.0, 0.0, 15.0),
            (0.0, 0.0, 0.0).into(),
            cgmath::Vector3::unit_y(),
            config.width as f32 / config.height as f32,
            // fovy は視野角。ここでは45度を指定
            45.0,
            0.1,
            200.0,
        );
        let camera_controller = CameraController::new(10.0);

        let rasterizer_pipeline =
            RasterizerPipeline::new(&device, size.width, size.height, config.format, quarity);

        let font_vertex_buffer = match FontVertexBuffer::new_buffer(&device, chars) {
            Ok(font_vertex_buffer) => font_vertex_buffer,
            Err(e) => {
                info!("err:{:?}", e);
                std::process::exit(2)
            }
        };

        let instances2 = SingleLineText(sample_text).to_instances(color_mode, &font_vertex_buffer);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            quarity,

            camera,
            camera_controller,
            rasterizer_pipeline,

            font_vertex_buffer,

            instances: instances2,
        }
    }

    pub(crate) fn redraw(&mut self) {
        self.resize(self.size)
    }

    pub(crate) fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.camera_controller.update_camera_aspect(
                &mut self.camera,
                new_size.width,
                new_size.height,
            );
            self.surface.configure(&self.device, &self.config);
            self.rasterizer_pipeline = RasterizerPipeline::new(
                &self.device,
                new_size.width,
                new_size.height,
                self.config.format,
                self.quarity,
            )
        }
    }

    #[allow(unused_variables)]
    pub(crate) fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let center_x = self.config.width as f64 / 2.0;
                let center_y = self.config.height as f64 / 2.0;
                if position.x > center_x {
                    self.update_camera(&CameraOperation::Left);
                } else if position.x < center_x {
                    self.update_camera(&CameraOperation::Right);
                }
                if position.y > center_y {
                    self.update_camera(&CameraOperation::Up);
                } else if position.y < center_y {
                    self.update_camera(&CameraOperation::Down);
                }
                true
            }
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state: ElementState::Pressed,
                        virtual_keycode: Some(code),
                        ..
                    },
                ..
            } => {
                debug!("Keycode: {:?}", code);
                match code {
                    VirtualKeyCode::Right => {
                        self.update_camera(&CameraOperation::Right);
                        true
                    }
                    VirtualKeyCode::Left => {
                        self.update_camera(&CameraOperation::Left);
                        true
                    }
                    VirtualKeyCode::Up => {
                        self.update_camera(&CameraOperation::Up);
                        true
                    }
                    VirtualKeyCode::Down => {
                        self.update_camera(&CameraOperation::Down);
                        true
                    }
                    VirtualKeyCode::PageUp => {
                        self.update_camera(&CameraOperation::Forward);
                        true
                    }
                    VirtualKeyCode::PageDown => {
                        self.update_camera(&CameraOperation::Backward);
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn update_camera(&mut self, operation: &CameraOperation) {
        self.camera_controller.process(operation);
    }

    pub(crate) fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.camera);
        self.rasterizer_pipeline
            .overlap_bind_group
            .update(self.camera.build_view_projection_matrix().into());
        self.camera_controller.reset_state();
    }

    pub(crate) fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Overlap Stage
        self.rasterizer_pipeline
            .overlap_bind_group
            .update_buffer(&self.queue);
        self.rasterizer_pipeline.overlap_stage(
            &self.device,
            &mut encoder,
            &self.font_vertex_buffer,
            &self.instances,
        );

        // Outline Stage
        self.rasterizer_pipeline
            .outline_stage(&self.device, &mut encoder);

        // Screen Stage
        let screen_output = self.surface.get_current_texture()?;
        {
            let screen_view = screen_output
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            self.rasterizer_pipeline
                .screen_stage(&self.device, &mut encoder, screen_view);
        }

        self.queue.submit(iter::once(encoder.finish()));
        screen_output.present();

        Ok(())
    }
}
