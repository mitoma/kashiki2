use rusttype::gpu_cache::Cache;
use rusttype::{point, vector, Font, PositionedGlyph, Rect, Scale};
use std::borrow::Cow;
use std::env;
use std::error::Error;
use stroke_parser::{action_store_parser, Action, ActionStore};

fn layout_paragraph<'a>(
    font: &Font<'a>,
    scale: Scale,
    width: u32,
    text: &str,
) -> Vec<PositionedGlyph<'a>> {
    let mut result = Vec::new();
    let v_metrics = font.v_metrics(scale);
    let advance_height = v_metrics.ascent - v_metrics.descent + v_metrics.line_gap;
    let mut caret = point(0.0, v_metrics.ascent);
    let mut last_glyph_id = None;
    for c in text.chars() {
        if c.is_control() {
            match c {
                '\r' => {
                    caret = point(0.0, caret.y + advance_height);
                }
                '\n' => {}
                _ => {}
            }
            continue;
        }
        let base_glyph = font.glyph(c);
        if let Some(id) = last_glyph_id.take() {
            caret.x += font.pair_kerning(scale, id, base_glyph.id());
        }
        last_glyph_id = Some(base_glyph.id());
        let mut glyph = base_glyph.scaled(scale).positioned(caret);
        if let Some(bb) = glyph.pixel_bounding_box() {
            if bb.max.x > width as i32 {
                caret = point(0.0, caret.y + advance_height);
                glyph.set_position(caret);
                last_glyph_id = None;
            }
        }
        caret.x += glyph.unpositioned().h_metrics().advance_width;
        result.push(glyph);
    }
    result
}

fn main() -> Result<(), String> {
    env_logger::init();

    // setup text editor buffer
    let mut caret = text_buffer::caret::Caret::new(0, 0);
    let mut text_buffer = text_buffer::buffer::Buffer::new();
    let mut store: ActionStore = Default::default();
    let key_setting = include_str!("key-settings.txt");
    let keybinds = action_store_parser::parse_setting(String::from(key_setting));
    keybinds
        .iter()
        .for_each(|k| store.register_keybind(k.clone()));

    // Open window and create a surface
    let event_loop = winit::event_loop::EventLoop::new();

    let window = winit::window::WindowBuilder::new()
        .with_resizable(true)
        .build(&event_loop)
        .unwrap();

    let surface = wgpu::Surface::create(&window);

    // Initialize GPU
    let (device, queue) = futures::executor::block_on(async {
        let adapter = wgpu::Adapter::request(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
            },
            wgpu::BackendBit::all(),
        )
        .await
        .expect("Request adapter");

        adapter
            .request_device(&wgpu::DeviceDescriptor {
                extensions: wgpu::Extensions {
                    anisotropic_filtering: false,
                },
                limits: wgpu::Limits { max_bind_groups: 1 },
            })
            .await
    });

    // Prepare swap chain
    let render_format = wgpu::TextureFormat::Bgra8UnormSrgb;
    let mut size = window.inner_size();

    let mut swap_chain = device.create_swap_chain(
        &surface,
        &wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: render_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox,
        },
    );

    // Prepare font.
    let inconsolata: &[u8] = include_bytes!("TakaoGothic.ttf");
    let font = Font::from_bytes(inconsolata as &[u8]).unwrap();

    let scale = window.scale_factor();

    let (cache_width, cache_height) = ((1024.0 * scale) as u32, (1024.0 * scale) as u32);
    let mut cache: Cache<'static> = Cache::builder()
        .dimensions(cache_width, cache_height)
        .build();

    // Render loop
    window.request_redraw();

    event_loop.run(move |event, _, control_flow| {
        match store.winit_event_to_action(&event) {
            Some(Action::Command(category, name)) if *category == "system" => {
                println!("{:?}:{:?}", category, name);
                match &*name.to_string() {
                    "exit" => {
                        *control_flow = winit::event_loop::ControlFlow::Exit;
                        return;
                    }
                    "return" => text_buffer.insert_enter(&mut caret),
                    "backspace" => {
                        text_buffer.backspace(&mut caret);
                    }
                    "delete" => {
                        text_buffer.delete(&mut caret);
                    }
                    "previous" => {
                        text_buffer.previous(&mut caret);
                    }
                    "next" => {
                        text_buffer.next(&mut caret);
                    }
                    "back" => {
                        text_buffer.back(&mut caret);
                    }
                    "forward" => {
                        text_buffer.forward(&mut caret);
                    }
                    _ => (),
                }
                window.request_redraw();
            }
            Some(Action::Keytype(c)) => {
                text_buffer.insert_char(&mut caret, c);
                let glyph = font
                    .glyph(c)
                    .scaled(Scale::uniform(32.0))
                    .positioned(point(0.0, 0.0));
                cache.queue_glyph(0, glyph); // font_id is always 0. because i use single font.
                window.request_redraw();
            }
            Some(command) => println!("{:?}", command),
            None => {}
        }

        match event {
            winit::event::Event::WindowEvent {
                event: winit::event::WindowEvent::CloseRequested,
                ..
            } => *control_flow = winit::event_loop::ControlFlow::Exit,
            winit::event::Event::WindowEvent {
                event: winit::event::WindowEvent::Resized(new_size),
                ..
            } => {
                size = new_size;

                swap_chain = device.create_swap_chain(
                    &surface,
                    &wgpu::SwapChainDescriptor {
                        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                        format: render_format,
                        width: size.width,
                        height: size.height,
                        present_mode: wgpu::PresentMode::Mailbox,
                    },
                );
            }
            winit::event::Event::RedrawRequested { .. } => {
                // Get a command encoder for the current frame
                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Redraw"),
                });

                let create_glyph_texture_result = cache.cache_queued(|rect, data| {
                    let texture_extent = wgpu::Extent3d {
                        width: rect.width(),
                        height: rect.height(),
                        depth: 1,
                    };
                    let texture = device.create_texture(&wgpu::TextureDescriptor {
                        size: texture_extent,
                        array_layer_count: 1,
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8UnormSrgb,
                        usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
                        label: None,
                    });
                    let texture_view = texture.create_default_view();
                    let temp_buf =
                        device.create_buffer_with_data(data, wgpu::BufferUsage::COPY_SRC);
                    encoder.copy_buffer_to_texture(
                        wgpu::BufferCopyView {
                            buffer: &temp_buf,
                            offset: 0,
                            bytes_per_row: 4 * rect.width(),
                            rows_per_image: 0,
                        },
                        wgpu::TextureCopyView {
                            texture: &texture,
                            mip_level: 0,
                            array_layer: 0,
                            origin: wgpu::Origin3d::ZERO,
                        },
                        texture_extent,
                    );
                    println!("create texture.");
                });

                match create_glyph_texture_result {
                    Ok(_) => (),
                    Err(e) => println!("{}", e),
                }

                // Get the next frame
                let frame = swap_chain.get_next_texture().expect("Get next frame");

                // Clear frame
                {
                    let _ = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                            attachment: &frame.view,
                            resolve_target: None,
                            load_op: wgpu::LoadOp::Clear,
                            store_op: wgpu::StoreOp::Store,
                            clear_color: wgpu::Color {
                                r: 0.4,
                                g: 0.4,
                                b: 0.4,
                                a: 1.0,
                            },
                        }],
                        depth_stencil_attachment: None,
                    });
                }

                // ここから書いていく処理が必要なはず。
                encoder.finish();
            }
            _ => {
                *control_flow = winit::event_loop::ControlFlow::Wait;
            }
        }
    })
}
