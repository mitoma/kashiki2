use stroke_parser::{action_store_parser, Action, ActionStore};
use wgpu_glyph::{GlyphBrushBuilder, Scale, Section};

fn main() -> Result<(), String> {
    env_logger::init();

    let mut caret = text_buffer::caret::Caret::new(0, 0);
    let mut text_buffer = text_buffer::buffer::Buffer::new();
    let mut store: ActionStore = Default::default();
    let key_setting = include_str!("key-settings.txt");
    println!("{}", key_setting);
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

    // Prepare glyph_brush
    let inconsolata: &[u8] = include_bytes!("TakaoGothic.ttf");
    let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(inconsolata)
        .expect("Load fonts")
        .build(&device, render_format);

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

                glyph_brush.queue(Section {
                    text: &text_buffer.to_buffer_string(),
                    screen_position: (30.0, 30.0),
                    color: [0.0, 0.0, 0.0, 1.0],
                    scale: Scale { x: 40.0, y: 40.0 },
                    bounds: (size.width as f32, size.height as f32),
                    ..Section::default()
                });

                // Draw the text!
                glyph_brush
                    .draw_queued(&device, &mut encoder, &frame.view, size.width, size.height)
                    .expect("Draw queued");

                queue.submit(&[encoder.finish()]);
            }
            _ => {
                *control_flow = winit::event_loop::ControlFlow::Wait;
            }
        }
    })
}
