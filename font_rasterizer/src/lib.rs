use std::{collections::BTreeMap, iter};

use camera::CameraOperation;
use cgmath::Rotation3;
use font_vertex::FontVertex;
use instances::{Instance, Instances};
use log::{debug, info};
use rasterizer_pipeline::RasterizerPipeline;
use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

mod camera;
mod font_vertex;
mod instances;
mod outline_bind_group;
mod overlap_bind_group;
mod rasterizer_pipeline;
mod screen_bind_group;
mod screen_texture;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ScreenVertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl ScreenVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ScreenVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

const SCREEN_VERTICES: &[ScreenVertex] = &[
    ScreenVertex {
        position: [-1.0, -1.0, 0.0],
        tex_coords: [0.0, 1.0],
    }, // A
    ScreenVertex {
        position: [1.0, -1.0, 0.0],
        tex_coords: [1.0, 1.0],
    }, // B
    ScreenVertex {
        position: [-1.0, 1.0, 0.0],
        tex_coords: [0.0, 0.0],
    }, // C
    ScreenVertex {
        position: [1.0, 1.0, 0.0],
        tex_coords: [1.0, 0.0],
    }, // D
];

const SCREEN_INDICES: &[u16] = &[0, 1, 2, 2, 1, 3];

struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,

    camera: camera::Camera,
    camera_controller: camera::CameraController,

    rasterizer_pipeline: RasterizerPipeline,

    overlap_vertex_buffer: wgpu::Buffer,
    overlap_index_buffer: wgpu::Buffer,
    overlap_num_indices: u32,

    outline_vertex_buffer: wgpu::Buffer,
    outline_index_buffer: wgpu::Buffer,
    outline_num_indices: u32,

    screen_vertex_buffer: wgpu::Buffer,
    screen_index_buffer: wgpu::Buffer,
    screen_num_indices: u32,

    font_vertex: (Vec<FontVertex>, BTreeMap<char, Vec<u32>>),
    instances: Instances,
}

impl State {
    async fn new(window: &Window) -> Self {
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
        let camera = camera::Camera::new(
            (0.0, 1.0, 2.0).into(),
            (0.0, 0.0, 0.0).into(),
            cgmath::Vector3::unit_y(),
            config.width as f32 / config.height as f32,
            // fovy は視野角。ここでは45度を指定
            45.0,
            0.1,
            100.0,
        );
        let camera_controller = camera::CameraController::new(0.2);

        let rasterizer_pipeline = RasterizerPipeline::new(
            &device,
            size.width,
            size.height,
            config.format,
            rasterizer_pipeline::Quarity::High,
        );

        let font_vertex = match FontVertex::new_chars(vec![
            0x20 as char..=0x7e as char,
            /* ひらがな */ '\u{3040}'..='\u{309F}',
            /* カタカナ */ '\u{30A0}'..='\u{30FF}',
        ]) {
            Ok(font_vertex) => font_vertex,
            e => {
                info!("err:{:?}", e);
                std::process::exit(2)
            }
        };

        let mut is = Vec::new();

        for x in -50..50 {
            for y in -50..50 {
                let instance = Instance::new(
                    cgmath::Vector3 {
                        x: 1.2 * x as f32,
                        y: 1.2 * y as f32,
                        z: 0.0,
                    },
                    cgmath::Quaternion::from_axis_angle(
                        cgmath::Vector3::unit_z(),
                        cgmath::Deg(0.0),
                    ),
                );
                is.push(instance);
            }
        }
        let instances = Instances::new(is);

        let overlap_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Overlap Vertex Buffer"),
            contents: bytemuck::cast_slice(&font_vertex.0),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let idx = font_vertex.1.get(&'あ').unwrap();
        let overlap_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Overlap Index Buffer"),
            contents: bytemuck::cast_slice(idx),
            usage: wgpu::BufferUsages::INDEX,
        });
        let overlap_num_indices = idx.len() as u32;

        let outline_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Outline Vertex Buffer"),
            contents: bytemuck::cast_slice(SCREEN_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let outline_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Outline Index Buffer"),
            contents: bytemuck::cast_slice(SCREEN_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let outline_num_indices = SCREEN_INDICES.len() as u32;

        // screen
        let screen_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Outline Vertex Buffer"),
            contents: bytemuck::cast_slice(SCREEN_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let screen_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Outline Index Buffer"),
            contents: bytemuck::cast_slice(SCREEN_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let screen_num_indices = SCREEN_INDICES.len() as u32;

        Self {
            surface,
            device,
            queue,
            config,
            size,

            camera,
            camera_controller,
            rasterizer_pipeline,

            overlap_vertex_buffer,
            overlap_index_buffer,
            overlap_num_indices,

            outline_vertex_buffer,
            outline_index_buffer,
            outline_num_indices,

            screen_vertex_buffer,
            screen_index_buffer,
            screen_num_indices,

            font_vertex,
            instances,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.rasterizer_pipeline = RasterizerPipeline::new(
                &self.device,
                new_size.width,
                new_size.height,
                self.config.format,
                rasterizer_pipeline::Quarity::High,
            )
        }
    }

    #[allow(unused_variables)]
    fn input(&mut self, event: &WindowEvent) -> bool {
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

    fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.camera);
        self.rasterizer_pipeline
            .overlap_bind_group
            .update(self.camera.build_view_projection_matrix().into());
        self.camera_controller.reset_state();
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let overlap_bind_group = &self
                .rasterizer_pipeline
                .overlap_bind_group
                .to_bind_group(&self.device);

            let instance_buffer = self.instances.to_wgpu_buffer(&self.device);

            {
                let mut overlay_render_pass =
                    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Overlay Render Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &self.rasterizer_pipeline.overlap_texture.view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.0,
                                    g: 0.0,
                                    b: 0.0,
                                    a: 0.0,
                                }),
                                store: true,
                            },
                        })],
                        depth_stencil_attachment: None,
                    });

                overlay_render_pass.set_pipeline(&self.rasterizer_pipeline.overlap_render_pipeline);
                overlay_render_pass.set_bind_group(0, overlap_bind_group, &[]);
                overlay_render_pass.set_vertex_buffer(0, self.overlap_vertex_buffer.slice(..));
                overlay_render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
                overlay_render_pass.set_index_buffer(
                    self.overlap_index_buffer.slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                overlay_render_pass.draw_indexed(
                    0..self.overlap_num_indices,
                    0,
                    0..self.instances.len() as _,
                );
            }
        }

        {
            let outline_view = self
                .rasterizer_pipeline
                .outline_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            let outline_bind_group = &self
                .rasterizer_pipeline
                .outline_bind_group
                .to_bind_group(&self.device, &self.rasterizer_pipeline.overlap_texture);
            {
                let mut outline_render_pass =
                    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Render Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &outline_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.0,
                                    g: 0.0,
                                    b: 0.0,
                                    a: 0.0,
                                }),
                                store: true,
                            },
                        })],
                        depth_stencil_attachment: None,
                    });
                outline_render_pass.set_pipeline(&self.rasterizer_pipeline.outline_render_pipeline);
                outline_render_pass.set_bind_group(0, outline_bind_group, &[]);
                outline_render_pass.set_vertex_buffer(0, self.outline_vertex_buffer.slice(..));
                outline_render_pass.set_index_buffer(
                    self.outline_index_buffer.slice(..),
                    wgpu::IndexFormat::Uint16,
                );
                outline_render_pass.draw_indexed(0..self.outline_num_indices, 0, 0..1);
            }
        }

        // screen
        let screen_output = self.surface.get_current_texture()?;
        let screen_view = screen_output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let screen_bind_group = &self
            .rasterizer_pipeline
            .default_screen_bind_group
            .to_bind_group(&self.device, &self.rasterizer_pipeline.outline_texture);
        // Screen Render Pass
        {
            let mut screen_render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Screen Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &screen_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
            screen_render_pass
                .set_pipeline(&self.rasterizer_pipeline.default_screen_render_pipeline);
            screen_render_pass.set_bind_group(0, screen_bind_group, &[]);
            screen_render_pass.set_vertex_buffer(0, self.screen_vertex_buffer.slice(..));
            screen_render_pass.set_index_buffer(
                self.screen_index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );
            screen_render_pass.draw_indexed(0..self.screen_num_indices, 0, 0..1);
        }

        self.queue.submit(iter::once(encoder.finish()));
        screen_output.present();

        Ok(())
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Warn).expect("Could't initialize logger");
        } else {
            env_logger::init();
        }
    }

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    #[cfg(target_arch = "wasm32")]
    {
        // Winit prevents sizing with CSS, so we have to set
        // the size manually when on web.
        use winit::dpi::PhysicalSize;
        window.set_inner_size(PhysicalSize::new(450, 400));

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

    // State::new uses async code, so we're going to wait for it to finish
    let mut state = State::new(&window).await;

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                if !state.input(event) {
                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        } => *control_flow = ControlFlow::Exit,
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
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                state.update();
                match state.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if it's lost or outdated
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        state.resize(state.size)
                    }
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
