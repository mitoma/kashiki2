use std::collections::HashSet;
use std::iter;

use log::{debug, info};
use winit::{event::*, window::Window};

use crate::{
    camera::{Camera, CameraController, CameraOperation},
    color_theme::ColorMode,
    font_vertex_buffer::FontVertexBuffer,
    rasterizer_pipeline::{Quarity, RasterizerPipeline},
    text::MultiLineText,
};

pub(crate) struct State {
    color_mode: ColorMode,

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

    single_line_text: MultiLineText,

    target: usize,
}

impl State {
    pub(crate) async fn new(window: &Window) -> Self {
        // テストデータ
        //        let sample_text = include_str!("../data/memo.md").to_string();
        let sample_text = include_str!("../data/gingatetsudono_yoru.txt").to_string();
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
            (0.0, 0.0, 100.0).into(),
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

        let single_line_text = MultiLineText::new(sample_text);

        Self {
            color_mode: ColorMode::SolarizedDark,

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

            single_line_text,

            target: 0,
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
        let op = self.camera_controller.process_event(event);

        if op {
            return op;
        }

        let op = match event {
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
                    VirtualKeyCode::A => {
                        self.target = 0;
                        self.single_line_text
                            .update_value("ハローワールド".to_string());
                        CameraOperation::None
                    }
                    VirtualKeyCode::H => {
                        if self.target >= 1 {
                            self.target -= 1;
                        }
                        if let Ok(target_vec) = self.single_line_text.get_target(self.target) {
                            let point = cgmath::point3(target_vec.x, target_vec.y, target_vec.z);
                            CameraOperation::CangeTarget(point)
                        } else {
                            CameraOperation::None
                        }
                    }
                    VirtualKeyCode::L => {
                        self.target += 1;
                        if let Ok(target_vec) = self.single_line_text.get_target(self.target) {
                            let point = cgmath::point3(target_vec.x, target_vec.y, target_vec.z);
                            CameraOperation::CangeTarget(point)
                        } else {
                            CameraOperation::None
                        }
                    }
                    _ => CameraOperation::None,
                }
            }
            _ => CameraOperation::None,
        };
        self.camera_controller.process(&op);
        op != CameraOperation::None
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
            &mut encoder,
            &self.font_vertex_buffer,
            &self.single_line_text.generate_instances(
                self.color_mode,
                &self.font_vertex_buffer,
                &self.device,
                &self.queue,
            ),
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
