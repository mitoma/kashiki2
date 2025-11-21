use std::sync::{Arc, mpsc::Receiver};

use font_collector::FontRepository;
use font_rasterizer::{
    char_width_calcurator::CharWidthCalculator,
    color_theme::ColorTheme,
    context::{Senders, StateContext, WindowSize},
    glyph_instances::GlyphInstances,
    glyph_vertex_buffer::{Direction, GlyphVertexBuffer},
    rasterizer_pipeline::{Buffers, Quarity, RasterizerPipeline},
    svg::SvgVertexBuffer,
    vector_instances::VectorInstances,
    vector_vertex_buffer::VectorVertexBuffer,
};
use image::{DynamicImage, ImageBuffer, Rgba};
use log::info;

use crate::{
    InputResult, RenderData, SimpleStateCallback, easing_value::EasingPointN,
    metrics_counter::record_start_of_phase,
};

use stroke_parser::Action;
use wgpu::{InstanceDescriptor, SurfaceError};
use winit::{dpi::PhysicalSize, event::WindowEvent, window::Window};

// レンダリング対象を表す。
pub(crate) enum RenderTargetRequest {
    Window { window: Arc<Box<dyn Window>> },
    Image { window_size: WindowSize },
}

impl RenderTargetRequest {
    fn window_size(&self) -> WindowSize {
        match self {
            RenderTargetRequest::Window { window } => {
                let PhysicalSize { width, height } = window.surface_size();
                WindowSize::new(width, height)
            }
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
    /// wgpu の COPY_BYTES_PER_ROW_ALIGNMENT (256バイト) 要件を満たすように
    /// パディングを追加した bytes_per_row を計算する。
    fn padded_bytes_per_row(width: u32, format: wgpu::TextureFormat) -> u32 {
        let bytes_per_pixel = format.block_copy_size(None).unwrap();
        let unpadded_bytes_per_row = bytes_per_pixel * width;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded = (unpadded_bytes_per_row + align - 1) / align * align;
        padded
    }

    fn format(&self) -> wgpu::TextureFormat {
        match self {
            RenderTarget::Window { config, .. } => config.format,
            RenderTarget::Image {
                surface_texture, ..
            } => surface_texture.format(),
        }
    }

    fn get_screen_view(&mut self) -> Result<wgpu::TextureView, SurfaceError> {
        match self {
            RenderTarget::Window {
                surface,
                surface_texture,
                ..
            } => {
                let st = surface.get_current_texture()?;
                let texture_view = st
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                surface_texture.replace(st);
                Ok(texture_view)
            }
            RenderTarget::Image {
                surface_texture, ..
            } => Ok(surface_texture.create_view(&wgpu::TextureViewDescriptor::default())),
        }
    }

    fn pre_submit(&mut self, encoder: &mut wgpu::CommandEncoder, context: &StateContext) {
        match &self {
            RenderTarget::Window { .. } => {
                // 何もしない
            }
            RenderTarget::Image {
                surface_texture,
                output_buffer,
            } => {
                let size = context.window_size;
                let padded_bytes_per_row =
                    Self::padded_bytes_per_row(size.width, surface_texture.format());
                encoder.copy_texture_to_buffer(
                    wgpu::TexelCopyTextureInfo {
                        aspect: wgpu::TextureAspect::All,
                        texture: surface_texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                    },
                    wgpu::TexelCopyBufferInfo {
                        buffer: output_buffer,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(padded_bytes_per_row),
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
                surface_texture,
                output_buffer,
            } => {
                let size = context.window_size;
                let format = surface_texture.format();
                let bytes_per_pixel = format.block_copy_size(None).unwrap();
                let padded_bytes_per_row = Self::padded_bytes_per_row(size.width, format);
                let unpadded_bytes_per_row = bytes_per_pixel * size.width;
                let buffer = {
                    let buffer_slice = output_buffer.slice(..);

                    let (tx, rx) = std::sync::mpsc::channel();

                    // NOTE: We have to create the mapping THEN device.poll() before await
                    // the future. Otherwise the application will freeze.
                    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                        tx.send(result).unwrap();
                    });
                    let _ = context.device.poll(wgpu::PollType::wait_indefinitely());
                    rx.recv().unwrap().unwrap();

                    let data = buffer_slice.get_mapped_range();

                    // パディングを除去して実際の画像データのみを抽出
                    let raw_data = if padded_bytes_per_row == unpadded_bytes_per_row {
                        // パディングがない場合は直接コピー（最適化）
                        data.to_vec()
                    } else {
                        // パディングがある場合は行ごとに必要なバイトのみを抽出
                        let mut result =
                            Vec::with_capacity((unpadded_bytes_per_row * size.height) as usize);
                        for row in 0..size.height {
                            let offset = (row * padded_bytes_per_row) as usize;
                            result.extend_from_slice(
                                &data[offset..offset + unpadded_bytes_per_row as usize],
                            );
                        }
                        result
                    };

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
    svg_vertex_buffer: SvgVertexBuffer,

    simple_state_callback: Box<dyn SimpleStateCallback>,

    background_color: EasingPointN<4>,
    background_image: Option<DynamicImage>,

    pub(crate) ui_string_receiver: Receiver<String>,
    pub(crate) ui_svg_receiver: Receiver<(String, String)>,
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
        let instance = wgpu::Instance::new(&InstanceDescriptor::default());

        let surface = match &render_target_request {
            RenderTargetRequest::Window { window } => {
                Some(instance.create_surface(window.clone()).unwrap())
            }
            RenderTargetRequest::Image { .. } => None,
        };

        let mut features = wgpu::Features::empty();

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

        if adapter
            .features()
            .contains(wgpu::Features::CONSERVATIVE_RASTERIZATION)
        {
            features |= wgpu::Features::CONSERVATIVE_RASTERIZATION;
        }

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: features,
                // WebGL doesn't support all of wgpu's features, so if
                // we're building for the web we'll have to disable some.
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                // memory_hints とか今後の新しい機能はまぁデフォルトで行きましょう。
                ..Default::default()
            })
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

                // create output buffer with proper alignment
                let padded_bytes_per_row = RenderTarget::padded_bytes_per_row(
                    window_size.width,
                    wgpu::TextureFormat::Rgba8UnormSrgb,
                );
                let output_buffer_size =
                    (padded_bytes_per_row * window_size.height) as wgpu::BufferAddress;
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
        let svg_vertex_buffer = SvgVertexBuffer::default();

        let (ui_string_sender, ui_string_receiver) = std::sync::mpsc::channel();
        let (ui_svg_sender, ui_svg_receiver) = std::sync::mpsc::channel();
        let (action_queue_sender, action_queue_receiver) = std::sync::mpsc::channel();
        let (post_action_queue_sender, post_action_queue_receiver) = std::sync::mpsc::channel();

        let [r, g, b] = color_theme.background().get_color();
        let background_color = EasingPointN::new([r, g, b, 1.0]);

        let context = StateContext::new(
            device,
            queue,
            char_width_calcurator,
            color_theme,
            window_size,
            Direction::Horizontal,
            font_repository,
            Senders::new(
                ui_string_sender,
                ui_svg_sender,
                action_queue_sender,
                post_action_queue_sender,
            ),
        );

        simple_state_callback.init(&context);

        Self {
            context,
            quarity,

            render_target,

            rasterizer_pipeline,

            glyph_vertex_buffer,
            svg_vertex_buffer,
            simple_state_callback,

            background_color,
            background_image: None,

            ui_string_receiver,
            ui_svg_receiver,
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
        let [r, g, b] = self.context.color_theme.background().get_color();
        let color = match background_image {
            Some(_) => [r, g, b, 0.9],
            None => [r, g, b, 1.0],
        };
        info!("change_background_image, color: {:?}", color);
        self.background_color.update(color);
        self.background_image = background_image;

        self.rasterizer_pipeline.set_background_image(
            &self.context.device,
            &self.context.queue,
            self.background_image.as_ref(),
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
                RenderTarget::Image {
                    ref mut surface_texture,
                    ref mut output_buffer,
                } => {
                    let surface_texture_desc = wgpu::TextureDescriptor {
                        size: wgpu::Extent3d {
                            width: new_size.width,
                            height: new_size.height,
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
                    *surface_texture = self.context.device.create_texture(&surface_texture_desc);

                    // create output buffer with proper alignment
                    let padded_bytes_per_row = RenderTarget::padded_bytes_per_row(
                        new_size.width,
                        wgpu::TextureFormat::Rgba8UnormSrgb,
                    );
                    let output_buffer_size =
                        (padded_bytes_per_row * new_size.height) as wgpu::BufferAddress;
                    let output_buffer_desc = wgpu::BufferDescriptor {
                        size: output_buffer_size,
                        usage: wgpu::BufferUsages::COPY_DST
                        // this tells wpgu that we want to read this buffer from the cpu
                        | wgpu::BufferUsages::MAP_READ,
                        label: Some("Output Buffer"),
                        mapped_at_creation: false,
                    };
                    *output_buffer = self.context.device.create_buffer(&output_buffer_desc);
                }
            }

            self.simple_state_callback.resize(new_size);

            let bg_color = self.rasterizer_pipeline.bg_color;
            // サイズ変更時にはパイプラインを作り直す
            self.rasterizer_pipeline = RasterizerPipeline::new(
                &self.context.device,
                new_size.width,
                new_size.height,
                self.render_target.format(),
                self.quarity,
                bg_color,
            );
            self.rasterizer_pipeline.set_background_image(
                &self.context.device,
                &self.context.queue,
                self.background_image.as_ref(),
            );
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
        while let Ok(s) = self.ui_string_receiver.try_recv() {
            let _ = self.glyph_vertex_buffer.append_chars(
                &self.context.device,
                &self.context.queue,
                s.chars().collect(),
            );
        }

        record_start_of_phase("render 0: append svg");
        while let Ok((key, svg)) = self.ui_svg_receiver.try_recv() {
            let _ = self.svg_vertex_buffer.append_svg(
                &self.context.device,
                &self.context.queue,
                &key,
                &svg,
            );
        }

        record_start_of_phase("render 1: setup encoder");
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        // run all stage
        record_start_of_phase("render 2: update buffer");
        self.rasterizer_pipeline.update_buffer(&self.context.queue);
        record_start_of_phase("render 2-2: create texture");
        let screen_view = self.render_target.get_screen_view()?;

        record_start_of_phase("render 3: callback render");
        let RenderData {
            camera,
            glyph_instances,
            vector_instances,
            glyph_instances_for_modal,
            vector_instances_for_modal,
        } = self.simple_state_callback.render();

        record_start_of_phase("render 4: run all stage");
        let buffers = {
            let glyph_buffers: Option<(&GlyphVertexBuffer, &[&GlyphInstances])> =
                if glyph_instances.is_empty() {
                    None
                } else {
                    Some((&self.glyph_vertex_buffer, &glyph_instances))
                };
            let vector_buffers: Option<(&VectorVertexBuffer<String>, &[&VectorInstances<String>])> =
                if vector_instances.is_empty() {
                    None
                } else {
                    Some((
                        self.svg_vertex_buffer.vector_vertex_buffer(),
                        &vector_instances,
                    ))
                };
            Buffers {
                glyph_buffers,
                vector_buffers,
            }
        };
        let modal_buffers = {
            let glyph_buffers: Option<(&GlyphVertexBuffer, &[&GlyphInstances])> =
                if glyph_instances_for_modal.is_empty() {
                    None
                } else {
                    Some((&self.glyph_vertex_buffer, &glyph_instances_for_modal))
                };
            let vector_buffers: Option<(&VectorVertexBuffer<String>, &[&VectorInstances<String>])> =
                if vector_instances_for_modal.is_empty() {
                    None
                } else {
                    Some((
                        self.svg_vertex_buffer.vector_vertex_buffer(),
                        &vector_instances_for_modal,
                    ))
                };
            Buffers {
                glyph_buffers,
                vector_buffers,
            }
        };

        self.rasterizer_pipeline.run_all_stage(
            &mut encoder,
            &self.context.device,
            &self.context.queue,
            (
                camera.build_view_projection_matrix().into(),
                camera.build_default_view_projection_matrix().into(),
            ),
            buffers,
            modal_buffers,
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

    pub(crate) fn change_font(&mut self, font_name: Option<String>) {
        match font_name {
            Some(font_name) => {
                self.context.font_repository.set_primary_font(&font_name);
            }
            None => {
                self.context.font_repository.clear_primary_font();
            }
        }
        let font_binaries = self.context.font_repository.get_fonts();
        let font_binaries = Arc::new(font_binaries);
        let char_width_calcurator = Arc::new(CharWidthCalculator::new(font_binaries.clone()));

        let registerd_chars = self.glyph_vertex_buffer.registerd_chars();
        self.glyph_vertex_buffer =
            GlyphVertexBuffer::new(font_binaries, char_width_calcurator.clone());
        let _ = self.glyph_vertex_buffer.append_chars(
            &self.context.device,
            &self.context.queue,
            registerd_chars,
        );
        self.context.char_width_calcurator = char_width_calcurator;
    }

    pub(crate) fn change_quarity(&mut self, quarity: Quarity) {
        if self.quarity != quarity {
            self.quarity = quarity;
            let bg_color = self.rasterizer_pipeline.bg_color;
            self.rasterizer_pipeline = RasterizerPipeline::new(
                &self.context.device,
                self.context.window_size.width,
                self.context.window_size.height,
                self.render_target.format(),
                self.quarity,
                bg_color,
            );
            self.rasterizer_pipeline.set_background_image(
                &self.context.device,
                &self.context.queue,
                self.background_image.as_ref(),
            );
        }
    }
}
