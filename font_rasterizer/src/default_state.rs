use std::{collections::HashSet, iter};

use winit::{event::*, window::Window};

use crate::{
    camera::Camera,
    font_buffer::GlyphVertexBuffer,
    instances::GlyphInstances,
    rasterizer_pipeline::{Quarity, RasterizerPipeline},
};

pub trait SimpleStateCallback {
    fn init(&mut self, device: &wgpu::Device, queue: &wgpu::Queue);
    fn resize(&mut self, width: u32, height: u32);
    fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue);
    fn input(&mut self, event: &WindowEvent) -> bool;
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
    ) -> Self {
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

        let rasterizer_pipeline = RasterizerPipeline::new(
            &device,
            size.width,
            size.height,
            config.format,
            quarity,
            bg_color,
        );
        let glyph_vertex_buffer = GlyphVertexBuffer::default();
        simple_state_callback.init(&device, &queue);

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

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        self.simple_state_callback.input(event)
    }

    pub fn update(&mut self) {
        self.simple_state_callback.update(&self.device, &self.queue);
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
