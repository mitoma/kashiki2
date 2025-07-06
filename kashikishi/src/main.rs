#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
/* ↑は Windows で実行する時にコマンドプロンプトが開かないようにするためのもの。 */
mod action_repository;
mod categorized_memos;
mod kashikishi_actions;
mod local_datetime_format;
mod memos;
mod rokid_max_ext;
mod world;

use std::{rc::Rc, sync::Mutex};

use arboard::Clipboard;
use clap::{Parser, command};
use font_collector::{FontCollector, FontRepository};
use rokid_max_ext::RokidMaxAction;
use stroke_parser::{
    Action, ActionArgument, ActionStore, CommandName, CommandNamespace,
    action_store_parser::parse_setting,
};
use text_buffer::action::EditorOperation;
use world::{CategorizedMemosWorld, HelpWorld, ModalWorld, NullWorld, StartWorld};

use font_rasterizer::{
    color_theme::ColorTheme,
    context::{StateContext, WindowSize},
    glyph_vertex_buffer::Direction,
    rasterizer_pipeline::Quarity,
    time::set_clock_mode,
};
use log::info;
use ui_support::{
    Flags, InputResult, RenderData, SimpleStateCallback, SimpleStateSupport,
    action::{ActionProcessor, ActionProcessorStore},
    action_recorder::{ActionRecorder, InMemoryActionRecordRepository},
    camera::{CameraAdjustment, CameraOperation},
    layout_engine::{Model, ModelOperation, World},
    register_default_border, register_default_caret, run_support,
    ui::{ImeInput, caret_char, ime_chars},
};
use winit::event::WindowEvent;

use crate::kashikishi_actions::command_palette_select;

//const ICON_IMAGE: &[u8] = include_bytes!("kashikishi-logo.png");

const FONT_DATA: &[u8] = include_bytes!("../../fonts/BIZUDMincho-Regular.ttf");
const EMOJI_FONT_DATA: &[u8] = include_bytes!("../../fonts/NotoEmoji-Regular.ttf");

pub fn main() {
    //std::env::set_var("RUST_LOG", "simple_text=debug");
    //std::env::set_var("RUST_LOG", "font_rasterizer::ui::textedit=info");
    //std::env::set_var("RUST_LOG", "font_rasterizer::ui::view_element_state=debug");
    //std::env::set_var("RUST_LOG", "font_rasterizer::layout_engine=info");
    //std::env::set_var("RUST_LOG", "ui_support::action::system=debug");
    //std::env::set_var("FONT_RASTERIZER_DEBUG", "debug");
    let args = Args::parse();
    pollster::block_on(run(args));
}

#[derive(Parser, Debug, Clone)]
#[command(author, version, about = "idea note", long_about = None)]
pub struct Args {
    /// use high performance mode
    #[arg(short, long, default_value = "false")]
    pub performance_mode: bool,

    /// use embedded font
    #[arg(short, long, default_value = "false")]
    pub use_embedded_font: bool,

    /// font
    #[arg(short, long, default_values = ["UD デジタル 教科書体 N", "UD デジタル 教科書体 N-R"])]
    pub font_names: Vec<String>,
}

const COLOR_THEME: ColorTheme = ColorTheme::SolarizedDark;

struct SystemCommandPalette;
impl ActionProcessor for SystemCommandPalette {
    fn namespace(&self) -> CommandNamespace {
        "system".into()
    }

    fn name(&self) -> CommandName {
        "command-palette".into()
    }

    fn process(
        &self,
        arg: &ActionArgument,
        context: &StateContext,
        world: &mut dyn World,
    ) -> InputResult {
        let narrow = match arg {
            ActionArgument::String(value) => Some(value.to_owned()),
            _ => None,
        };
        let modal = command_palette_select(context, narrow);
        context.register_string(modal.to_string());
        world.add_next(Box::new(modal));
        world.re_layout();
        let adjustment = if context.global_direction == Direction::Horizontal {
            CameraAdjustment::FitWidth
        } else {
            CameraAdjustment::FitHeight
        };
        world.look_next(adjustment);
        InputResult::InputConsumed
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run(args: Args) {
    // setup icon
    // TODO 正式公開前にアイコンを作る必要がありそう
    //let icon_image = image::load_from_memory(ICON_IMAGE).unwrap().to_rgba8();
    //let icon = Icon::from_rgba(icon_image.to_vec(), icon_image.width(), icon_image.height()).ok();
    let icon = None;

    // setup font
    let font_repository = {
        let mut font_collector = FontCollector::default();
        if !args.use_embedded_font {
            font_collector.add_system_fonts();
        }
        let mut font_repository = FontRepository::new(font_collector);
        if !args.use_embedded_font {
            args.font_names.iter().for_each(|name| {
                font_repository.add_fallback_font_from_system(name);
            });
        }
        font_repository.add_fallback_font_from_binary(FONT_DATA.to_vec(), None);
        font_repository.add_fallback_font_from_binary(EMOJI_FONT_DATA.to_vec(), None);
        font_repository
    };

    set_clock_mode(font_rasterizer::time::ClockMode::StepByStep);
    let window_size = WindowSize::new(800, 600);
    let callback = KashikishiCallback::new(window_size);
    let support = SimpleStateSupport {
        window_icon: icon,
        window_title: "Kashikishi".to_string(),
        window_size,
        callback: Box::new(callback),
        //quarity: Quarity::CappedVeryHigh(1920 * 2, 1200 * 2),
        quarity: Quarity::Middle,
        //quarity: Quarity::Fixed(640, 480),
        color_theme: COLOR_THEME,
        flags: Flags::EXIT_ON_ESC | Flags::FULL_SCREEN,
        font_repository,
        performance_mode: args.performance_mode,
    };
    run_support(support).await;
}

struct KashikishiCallback {
    store: ActionStore,
    world: Box<dyn ModalWorld>,
    ime: ImeInput,
    action_processor_store: ActionProcessorStore,
    rokid_max_action: Rc<Mutex<RokidMaxAction>>,
    action_recorder: Rc<Mutex<ActionRecorder>>,
}

impl KashikishiCallback {
    fn new(window_size: WindowSize) -> Self {
        let mut store: ActionStore = Default::default();
        let key_setting = include_str!("../asset/key-settings.txt");
        info!("{}", key_setting);
        let keybinds = parse_setting(key_setting);
        keybinds
            .iter()
            .for_each(|k| store.register_keybind(k.clone()));
        let ime = ImeInput::new();

        let mut action_processor_store = ActionProcessorStore::default();
        action_processor_store.add_default_system_processors();
        action_processor_store.add_default_edit_processors();
        action_processor_store.add_default_world_processors();
        action_processor_store.add_processor(Box::new(SystemCommandPalette));

        let action_recorder =
            ActionRecorder::new(Box::new(InMemoryActionRecordRepository::default()));
        let action_recorder = Rc::new(Mutex::new(action_recorder));
        action_processor_store.add_namespace_processors(action_recorder.clone());

        let rokid_max_action = RokidMaxAction::new();
        let rokid_max_action = Rc::new(Mutex::new(rokid_max_action));
        action_processor_store.add_namespace_processors(rokid_max_action.clone());

        Self {
            store,
            world: Box::new(NullWorld::new(window_size)),
            ime,
            action_processor_store,
            rokid_max_action,
            action_recorder,
        }
    }

    fn execute_world_action(
        &mut self,
        command_name: &str,
        _argument: ActionArgument,
        context: &StateContext,
    ) {
        let world = self.world.get_mut();
        match command_name {
            "copy-display" => world.model_operation(&ModelOperation::CopyDisplayString(
                context.char_width_calcurator.clone(),
                |text| {
                    let _ = Clipboard::new().and_then(|mut context| context.set_text(text));
                },
            )),
            "look-current-and-centering" => {
                let _ = self
                    .rokid_max_action
                    .lock()
                    .map(|rokid_max_action| rokid_max_action.reset());
                world.look_current(CameraAdjustment::FitBothAndCentering)
            }
            _ => {}
        };
    }
}

impl SimpleStateCallback for KashikishiCallback {
    fn init(&mut self, context: &StateContext) {
        // 初期状態で表示するワールドを設定する
        self.world = Box::new(StartWorld::new(context));

        // world にすでに表示されるグリフを追加する
        let mut chars = self.world.world_chars();
        // キャレットのグリフを追加する
        chars.insert(caret_char(text_buffer::caret::CaretType::Primary));
        chars.insert(caret_char(text_buffer::caret::CaretType::Mark));
        // IME のグリフを追加する
        chars.extend(ime_chars());
        context.register_string(chars.into_iter().collect::<String>());

        register_default_caret(context);
        register_default_border(context);

        // カメラを初期化する
        context.register_post_action(Action::new_command("world", "fit-by-direction"));
    }

    fn resize(&mut self, window_size: WindowSize) {
        self.world.get_mut().change_window_size(window_size);
    }

    fn update(&mut self, context: &StateContext) {
        self.action_recorder.lock().unwrap().replay(context);

        self.world.get_mut().update(context);
        self.ime.update(context);

        let _ = self.rokid_max_action.lock().map(|rokid_max_action| {
            self.world
                .get_mut()
                .camera_operation(CameraOperation::UpdateEyeQuaternion(
                    rokid_max_action.quaternion(),
                ));
        });
    }

    fn input(&mut self, context: &StateContext, event: &WindowEvent) -> InputResult {
        if let Some(action) = self.store.winit_window_event_to_action(event) {
            self.action(context, action)
        } else {
            InputResult::Noop
        }
    }

    fn action(&mut self, context: &StateContext, action: Action) -> InputResult {
        self.action_recorder.lock().unwrap().record(&action);

        let result = self
            .action_processor_store
            .process(&action, context, self.world.get_mut());
        if result != InputResult::Noop {
            return result;
        }

        match action {
            Action::Command(category, name, argument) => match &*category.to_string() {
                "world" => {
                    self.execute_world_action(&name, argument, context);
                    InputResult::InputConsumed
                }
                "mode" => {
                    let world: Option<Box<dyn ModalWorld>> = match &*name.to_string() {
                        "start" => Some(Box::new(StartWorld::new(context))),
                        "category" => Some(Box::new(CategorizedMemosWorld::new(
                            context.window_size,
                            context.global_direction,
                        ))),
                        "help" => Some(Box::new(HelpWorld::new(context.window_size))),
                        _ => None,
                    };
                    if let Some(world) = world {
                        self.world.graceful_exit();
                        self.world = world;
                        context.register_string(self.world.world_chars().into_iter().collect());
                        context
                            .register_post_action(Action::new_command("world", "fit-by-direction"));
                    }
                    InputResult::InputConsumed
                }
                _ => {
                    let (result, chars) = self
                        .world
                        .apply_action(context, Action::Command(category, name, argument));
                    context.register_string(chars.into_iter().collect());
                    result
                }
            },
            Action::Keytype(c) => {
                context.register_string(c.to_string());
                let action = EditorOperation::InsertChar(c);
                self.world.get_mut().editor_operation(&action);
                InputResult::InputConsumed
            }
            Action::ImeInput(value) => {
                context.register_string(value.clone());
                self.ime
                    .apply_ime_event(&Action::ImeInput(value.clone()), context);
                self.world
                    .get_mut()
                    .editor_operation(&EditorOperation::InsertString(value));
                InputResult::InputConsumed
            }
            Action::ImePreedit(value, position) => {
                context.register_string(value.clone());
                self.ime
                    .apply_ime_event(&Action::ImePreedit(value, position), context);
                InputResult::InputConsumed
            }
            Action::ImeEnable => InputResult::Noop,
            Action::ImeDisable => InputResult::Noop,
        }
    }

    fn render(&mut self) -> RenderData<'_> {
        let world = self.world.get();
        let mut world_instances = world.glyph_instances();
        let mut ime_instances = self.ime.get_instances();
        world_instances.append(&mut ime_instances);
        RenderData {
            camera: self.world.get().camera(),
            glyph_instances: world_instances,
            vector_instances: world.vector_instances(),
        }
    }

    fn shutdown(&mut self) {
        self.world.graceful_exit();
    }
}
