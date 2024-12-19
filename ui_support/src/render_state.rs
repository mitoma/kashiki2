use std::sync::{mpsc::Receiver, Arc};

use font_collector::FontRepository;
use font_rasterizer::{
    char_width_calcurator::CharWidthCalculator,
    color_theme::ColorTheme,
    context::{StateContext, WindowSize},
    font_buffer::{Direction, GlyphVertexBuffer},
    rasterizer_pipeline::{Quarity, RasterizerPipeline},
};
use image::{DynamicImage, ImageBuffer, Rgba};
use log::info;

use crate::{
    easing_value::EasingPointN, metrics_counter::record_start_of_phase, InputResult,
    SimpleStateCallback,
};

use stroke_parser::Action;
use wgpu::InstanceDescriptor;
use winit::{event::WindowEvent, window::Window};

// レンダリング対象を表す。
pub(crate) enum RenderTargetRequest {
    Window { window: Arc<Window> },
    Image { window_size: WindowSize },
}

impl RenderTargetRequest {
    fn window_size(&self) -> WindowSize {
        match self {
            RenderTargetRequest::Window { window } => WindowSize::from(window.inner_size()),
            RenderTargetRequest::Image { window_size } => *window_size,
        }
    }
}

// レンダリング結果を返す。
// ウィンドウの場合は画面に表示されるので値は無いが、イメージの場合はイメージバッファを返す。
pub enum RenderTargetResponse {
    Window,
    Image(ImageBuffer<Rgba<u8>, Vec<u8>>),
}

enum RenderTarget {
    Window {
        surface: wgpu::Surface<'static>,
        surface_texture: Option<wgpu::SurfaceTexture>,
        config: wgpu::SurfaceConfiguration,
    },
    Image {
        surface_texture: wgpu::Texture,
        output_buffer: wgpu::Buffer,
    },
}

impl RenderTarget {
    fn format(&self) -> wgpu::TextureFormat {
        match self {
            RenderTarget::Window { config, .. } => config.format,
            RenderTarget::Image {
                surface_texture, ..
            } => surface_texture.format(),
        }
    }

    fn get_screen_view(&mut self) -> wgpu::TextureView {
        match self {
            RenderTarget::Window {
                surface,
                surface_texture,
                ..
            } => {
                let st = surface.get_current_texture().unwrap();
                let texture_view = st
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                surface_texture.replace(st);
                texture_view
            }
            RenderTarget::Image {
                surface_texture, ..
            } => surface_texture.create_view(&wgpu::TextureViewDescriptor::default()),
        }
    }

    fn pre_submit(&mut self, encoder: &mut wgpu::CommandEncoder, context: &StateContext) {
        match self {
            RenderTarget::Window { .. } => {
                // 何もしない
            }
            RenderTarget::Image {
                ref surface_texture,
                ref output_buffer,
            } => {
                let size = context.window_size;
                let u32_size = std::mem::size_of::<u32>() as u32;
                encoder.copy_texture_to_buffer(
                    wgpu::ImageCopyTexture {
                        aspect: wgpu::TextureAspect::All,
                        texture: surface_texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                    },
                    wgpu::ImageCopyBuffer {
                        buffer: output_buffer,
                        layout: wgpu::ImageDataLayout {
                            offset: 0,
                            bytes_per_row: Some(u32_size * size.width),
                            rows_per_image: Some(size.height),
                        },
                    },
                    wgpu::Extent3d {
                        width: size.width,
                        height: size.height,
                        depth_or_array_layers: 1,
                    },
                );
            }
        }
    }

    fn flush(&mut self, context: &StateContext) -> RenderTargetResponse {
        match self {
            RenderTarget::Window {
                surface_texture, ..
            } => {
                surface_texture.take().unwrap().present();
                RenderTargetResponse::Window
            }
            RenderTarget::Image {
                ref output_buffer, ..
            } => {
                let size = context.window_size;
                let buffer = {
                    let buffer_slice = output_buffer.slice(..);

                    let (tx, rx) = std::sync::mpsc::channel();

                    // NOTE: We have to create the mapping THEN device.poll() before await
                    // the future. Otherwise the application will freeze.
                    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                        tx.send(result).unwrap();
                    });
                    context.device.poll(wgpu::Maintain::Wait);
                    rx.recv().unwrap().unwrap();

                    let data = buffer_slice.get_mapped_range();
                    let raw_data = data.to_vec();

                    ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(size.width, size.height, raw_data)
                        .unwrap()
                };
                output_buffer.unmap();
                RenderTargetResponse::Image(buffer)
            }
        }
    }
}

pub(crate) struct RenderState {
    pub(crate) context: StateContext,

    quarity: Quarity,

    render_target: RenderTarget,

    rasterizer_pipeline: RasterizerPipeline,
    glyph_vertex_buffer: GlyphVertexBuffer,

    simple_state_callback: Box<dyn SimpleStateCallback>,

    background_color: EasingPointN<4>,

    pub(crate) ui_string_receiver: Receiver<String>,
    pub(crate) action_queue_receiver: Receiver<Action>,
    pub(crate) post_action_queue_receiver: Receiver<Action>,
}

impl RenderState {
    pub(crate) async fn new(
        render_target_request: RenderTargetRequest,
        quarity: Quarity,
        color_theme: ColorTheme,
        mut simple_state_callback: Box<dyn SimpleStateCallback>,
        font_repository: FontRepository,
        performance_mode: bool,
    ) -> Self {
        let window_size = render_target_request.window_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(InstanceDescriptor::default());

        let surface = match &render_target_request {
            RenderTargetRequest::Window { window } => {
                Some(instance.create_surface(window.clone()).unwrap())
            }
            RenderTargetRequest::Image { .. } => None,
        };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: if performance_mode {
                    wgpu::PowerPreference::HighPerformance
                } else {
                    wgpu::PowerPreference::default()
                },
                compatible_surface: surface.as_ref(),
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
                    // memory_hints とか今後の新しい機能はまぁデフォルトで行きましょう。
                    ..Default::default()
                },
                None, // Trace path
            )
            .await
            .unwrap();

        let render_target = match render_target_request {
            RenderTargetRequest::Window { .. } => {
                let surface = surface.unwrap();
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
                    width: window_size.width,
                    height: window_size.height,
                    present_mode: wgpu::PresentMode::Fifo,
                    alpha_mode: wgpu::CompositeAlphaMode::Opaque,
                    view_formats: vec![],
                    desired_maximum_frame_latency: 2,
                };
                RenderTarget::Window {
                    surface,
                    surface_texture: None,
                    config,
                }
            }
            RenderTargetRequest::Image { window_size } => {
                let surface_texture_desc = wgpu::TextureDescriptor {
                    size: wgpu::Extent3d {
                        width: window_size.width,
                        height: window_size.height,
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
                let output_buffer_size =
                    (u32_size * window_size.width * window_size.height) as wgpu::BufferAddress;
                let output_buffer_desc = wgpu::BufferDescriptor {
                    size: output_buffer_size,
                    usage: wgpu::BufferUsages::COPY_DST
                        // this tells wpgu that we want to read this buffer from the cpu
                        | wgpu::BufferUsages::MAP_READ,
                    label: Some("Output Buffer"),
                    mapped_at_creation: false,
                };
                let output_buffer = device.create_buffer(&output_buffer_desc);

                RenderTarget::Image {
                    surface_texture,
                    output_buffer,
                }
            }
        };

        // 初期のパイプラインサイズは 256x256 で作成する(window_size が 0 の場合はエラーになるので)
        let initial_pipeline_size = (256, 256);
        let rasterizer_pipeline = RasterizerPipeline::new(
            &device,
            initial_pipeline_size.0,
            initial_pipeline_size.1,
            render_target.format(),
            quarity,
            color_theme.background().into(),
        );

        let font_binaries = font_repository.get_fonts();
        let font_binaries = Arc::new(font_binaries);
        let char_width_calcurator = Arc::new(CharWidthCalculator::new(font_binaries.clone()));
        let glyph_vertex_buffer =
            GlyphVertexBuffer::new(font_binaries, char_width_calcurator.clone());

        let (ui_string_sender, ui_string_receiver) = std::sync::mpsc::channel();
        let (action_queue_sender, action_queue_receiver) = std::sync::mpsc::channel();
        let (post_action_queue_sender, post_action_queue_receiver) = std::sync::mpsc::channel();

        let [r, g, b] = color_theme.background().get_color();
        let background_color = EasingPointN::new([r, g, b, 1.0]);

        let context = StateContext {
            device,
            queue,
            char_width_calcurator,
            color_theme,
            window_size,
            ui_string_sender,
            action_queue_sender,
            post_action_queue_sender,
            global_direction: Direction::Horizontal,
            font_repository,
        };

        simple_state_callback.init(&context);

        Self {
            context,
            quarity,

            render_target,

            rasterizer_pipeline,

            glyph_vertex_buffer,
            simple_state_callback,

            background_color,

            ui_string_receiver,
            action_queue_receiver,
            post_action_queue_receiver,
        }
    }

    pub(crate) fn redraw(&mut self) {
        self.resize(self.context.window_size)
    }

    pub(crate) fn change_color_theme(&mut self, color_theme: ColorTheme) {
        self.context.color_theme = color_theme;
        let [r, g, b] = self.context.color_theme.background().get_color();
        self.background_color.update([r, g, b, 1.0]);
    }

    pub(crate) fn change_background_image(&mut self, background_image: Option<DynamicImage>) {
        let color = match background_image {
            Some(_) => [0.0, 0.0, 0.0, 0.0],
            None => {
                let [r, g, b] = self.context.color_theme.background().get_color();
                [r, g, b, 1.0]
            }
        };
        info!("change_background_image, color: {:?}", color);
        self.background_color.update(color);

        self.rasterizer_pipeline.set_background_image(
            &self.context.device,
            &self.context.queue,
            background_image,
        );
    }

    pub(crate) fn resize(&mut self, new_size: WindowSize) {
        if new_size.width > 0 && new_size.height > 0 {
            self.context.window_size = new_size;

            match self.render_target {
                RenderTarget::Window {
                    ref mut surface,
                    ref mut config,
                    ..
                } => {
                    config.width = new_size.width;
                    config.height = new_size.height;
                    surface.configure(&self.context.device, config);
                }
                RenderTarget::Image { .. } => {
                    // 何もしない
                }
            }

            self.simple_state_callback.resize(new_size);

            // サイズ変更時にはパイプラインを作り直す
            self.rasterizer_pipeline = RasterizerPipeline::new(
                &self.context.device,
                new_size.width,
                new_size.height,
                self.render_target.format(),
                self.quarity,
                self.context.color_theme.background().into(),
            )
        }
    }

    pub(crate) fn input(&mut self, event: &WindowEvent) -> InputResult {
        self.simple_state_callback.input(&self.context, event)
    }

    pub(crate) fn action(&mut self, action: Action) -> InputResult {
        self.simple_state_callback.action(&self.context, action)
    }

    pub(crate) fn update(&mut self) {
        self.simple_state_callback.update(&self.context);
        if self.background_color.in_animation() {
            let [r, g, b, a] = self.background_color.current();
            self.rasterizer_pipeline.bg_color = wgpu::Color {
                r: r as f64,
                g: g as f64,
                b: b as f64,
                a: a as f64,
            };
        }
    }

    pub(crate) fn render(&mut self) -> Result<RenderTargetResponse, wgpu::SurfaceError> {
        record_start_of_phase("render 0: append glyph");
        self.ui_string_receiver
            .try_recv()
            .into_iter()
            .for_each(|s| {
                let _ = self.glyph_vertex_buffer.append_glyph(
                    &self.context.device,
                    &self.context.queue,
                    s.chars().collect(),
                );
            });

        record_start_of_phase("render 1: setup encoder");
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        // run all stage
        record_start_of_phase("render 2: update buffer");
        self.rasterizer_pipeline
            .overlap_bind_group
            .update_buffer(&self.context.queue);
        record_start_of_phase("render 2-2: create texture");
        let screen_view = self.render_target.get_screen_view();

        record_start_of_phase("render 3: callback render");
        let (camera, glyph_instances) = self.simple_state_callback.render();

        record_start_of_phase("render 4: run all stage");
        self.rasterizer_pipeline.run_all_stage(
            &mut encoder,
            &self.context.device,
            &self.context.queue,
            &self.glyph_vertex_buffer,
            (
                camera.build_view_projection_matrix().into(),
                camera.build_default_view_projection_matrix().into(),
            ),
            &glyph_instances,
            screen_view,
        );

        record_start_of_phase("render 5: submit");

        self.render_target.pre_submit(&mut encoder, &self.context);

        self.context.queue.submit(Some(encoder.finish()));

        let result = self.render_target.flush(&self.context);

        Ok(result)
    }

    pub(crate) fn shutdown(&mut self) {
        self.simple_state_callback.shutdown();
    }

    pub(crate) fn change_font(&mut self, font_name: String) {
        self.context.font_repository.set_primary_font(&font_name);
        let font_binaries = self.context.font_repository.get_fonts();
        let font_binaries = Arc::new(font_binaries);
        let char_width_calcurator = Arc::new(CharWidthCalculator::new(font_binaries.clone()));

        let registerd_chars = self.glyph_vertex_buffer.registerd_chars();
        self.glyph_vertex_buffer =
            GlyphVertexBuffer::new(font_binaries, char_width_calcurator.clone());
        let _ = self.glyph_vertex_buffer.append_glyph(
            &self.context.device,
            &self.context.queue,
            registerd_chars,
        );
        self.context.char_width_calcurator = char_width_calcurator;
    }
}
