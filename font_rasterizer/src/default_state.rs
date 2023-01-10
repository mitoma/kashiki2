use std::iter;

use winit::{event::*, window::Window};

use crate::{
    camera::{Camera, CameraController},
    color_theme::ColorTheme,
    font_buffer::GlyphVertexBuffer,
    rasterizer_pipeline::{Quarity, RasterizerPipeline},
};

pub struct SimpleState {
    quarity: Quarity,
    color_theme: ColorTheme,

    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,

    camera: Camera,
    camera_controller: CameraController,

    rasterizer_pipeline: RasterizerPipeline,
    glyph_vertex_buffer: GlyphVertexBuffer,
}

impl SimpleState {
    pub async fn new(window: &Window, quarity: Quarity, color_theme: ColorTheme) -> Self {
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
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
        };
        surface.configure(&device, &config);

        // Camera
        let camera = Camera::new(
            (0.0, 0.0, 10.0).into(),
            (0.0, 0.0, 0.0).into(),
            cgmath::Vector3::unit_y(),
            config.width as f32 / config.height as f32,
            // fovy は視野角。ここでは45度を指定
            45.0,
            0.1,
            200.0,
        );
        let camera_controller = CameraController::new(10.0);

        let rasterizer_pipeline = RasterizerPipeline::new(
            &device,
            size.width,
            size.height,
            config.format,
            quarity,
            color_theme.background().into(),
        );

        let glyph_vertex_buffer = GlyphVertexBuffer::default();

        Self {
            color_theme,

            surface,
            device,
            queue,
            config,
            size,
            quarity,

            camera,
            camera_controller,
            rasterizer_pipeline,

            glyph_vertex_buffer,
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
            self.camera_controller.update_camera_aspect(
                &mut self.camera,
                new_size.width,
                new_size.height,
            );
            self.surface.configure(&self.device, &self.config);

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

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }

    pub fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.camera);
        self.camera_controller.reset_state();
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
        let view_proj = self.camera.build_view_projection_matrix().into();
        let screen_output = self.surface.get_current_texture()?;
        let screen_view = screen_output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.rasterizer_pipeline.run_all_stage(
            &mut encoder,
            &self.device,
            &self.queue,
            &self.glyph_vertex_buffer,
            view_proj,
            &[],
            screen_view,
        );

        self.queue.submit(iter::once(encoder.finish()));
        screen_output.present();

        Ok(())
    }
}
