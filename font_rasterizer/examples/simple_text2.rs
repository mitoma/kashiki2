use font_collector::FontCollector;
use stroke_parser::{action_store_parser::parse_setting, Action, ActionStore};
use text_buffer::action::EditorOperation;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use font_rasterizer::{
    camera::{Camera, CameraAdjustment, CameraOperation},
    color_theme::ColorTheme,
    context::{StateContext, WindowSize},
    font_buffer::GlyphVertexBuffer,
    instances::GlyphInstances,
    layout_engine::{HorizontalWorld, ModelOperation, World},
    rasterizer_pipeline::Quarity,
    support::{run_support, Flags, InputResult, SimpleStateCallback, SimpleStateSupport},
    ui::{ime_input::ImeInput, textedit::TextEdit},
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
        window_icon: None,
        window_title: "Hello".to_string(),
        window_size: (800, 600),
        callback: Box::new(callback),
        quarity: Quarity::VeryHigh,
        color_theme: ColorTheme::SolarizedDark,
        flags: Flags::DEFAULT,
        font_binaries,
    };
    run_support(support).await;
}

struct SingleCharCallback {
    store: ActionStore,
    world: Box<dyn World>,
    ime: ImeInput,
}

impl SingleCharCallback {
    fn new() -> Self {
        let mut store: ActionStore = Default::default();
        let key_setting = include_str!("key-settings.txt");
        info!("{}", key_setting);
        let keybinds = parse_setting(key_setting);
        keybinds
            .iter()
            .for_each(|k| store.register_keybind(k.clone()));
        let ime = ImeInput::new();

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

        Self { store, world, ime }
    }
}

impl SimpleStateCallback for SingleCharCallback {
    fn init(&mut self, glyph_vertex_buffer: &mut GlyphVertexBuffer, context: &StateContext) {
        self.world.look_at(0, CameraAdjustment::FitBoth);
        self.update(glyph_vertex_buffer, context);
    }

    fn resize(&mut self, window_size: WindowSize) {
        self.world.change_window_size(window_size);
    }

    fn update(&mut self, glyph_vertex_buffer: &mut GlyphVertexBuffer, context: &StateContext) {
        self.world.update(glyph_vertex_buffer, context);
        self.ime.update(
            &context.color_theme,
            glyph_vertex_buffer,
            &context.device,
            &context.queue,
        );
    }

    fn input(
        &mut self,
        _glyph_vertex_buffer: &GlyphVertexBuffer,
        event: &WindowEvent,
    ) -> InputResult {
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
                self.world.editor_operation(&action);
                InputResult::InputConsumed
            }
            Some(Action::Command(category, name)) if *category == "world" => {
                info!("world:");

                match &*name.to_string() {
                    "left" => self.world.look_prev(CameraAdjustment::FitBoth),
                    "right" => self.world.look_next(CameraAdjustment::FitBoth),
                    "forward" => self.world.camera_operation(CameraOperation::Forward),
                    "back" => self.world.camera_operation(CameraOperation::Backward),
                    "change-direction" => {
                        info!("change direction");
                        self.world.model_operation(&ModelOperation::ChangeDirection)
                    }
                    _ => {}
                };
                InputResult::InputConsumed
            }
            Some(Action::Command(_, _)) => InputResult::Noop,
            Some(Action::Keytype(c)) => {
                let action = EditorOperation::InsertChar(c);
                self.world.editor_operation(&action);
                InputResult::InputConsumed
            }
            Some(Action::ImeInput(value)) => {
                self.ime.apply_ime_event(&Action::ImeInput(value.clone()));
                self.world
                    .editor_operation(&EditorOperation::InsertString(value));
                InputResult::InputConsumed
            }
            Some(Action::ImePreedit(value, position)) => {
                self.ime
                    .apply_ime_event(&Action::ImePreedit(value, position));
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
