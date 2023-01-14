use cgmath::Rotation3;
use font_rusterizer::{
    camera::Camera,
    default_state::SimpleStateCallback,
    instances::{GlyphInstance, GlyphInstances},
    rasterizer_pipeline::Quarity,
    support::{run_support, SimpleStateSupport},
};
use winit::event::WindowEvent;

pub fn main() {
    std::env::set_var("RUST_LOG", "font_rusterizer=debug");
    pollster::block_on(run());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    let callback = Pog::default();
    let support = SimpleStateSupport {
        window_title: "Hello".to_string(),
        window_size: (800, 600),
        callback: Box::new(callback),
        quarity: Quarity::VeryHigh,
    };
    run_support(support).await;
}

#[derive(Default)]
struct Pog(Vec<GlyphInstances>);

impl SimpleStateCallback for Pog {
    fn init(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let value = GlyphInstance::new(
            (0.0, 0.0, 0.0).into(),
            cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)),
            [1.0, 1.0, 1.0],
        );
        let mut instance = GlyphInstances::new('あ', Vec::new(), device);
        instance.push(value);
        self.0 = vec![instance];
    }

    fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.0
            .iter_mut()
            .for_each(|i| i.update_buffer(device, queue));
    }

    fn input(&mut self, _event: &WindowEvent) -> bool {
        false
    }

    fn render(&mut self) -> ([[f32; 4]; 4], Vec<&GlyphInstances>) {
        let camera = Camera::new(
            (0.0, 0.0, 1.0).into(),
            (0.0, 0.0, 0.0).into(),
            cgmath::Vector3::unit_y(),
            800 as f32 / 600 as f32,
            // fovy は視野角。ここでは45度を指定
            45.0,
            0.1,
            200.0,
        );
        (
            camera.build_view_projection_matrix().into(),
            self.0.iter().collect(),
        )
    }
}
