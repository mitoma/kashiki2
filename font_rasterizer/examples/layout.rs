use stroke_parser::{action_store_parser::parse_setting, Action, ActionStore};
use text_buffer::{action::EditorOperation, editor::Editor};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use font_rasterizer::{
    camera::{Camera, CameraController},
    color_theme::ColorTheme::{self, SolarizedDark},
    font_buffer::GlyphVertexBuffer,
    instances::GlyphInstances,
    layout_engine::{HorizontalWorld, World},
    motion::{EasingFuncType, MotionDetail, MotionFlags, MotionTarget, MotionType},
    rasterizer_pipeline::Quarity,
    support::{run_support, Flags, InputResult, SimpleStateCallback, SimpleStateSupport},
    ui::{split_preedit_string, PlaneTextReader, SingleLineComponent},
};
use log::info;
use winit::event::WindowEvent;

const FONT_DATA: &[u8] = include_bytes!("font/HackGenConsole-Regular.ttf");
const EMOJI_FONT_DATA: &[u8] = include_bytes!("font/NotoEmoji-Regular.ttf");

pub fn main() {
    std::env::set_var("RUST_LOG", "simple_text=debug");
    pollster::block_on(run());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    let font_binaries = vec![FONT_DATA.to_vec(), EMOJI_FONT_DATA.to_vec()];

    let callback = SingleCharCallback::new();
    let support = SimpleStateSupport {
        window_title: "Hello".to_string(),
        window_size: (800, 600),
        callback: Box::new(callback),
        quarity: Quarity::High,
        bg_color: SolarizedDark.background().into(),
        flags: Flags::DEFAULT,
        font_binaries,
    };
    run_support(support).await;
}

struct SingleCharCallback {
    world: Box<dyn World>,
    color_theme: ColorTheme,
    store: ActionStore,
    editor: Editor,
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

        let world = Box::new(HorizontalWorld::new(800, 600));
        let model = Box::new(PlaneTextReader::new("ジョブ次郎".to_string()));
        world.add(model);

        Self {
            world,
            color_theme: ColorTheme::SolarizedDark,
            editor: text_buffer::editor::Editor::default(),
            store,
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
        glyph_vertex_buffer
            .append_glyph(device, queue, "ジョブ次郎".chars().collect())
            .unwrap();
    }

    fn resize(&mut self, width: u32, height: u32) {
        // TODO update world aspect
    }

    fn update(
        &mut self,
        glyph_vertex_buffer: &mut GlyphVertexBuffer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
    }

    fn input(&mut self, event: &WindowEvent) -> InputResult {
        let input_result = match self.store.winit_window_event_to_action(event) {
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
                self.editor.operation(&action);
                InputResult::InputConsumed
            }
            Some(Action::Command(_, _)) => InputResult::Noop,
            Some(Action::Keytype(c)) => {
                let action = EditorOperation::InsertChar(c);
                self.editor.operation(&action);
                InputResult::InputConsumed
            }
            Some(Action::ImeInput(value)) => {
                //self.ime.update_value("".to_string());
                self.editor.operation(&EditorOperation::InsertString(value));
                InputResult::InputConsumed
            }
            Some(Action::ImePreedit(value, position)) => {
                match position {
                    Some((start, end)) if start != end => {
                        info!("start:{start}, end:{end}");
                        let (first, center, last) = split_preedit_string(value, start, end);
                        let preedit_str = format!("{}[{}]{}", first, center, last);
                        //self.ime.update_value(preedit_str);
                    }
                    _ => {
                        //self.ime.update_value(value);
                    }
                }
                InputResult::InputConsumed
            }
            Some(_) => InputResult::Noop,
            None => InputResult::Noop,
        };
        input_result
    }

    fn render(&mut self) -> (&Camera, Vec<&GlyphInstances>) {
        let instances = self.reader.get_instances();
        let ime_instances = self.ime.get_instances();
        let mut v = Vec::new();
        for i in instances {
            v.push(i);
        }
        for i in ime_instances {
            v.push(i);
        }
        (&self.camera, v)
    }
}
