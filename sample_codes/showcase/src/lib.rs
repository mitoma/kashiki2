use std::{
    collections::HashSet,
    sync::{mpsc::Sender, LazyLock, Mutex},
};

use font_collector::FontCollector;
use stroke_parser::{action_store_parser::parse_setting, Action, ActionArgument, ActionStore};
use text_buffer::action::EditorOperation;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use font_rasterizer::{
    camera::{Camera, CameraAdjustment, CameraOperation},
    color_theme::ColorTheme,
    context::{StateContext, TextContext, WindowSize},
    font_buffer::{Direction, GlyphVertexBuffer},
    instances::GlyphInstances,
    layout_engine::{HorizontalWorld, Model, ModelOperation, World},
    rasterizer_pipeline::Quarity,
    support::{run_support, Flags, InputResult, SimpleStateCallback, SimpleStateSupport},
    ui::{caret_char, ImeInput, TextEdit},
};
use winit::event::WindowEvent;

const FONT_DATA: &[u8] =
    include_bytes!("../../../font_rasterizer/examples/font/BIZUDMincho-Regular.ttf");
const EMOJI_FONT_DATA: &[u8] =
    include_bytes!("../../../font_rasterizer/examples/font/NotoEmoji-Regular.ttf");

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    let collector = FontCollector::default();
    let font_binaries = vec![
        collector.convert_font(FONT_DATA.to_vec(), None).unwrap(),
        collector
            .convert_font(EMOJI_FONT_DATA.to_vec(), None)
            .unwrap(),
    ];

    let window_size = WindowSize::new(1024, 768);
    let callback = SingleCharCallback::new(window_size);
    let support = SimpleStateSupport {
        window_icon: None,
        window_title: "Hello".to_string(),
        window_size,
        callback: Box::new(callback),
        quarity: Quarity::VeryHigh,
        color_theme: ColorTheme::SolarizedDark,
        flags: Flags::DEFAULT,
        font_binaries,
        performance_mode: false,
    };
    run_support(support).await;
}

static ACTION_FROM_JS: LazyLock<Mutex<Option<Sender<Action>>>> = LazyLock::new(|| Mutex::new(None));

fn set_action_sender(sender: Sender<Action>) {
    ACTION_FROM_JS.lock().unwrap().replace(sender);
}

fn send_action(action: Action) {
    match ACTION_FROM_JS.lock().unwrap().as_ref() {
        Some(sender) => sender.send(action).unwrap(),
        None => log::warn!("Action sender is not set"),
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn toggle_direction() {
    send_action(Action::new_command("world", "change-direction"));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn look_current_and_centering() {
    send_action(Action::new_command("world", "look-current-and-centering"));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn change_theme_dark() {
    send_action(Action::new_command_with_argument(
        "system",
        "change-theme",
        "dark",
    ));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn change_theme_light() {
    send_action(Action::new_command_with_argument(
        "system",
        "change-theme",
        "light",
    ));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn zoom_in() {
    send_action(Action::new_command("world", "forward"));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn zoom_out() {
    send_action(Action::new_command("world", "back"));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn toggle_psychedelic() {
    send_action(Action::new_command("world", "toggle-psychedelic"));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn send_log(message: &str) {
    log::warn!("{}", message);
}

enum SystemActionResult {
    ChangeColorTheme(ColorTheme),
    ChangeGlobalDirection(Direction),
    Noop,
}

struct SingleCharCallback {
    world: HorizontalWorld,
    store: ActionStore,
    ime: ImeInput,
    new_chars: HashSet<char>,
}

impl SingleCharCallback {
    fn new(window_size: WindowSize) -> Self {
        let mut store: ActionStore = Default::default();
        let key_setting = include_str!("../asset/key-settings.txt");
        parse_setting(key_setting)
            .into_iter()
            .for_each(|k| store.register_keybind(k));

        let mut world = HorizontalWorld::new(window_size);
        let mut textedit = TextEdit::new(TextContext::default().with_max_col(40));

        textedit.editor_operation(&EditorOperation::InsertString(
            include_str!("../asset/initial.txt").to_string(),
        ));
        world.add(Box::new(textedit));
        world.look_current(CameraAdjustment::FitBothAndCentering);
        let ime = ImeInput::new();
        let mut new_chars = HashSet::new();
        // キャレットのグリフを追加する
        new_chars.insert(caret_char(text_buffer::caret::CaretType::Primary));
        new_chars.insert(caret_char(text_buffer::caret::CaretType::Mark));

        Self {
            world,
            store,
            ime,
            new_chars,
        }
    }

    fn execute_system_action(
        &mut self,
        command_name: &str,
        argument: ActionArgument,
        context: &StateContext,
    ) -> SystemActionResult {
        match command_name {
            "change-theme" => match argument {
                ActionArgument::String(value) => match &*value.to_string() {
                    "black" => SystemActionResult::ChangeColorTheme(ColorTheme::SolarizedBlackback),
                    "dark" => SystemActionResult::ChangeColorTheme(ColorTheme::SolarizedDark),
                    "light" => SystemActionResult::ChangeColorTheme(ColorTheme::SolarizedLight),
                    _ => SystemActionResult::Noop,
                },
                _ => SystemActionResult::Noop,
            },
            "change-global-direction" => {
                SystemActionResult::ChangeGlobalDirection(context.global_direction.toggle())
            }
            _ => SystemActionResult::Noop,
        }
    }

    fn execute_editor_action(command_name: &str) -> EditorOperation {
        match command_name {
            "return" => EditorOperation::InsertEnter,
            "backspace" => EditorOperation::Backspace,
            "backspace-word" => EditorOperation::BackspaceWord,
            "delete" => EditorOperation::Delete,
            "delete-word" => EditorOperation::DeleteWord,
            "previous" => EditorOperation::Previous,
            "next" => EditorOperation::Next,
            "back" => EditorOperation::Back,
            "forward" => EditorOperation::Forward,
            "back-word" => EditorOperation::BackWord,
            "forward-word" => EditorOperation::ForwardWord,
            "head" => EditorOperation::Head,
            "last" => EditorOperation::Last,
            "undo" => EditorOperation::Undo,
            "buffer-head" => EditorOperation::BufferHead,
            "buffer-last" => EditorOperation::BufferLast,
            "mark" => EditorOperation::Mark,
            "unmark" => EditorOperation::UnMark,
            _ => EditorOperation::Noop,
        }
    }

    fn execute_world_action(
        &mut self,
        command_name: &str,
        argument: ActionArgument,
        context: &StateContext,
    ) {
        match command_name {
            "reset-zoom" => self.world.look_current(CameraAdjustment::FitBoth),
            "look-current-and-centering" => self
                .world
                .look_current(CameraAdjustment::FitBothAndCentering),
            "look-current" => self.world.look_current(CameraAdjustment::NoCare),
            "look-next" => self.world.look_next(CameraAdjustment::NoCare),
            "look-prev" => self.world.look_prev(CameraAdjustment::NoCare),
            "swap-next" => self.world.swap_next(),
            "swap-prev" => self.world.swap_prev(),
            "fit-width" => self.world.look_current(CameraAdjustment::FitWidth),
            "fit-height" => self.world.look_current(CameraAdjustment::FitHeight),
            "fit-by-direction" => {
                if context.global_direction == Direction::Horizontal {
                    self.world.look_current(CameraAdjustment::FitWidth)
                } else {
                    self.world.look_current(CameraAdjustment::FitHeight)
                }
            }
            "forward" => self.world.camera_operation(CameraOperation::Forward),
            "back" => self.world.camera_operation(CameraOperation::Backward),
            "change-direction" => self
                .world
                .model_operation(&ModelOperation::ChangeDirection(None)),
            "increase-row-interval" => self
                .world
                .model_operation(&ModelOperation::IncreaseRowInterval),
            "decrease-row-interval" => self
                .world
                .model_operation(&ModelOperation::DecreaseRowInterval),
            "increase-col-interval" => self
                .world
                .model_operation(&ModelOperation::IncreaseColInterval),
            "decrease-col-interval" => self
                .world
                .model_operation(&ModelOperation::DecreaseColInterval),
            "increase-col-scale" => self
                .world
                .model_operation(&ModelOperation::IncreaseColScale),
            "decrease-col-scale" => self
                .world
                .model_operation(&ModelOperation::DecreaseColScale),
            "increase-row-scale" => self
                .world
                .model_operation(&ModelOperation::IncreaseRowScale),
            "decrease-row-scale" => self
                .world
                .model_operation(&ModelOperation::DecreaseRowScale),
            "toggle-psychedelic" => self
                .world
                .model_operation(&ModelOperation::TogglePsychedelic),
            "move-to-click" => {
                match argument {
                    ActionArgument::Point((x, y)) => {
                        let (x_ratio, y_ratio) = (
                            (x / context.window_size.width as f32 * 2.0) - 1.0,
                            1.0 - (y / context.window_size.height as f32 * 2.0),
                        );
                        self.world.move_to_position(x_ratio, y_ratio);
                    }
                    _ => { /* noop */ }
                }
            }
            "move-to-click-with-mark" => {
                match argument {
                    ActionArgument::Point((x, y)) => {
                        let (x_ratio, y_ratio) = (
                            (x / context.window_size.width as f32 * 2.0) - 1.0,
                            1.0 - (y / context.window_size.height as f32 * 2.0),
                        );
                        self.world.move_to_position(x_ratio, y_ratio);
                        self.world.editor_operation(&EditorOperation::Mark);
                    }
                    _ => { /* noop */ }
                }
            }
            _ => {}
        };
    }
}

impl SimpleStateCallback for SingleCharCallback {
    fn init(&mut self, glyph_vertex_buffer: &mut GlyphVertexBuffer, context: &StateContext) {
        set_action_sender(context.action_queue_sender.clone());

        glyph_vertex_buffer
            .append_glyph(&context.device, &context.queue, self.world.chars())
            .unwrap();
        [
            Action::new_command_with_argument("system", "change-theme", "light"),
            Action::new_command("edit", "buffer-head"),
            Action::new_command("world", "back"),
            Action::new_command("world", "back"),
        ]
        .into_iter()
        .for_each(|action| {
            context.action_queue_sender.send(action).unwrap();
        });
    }

    fn update(&mut self, glyph_vertex_buffer: &mut GlyphVertexBuffer, context: &StateContext) {
        // 入力などで新しい char が追加されたら、グリフバッファに追加する
        if !self.new_chars.is_empty() {
            let new_chars = self.new_chars.clone();
            glyph_vertex_buffer
                .append_glyph(&context.device, &context.queue, new_chars)
                .unwrap();
            self.new_chars.clear();
        }
        self.world.update(glyph_vertex_buffer, context);
        self.ime.update(context);
    }

    fn input(&mut self, context: &StateContext, event: &WindowEvent) -> InputResult {
        if let Some(action) = self.store.winit_window_event_to_action(event) {
            self.action(context, action)
        } else {
            InputResult::Noop
        }
    }

    fn action(&mut self, context: &StateContext, action: Action) -> InputResult {
        match action {
            Action::Command(category, name, argument) => match &*category.to_string() {
                "system" => match self.execute_system_action(&name, argument, context) {
                    SystemActionResult::ChangeColorTheme(theme) => {
                        InputResult::ChangeColorTheme(theme)
                    }
                    SystemActionResult::ChangeGlobalDirection(direction) => {
                        InputResult::ChangeGlobalDirection(direction)
                    }
                    SystemActionResult::Noop => InputResult::InputConsumed,
                },
                "edit" => {
                    let op = Self::execute_editor_action(&name);
                    self.world.editor_operation(&op);
                    InputResult::InputConsumed
                }
                "world" => {
                    self.execute_world_action(&name, argument, context);
                    InputResult::InputConsumed
                }
                _ => InputResult::Noop,
            },
            Action::Keytype(c) => {
                self.new_chars.insert(c);
                let action = EditorOperation::InsertChar(c);
                self.world.editor_operation(&action);
                InputResult::InputConsumed
            }
            Action::ImeInput(value) => {
                self.new_chars.extend(value.chars());
                self.ime
                    .apply_ime_event(&Action::ImeInput(value.clone()), context);
                self.world
                    .editor_operation(&EditorOperation::InsertString(value));
                InputResult::InputConsumed
            }
            Action::ImePreedit(value, position) => {
                self.new_chars.extend(value.chars());
                self.ime
                    .apply_ime_event(&Action::ImePreedit(value, position), context);
                InputResult::InputConsumed
            }
            _ => InputResult::Noop,
        }
    }

    fn resize(&mut self, window_size: WindowSize) {
        self.world.change_window_size(window_size);
    }

    fn render(&mut self) -> (&Camera, Vec<&GlyphInstances>) {
        let mut world_instances = self.world.glyph_instances();
        let mut ime_instances = self.ime.get_instances();
        world_instances.append(&mut ime_instances);
        (self.world.camera(), world_instances)
    }
}
