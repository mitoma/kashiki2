use std::sync::{mpsc::Sender, LazyLock, Mutex};

use font_collector::FontCollector;
use stroke_parser::{action_store_parser::parse_setting, Action, ActionStore};
use text_buffer::action::EditorOperation;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use font_rasterizer::{
    color_theme::ColorTheme,
    context::{StateContext, WindowSize},
    instances::GlyphInstances,
    rasterizer_pipeline::Quarity,
};
use ui_support::{
    action::ActionProcessorStore,
    camera::{Camera, CameraAdjustment},
    layout_engine::{DefaultWorld, Model, World},
    run_support,
    ui::{caret_char, ImeInput, TextEdit},
    ui_context::TextContext,
    Flags, InputResult, SimpleStateCallback, SimpleStateSupport,
};
use winit::event::WindowEvent;

const FONT_DATA: &[u8] = include_bytes!("../../../fonts/BIZUDMincho-Regular.ttf");
const EMOJI_FONT_DATA: &[u8] = include_bytes!("../../../fonts/NotoEmoji-Regular.ttf");

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
pub fn change_window_size() {
    send_action(Action::new_command("system", "change-window-size-ui"));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn send_log(message: &str) {
    log::warn!("{}", message);
}

struct SingleCharCallback {
    world: DefaultWorld,
    store: ActionStore,
    action_processor_store: ActionProcessorStore,
    ime: ImeInput,
}

impl SingleCharCallback {
    fn new(window_size: WindowSize) -> Self {
        let mut store: ActionStore = Default::default();
        let key_setting = include_str!("../asset/key-settings.txt");
        parse_setting(key_setting)
            .into_iter()
            .for_each(|k| store.register_keybind(k));

        let mut world = DefaultWorld::new(window_size);
        let mut textedit = TextEdit::new(TextContext::default().with_max_col(40));

        textedit.editor_operation(&EditorOperation::InsertString(
            include_str!("../asset/initial.txt").to_string(),
        ));
        world.add(Box::new(textedit));
        world.look_current(CameraAdjustment::FitBothAndCentering);
        let ime = ImeInput::new();

        let mut action_processor_store = ActionProcessorStore::default();
        action_processor_store.add_default_system_processors();
        action_processor_store.add_default_world_processors();
        action_processor_store.add_default_edit_processors();
        action_processor_store.remove_processor(&"system".into(), &"exit".into());

        Self {
            world,
            store,
            action_processor_store,
            ime,
        }
    }
}

impl SimpleStateCallback for SingleCharCallback {
    fn init(&mut self, context: &StateContext) {
        set_action_sender(context.action_queue_sender.clone());

        context
            .ui_string_sender
            .send(caret_char(text_buffer::caret::CaretType::Primary).to_string())
            .unwrap();
        context
            .ui_string_sender
            .send(caret_char(text_buffer::caret::CaretType::Mark).to_string())
            .unwrap();
        context
            .ui_string_sender
            .send(self.world.chars().into_iter().collect::<String>())
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

    fn update(&mut self, context: &StateContext) {
        self.world.update(context);
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
        let result = self
            .action_processor_store
            .process(&action, context, &mut self.world);
        if result != InputResult::Noop {
            return result;
        }

        match action {
            Action::Command(category, name, _) => match &*category.to_string() {
                "world" => {
                    if name.as_str() == "look-current-and-centering" {
                        self.world
                            .look_current(CameraAdjustment::FitBothAndCentering);
                    }
                    InputResult::InputConsumed
                }
                _ => InputResult::Noop,
            },
            Action::Keytype(c) => {
                context.ui_string_sender.send(c.to_string()).unwrap();
                let action = EditorOperation::InsertChar(c);
                self.world.editor_operation(&action);
                InputResult::InputConsumed
            }
            Action::ImeInput(value) => {
                context.ui_string_sender.send(value.clone()).unwrap();
                self.ime
                    .apply_ime_event(&Action::ImeInput(value.clone()), context);
                self.world
                    .editor_operation(&EditorOperation::InsertString(value));
                InputResult::InputConsumed
            }
            Action::ImePreedit(value, position) => {
                context.ui_string_sender.send(value.clone()).unwrap();
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

    fn shutdown(&mut self) {}
}
