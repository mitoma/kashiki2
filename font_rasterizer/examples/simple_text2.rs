use font_collector::FontCollector;
use stroke_parser::{action_store_parser::parse_setting, Action, ActionStore};
use text_buffer::action::EditorOperation;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use font_rasterizer::{
    camera::{Camera, CameraAdjustment, CameraOperation},
    color_theme::ColorTheme::{self, SolarizedDark},
    font_buffer::GlyphVertexBuffer,
    instances::GlyphInstances,
    layout_engine::{HorizontalWorld, World},
    motion::{CameraDetail, EasingFuncType, MotionDetail, MotionFlags, MotionTarget, MotionType},
    rasterizer_pipeline::Quarity,
    support::{run_support, Flags, InputResult, SimpleStateCallback, SimpleStateSupport},
    ui::{split_preedit_string, textedit::TextEdit, SingleLineComponent},
};
use log::info;
use winit::event::WindowEvent;

const FONT_DATA: &[u8] = include_bytes!("font/HackGenConsole-Regular.ttf");
const EMOJI_FONT_DATA: &[u8] = include_bytes!("font/NotoEmoji-Regular.ttf");

pub fn main() {
    std::env::set_var("RUST_LOG", "simple_text=debug");
    //std::env::set_var("FONT_RASTERIZER_DEBUG", "debug");
    pollster::block_on(run());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    let mut collector = FontCollector::default();
    collector.add_system_fonts();
    let data = collector.load_font("UD デジタル 教科書体 N-R");

    let font_binaries = vec![
        data.unwrap(),
        collector.convert_font(FONT_DATA.to_vec(), None).unwrap(),
        collector
            .convert_font(EMOJI_FONT_DATA.to_vec(), None)
            .unwrap(),
    ];

    let callback = SingleCharCallback::new();
    let support = SimpleStateSupport {
        window_title: "Hello".to_string(),
        window_size: (800, 600),
        callback: Box::new(callback),
        quarity: Quarity::VeryHigh,
        bg_color: SolarizedDark.background().into(),
        flags: Flags::DEFAULT,
        font_binaries,
    };
    run_support(support).await;
}

struct SingleCharCallback {
    color_theme: ColorTheme,
    store: ActionStore,
    world: Box<dyn World>,
    ime: SingleLineComponent,
}

impl SingleCharCallback {
    fn new() -> Self {
        let mut store: ActionStore = Default::default();
        let key_setting = include_str!("key-settings.txt");
        info!("{}", key_setting);
        let keybinds = parse_setting(String::from(key_setting));
        keybinds
            .iter()
            .for_each(|k| store.register_keybind(k.clone()));
        let mut ime = SingleLineComponent::new("".to_string());
        ime.update_motion(
            MotionFlags::builder()
                .camera_detail(CameraDetail::IGNORE_CAMERA)
                .motion_type(MotionType::EaseOut(EasingFuncType::Sin, false))
                .motion_detail(MotionDetail::TO_CURRENT)
                .motion_target(MotionTarget::STRETCH_X_PLUS)
                .build(),
        );
        ime.update_scale(0.1);

        let mut world = Box::new(HorizontalWorld::new(800, 600));
        let model = Box::new(TextEdit::default());
        world.add(model);
        let model = Box::new(TextEdit::default());
        world.add(model);
        let model = Box::new(TextEdit::default());
        world.add(model);
        let model = Box::new(TextEdit::default());
        world.add(model);
        let look_at = 0;
        world.look_at(look_at, CameraAdjustment::FitBoth);
        world.re_layout();

        Self {
            color_theme: ColorTheme::SolarizedDark,
            store,
            world,
            ime,
        }
    }
}

impl SimpleStateCallback for SingleCharCallback {
    fn init(
        &mut self,
        glyph_vertex_buffer: &mut GlyphVertexBuffer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        self.world.look_at(0, CameraAdjustment::FitBoth);
        self.update(glyph_vertex_buffer, device, queue);
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.world.change_window_size((width, height));
    }

    fn update(
        &mut self,
        glyph_vertex_buffer: &mut GlyphVertexBuffer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        self.world
            .update(&self.color_theme, glyph_vertex_buffer, &device, &queue);
        glyph_vertex_buffer
            .append_glyph(device, queue, self.ime.value.chars().collect())
            .unwrap();
        self.ime
            .generate_instances(&self.color_theme, glyph_vertex_buffer, device, queue);
    }

    fn input(&mut self, event: &WindowEvent) -> InputResult {
        match self.store.winit_window_event_to_action(event) {
            Some(Action::Command(category, name)) if *category == "system" => {
                let action = match &*name.to_string() {
                    "exit" => return InputResult::SendExit,
                    "return" => EditorOperation::InsertEnter,
                    "backspace" => EditorOperation::Backspace,
                    "delete" => EditorOperation::Delete,
                    "previous" => EditorOperation::Previous,
                    "next" => EditorOperation::Next,
                    "back" => EditorOperation::Back,
                    "forward" => EditorOperation::Forward,
                    "head" => EditorOperation::Head,
                    "last" => EditorOperation::Last,
                    "undo" => EditorOperation::Undo,
                    _ => EditorOperation::Noop,
                };
                self.world.operation(&action);
                InputResult::InputConsumed
            }
            Some(Action::Command(category, name)) if *category == "world" => {
                info!("world:");

                match &*name.to_string() {
                    "left" => self.world.look_prev(CameraAdjustment::FitBoth),
                    "right" => self.world.look_next(CameraAdjustment::FitBoth),
                    "forward" => self.world.camera_operation(CameraOperation::Forward),
                    "back" => self.world.camera_operation(CameraOperation::Backward),
                    _ => {}
                };
                InputResult::InputConsumed
            }
            Some(Action::Command(_, _)) => InputResult::Noop,
            Some(Action::Keytype(c)) => {
                let action = EditorOperation::InsertChar(c);
                self.world.operation(&action);
                InputResult::InputConsumed
            }
            Some(Action::ImeInput(value)) => {
                self.ime.update_value("".to_string());
                self.world.operation(&EditorOperation::InsertString(value));
                InputResult::InputConsumed
            }
            Some(Action::ImePreedit(value, position)) => {
                match position {
                    Some((start, end)) if start != end => {
                        info!("start:{start}, end:{end}");
                        let (first, center, last) = split_preedit_string(value, start, end);
                        let preedit_str = format!("{}[{}]{}", first, center, last);
                        self.ime.update_value(preedit_str);
                    }
                    _ => {
                        self.ime.update_value(value);
                    }
                }
                InputResult::InputConsumed
            }
            Some(_) => InputResult::Noop,
            None => InputResult::Noop,
        }
    }

    fn render(&mut self) -> (&Camera, Vec<&GlyphInstances>) {
        let mut world_instances = self.world.glyph_instances();
        let mut ime_instances = self.ime.get_instances();
        world_instances.append(&mut ime_instances);
        (self.world.camera(), world_instances)
    }
}