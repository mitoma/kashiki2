use font_rusterizer::{
    default_state::SimpleStateCallback,
    instances::GlyphInstances,
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
    let callback = Pog {};
    let support = SimpleStateSupport {
        window_title: "Hello".to_string(),
        window_size: (800, 600),
        callback: Box::new(callback),
        quarity: Quarity::High,
    };
    run_support(support).await;
}

struct Pog;
impl SimpleStateCallback for Pog {
    fn input(&mut self, _event: &WindowEvent) -> bool {
        println!("call input");
        false
    }

    fn render(&mut self) -> ([[f32; 4]; 4], &[&GlyphInstances]) {
        
        println!("call render");
        ([[0.0; 4]; 4], &[])
    }
}
