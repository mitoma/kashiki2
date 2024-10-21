#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
/* ↑は Windows で実行する時にコマンドプロンプトが開かないようにするためのもの。 */
mod action_repository;
mod categorized_memos;
mod kashikishi_actions;
mod local_datetime_format;
mod memos;
mod world;

use arboard::Clipboard;
use clap::{command, Parser};
use font_collector::FontCollector;
use rokid_3dof::RokidMax;
use stroke_parser::{
    action_store_parser::parse_setting, Action, ActionArgument, ActionStore, CommandName,
    CommandNamespace,
};
use text_buffer::action::EditorOperation;
use world::{CategorizedMemosWorld, HelpWorld, ModalWorld, NullWorld, StartWorld};

use font_rasterizer::{
    camera::{Camera, CameraAdjustment, CameraOperation},
    color_theme::ColorTheme,
    context::{StateContext, WindowSize},
    font_buffer::{Direction, GlyphVertexBuffer},
    instances::GlyphInstances,
    layout_engine::{Model, ModelOperation, World},
    rasterizer_pipeline::Quarity,
    support::{
        action_processor::{ActionProcessor, ActionProcessorStore},
        run_support, Flags, InputResult, SimpleStateCallback, SimpleStateSupport,
    },
    time::set_clock_mode,
    ui::{caret_char, ime_chars, ImeInput},
};
use log::info;
use std::collections::HashSet;
use winit::event::WindowEvent;

use crate::kashikishi_actions::command_palette_select;

//const ICON_IMAGE: &[u8] = include_bytes!("kashikishi-logo.png");

const FONT_DATA: &[u8] =
    include_bytes!("../../font_rasterizer/examples/font/BIZUDMincho-Regular.ttf");
const EMOJI_FONT_DATA: &[u8] =
    include_bytes!("../../font_rasterizer/examples/font/NotoEmoji-Regular.ttf");

pub fn main() {
    //std::env::set_var("RUST_LOG", "simple_text=debug");
    //std::env::set_var("RUST_LOG", "font_rasterizer::ui::textedit=info");
    //std::env::set_var("RUST_LOG", "font_rasterizer::ui::view_element_state=debug");
    //std::env::set_var("RUST_LOG", "font_rasterizer::layout_engine=info");
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
        new_chars: &mut HashSet<char>,
    ) -> InputResult {
        let narrow = match arg {
            ActionArgument::String(value) => Some(value.to_owned()),
            _ => None,
        };
        let modal = command_palette_select(context, narrow);
        new_chars.extend(modal.to_string().chars());
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
    let font_binaries = {
        let mut collector = FontCollector::default();
        let mut font_binaries = Vec::new();
        if !args.use_embedded_font {
            collector.add_system_fonts();
            let fonts = args
                .font_names
                .iter()
                .filter_map(|str| collector.load_font(str));
            for font in fonts {
                font_binaries.push(font);
            }
        }
        // 埋め込まれるフォントは fallback に使うから常に追加する
        font_binaries.push(collector.convert_font(FONT_DATA.to_vec(), None).unwrap());
        font_binaries.push(
            collector
                .convert_font(EMOJI_FONT_DATA.to_vec(), None)
                .unwrap(),
        );
        font_binaries
    };

    set_clock_mode(font_rasterizer::time::ClockMode::StepByStep);
    let window_size = WindowSize::new(800, 600);
    let callback = KashikishiCallback::new(window_size);
    let support = SimpleStateSupport {
        window_icon: icon,
        window_title: "Kashikishi".to_string(),
        window_size,
        callback: Box::new(callback),
        quarity: Quarity::CappedVeryHigh(1920 * 2, 1200 * 2),
        color_theme: COLOR_THEME,
        flags: Flags::DEFAULT,
        font_binaries,
        performance_mode: args.performance_mode,
    };
    run_support(support).await;
}

struct KashikishiCallback {
    store: ActionStore,
    world: Box<dyn ModalWorld>,
    ime: ImeInput,
    action_processor_store: ActionProcessorStore,
    new_chars: HashSet<char>,
    rokid_max: Option<RokidMax>,
    ar_mode: bool,
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
        action_processor_store.add_processor(Box::new(SystemCommandPalette));

        let rokid_max = RokidMax::new().ok();

        Self {
            store,
            world: Box::new(NullWorld::new(window_size)),
            ime,
            action_processor_store,
            new_chars: HashSet::new(),
            rokid_max,
            ar_mode: false,
        }
    }

    fn execute_editor_action(command_name: &str, chars: &mut HashSet<char>) -> EditorOperation {
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
            "paste" => match Clipboard::new().and_then(|mut context| context.get_text()) {
                Ok(text) => {
                    chars.extend(text.chars());
                    EditorOperation::InsertString(text)
                }
                Err(_) => EditorOperation::Noop,
            },
            "copy" => EditorOperation::Copy(|text| {
                let _ = Clipboard::new().and_then(|mut context| context.set_text(text));
            }),
            "cut" => EditorOperation::Cut(|text| {
                let _ = Clipboard::new().and_then(|mut context| context.set_text(text));
            }),
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
        let world = self.world.get_mut();
        match command_name {
            "remove-current" => {
                world.remove_current();
                world.re_layout();
                world.look_prev(CameraAdjustment::NoCare);
            }
            "reset-zoom" => world.look_current(CameraAdjustment::FitBoth),
            "look-current-and-centering" => {
                if let Some(rokid_max) = self.rokid_max.as_mut() {
                    let _ = rokid_max.reset();
                }
                world.look_current(CameraAdjustment::FitBothAndCentering)
            }
            "look-current" => world.look_current(CameraAdjustment::NoCare),
            "look-next" => world.look_next(CameraAdjustment::NoCare),
            "look-prev" => world.look_prev(CameraAdjustment::NoCare),
            "swap-next" => world.swap_next(),
            "swap-prev" => world.swap_prev(),
            "fit-width" => world.look_current(CameraAdjustment::FitWidth),
            "fit-height" => world.look_current(CameraAdjustment::FitHeight),
            "fit-by-direction" => {
                if context.global_direction == Direction::Horizontal {
                    world.look_current(CameraAdjustment::FitWidth)
                } else {
                    world.look_current(CameraAdjustment::FitHeight)
                }
            }
            "forward" => world.camera_operation(CameraOperation::Forward),
            "back" => world.camera_operation(CameraOperation::Backward),
            "change-direction" => world.model_operation(&ModelOperation::ChangeDirection(None)),
            "increase-row-interval" => world.model_operation(&ModelOperation::IncreaseRowInterval),
            "decrease-row-interval" => world.model_operation(&ModelOperation::DecreaseRowInterval),
            "increase-col-interval" => world.model_operation(&ModelOperation::IncreaseColInterval),
            "decrease-col-interval" => world.model_operation(&ModelOperation::DecreaseColInterval),
            "increase-col-scale" => world.model_operation(&ModelOperation::IncreaseColScale),
            "decrease-col-scale" => world.model_operation(&ModelOperation::DecreaseColScale),
            "increase-row-scale" => world.model_operation(&ModelOperation::IncreaseRowScale),
            "decrease-row-scale" => world.model_operation(&ModelOperation::DecreaseRowScale),
            "copy-display" => world.model_operation(&ModelOperation::CopyDisplayString(
                context.char_width_calcurator.clone(),
                |text| {
                    let _ = Clipboard::new().and_then(|mut context| context.set_text(text));
                },
            )),
            "toggle-psychedelic" => world.model_operation(&ModelOperation::TogglePsychedelic),
            "move-to-click" => {
                match argument {
                    ActionArgument::Point((x, y)) => {
                        let (x_ratio, y_ratio) = (
                            (x / context.window_size.width as f32 * 2.0) - 1.0,
                            1.0 - (y / context.window_size.height as f32 * 2.0),
                        );
                        world.move_to_position(x_ratio, y_ratio);
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
                        world.move_to_position(x_ratio, y_ratio);
                        world.editor_operation(&EditorOperation::Mark);
                    }
                    _ => { /* noop */ }
                }
            }
            // AR 系はアクションのカテゴリを変えるべきだろうか？
            "reset-rokid" => {
                if let Some(rokid_max) = self.rokid_max.as_mut() {
                    let _ = rokid_max.reset();
                }
            }
            "toggle-ar-mode" => {
                if let Some(rokid_max) = self.rokid_max.as_mut() {
                    let _ = rokid_max.reset();
                    world.camera_operation(CameraOperation::UpdateEyeQuaternion(Some(
                        rokid_max.quaternion(),
                    )));
                }
                self.ar_mode = !self.ar_mode;
            }
            _ => {}
        };
    }
}

impl SimpleStateCallback for KashikishiCallback {
    fn init(&mut self, glyph_vertex_buffer: &mut GlyphVertexBuffer, context: &StateContext) {
        // 初期状態で表示するワールドを設定する
        self.world = Box::new(StartWorld::new(context));

        // world にすでに表示されるグリフを追加する
        let mut chars = self.world.world_chars();

        // キャレットのグリフを追加する
        chars.insert(caret_char(text_buffer::caret::CaretType::Primary));
        chars.insert(caret_char(text_buffer::caret::CaretType::Mark));

        // IME のグリフを追加する
        chars.extend(ime_chars().iter().cloned());

        // グリフバッファに追加する
        glyph_vertex_buffer
            .append_glyph(&context.device, &context.queue, chars)
            .unwrap();

        // カメラを初期化する
        context
            .post_action_queue_sender
            .send(Action::new_command("world", "fit-by-direction"))
            .unwrap();
    }

    fn resize(&mut self, window_size: WindowSize) {
        self.world.get_mut().change_window_size(window_size);
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
        self.world.get_mut().update(glyph_vertex_buffer, context);
        self.ime.update(context);

        // AR モードが有効な場合はカメラの向きを変える。少し雑だが良い場所が見つかるまでここで。
        if self.ar_mode {
            if let Some(rokid_max) = self.rokid_max.as_ref() {
                self.world
                    .get_mut()
                    .camera_operation(CameraOperation::UpdateEyeQuaternion(Some(
                        rokid_max.quaternion(),
                    )));
            }
        }
    }

    fn input(&mut self, context: &StateContext, event: &WindowEvent) -> InputResult {
        if let Some(action) = self.store.winit_window_event_to_action(event) {
            self.action(context, action)
        } else {
            InputResult::Noop
        }
    }

    fn action(&mut self, context: &StateContext, action: Action) -> InputResult {
        let result = self.action_processor_store.process(
            &action,
            context,
            self.world.get_mut(),
            &mut self.new_chars,
        );
        if result != InputResult::Noop {
            return result;
        }

        match action {
            Action::Command(category, name, argument) => match &*category.to_string() {
                "edit" => {
                    let op = Self::execute_editor_action(&name, &mut self.new_chars);
                    self.world.get_mut().editor_operation(&op);
                    InputResult::InputConsumed
                }
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
                        self.new_chars.extend(self.world.world_chars());
                        context
                            .post_action_queue_sender
                            .send(Action::new_command("world", "fit-by-direction"))
                            .unwrap();
                    }
                    InputResult::InputConsumed
                }
                _ => {
                    let (result, chars) = self
                        .world
                        .apply_action(context, Action::Command(category, name, argument));
                    self.new_chars.extend(chars);
                    result
                }
            },
            Action::Keytype(c) => {
                self.new_chars.insert(c);
                let action = EditorOperation::InsertChar(c);
                self.world.get_mut().editor_operation(&action);
                InputResult::InputConsumed
            }
            Action::ImeInput(value) => {
                self.new_chars.extend(value.chars());
                self.ime
                    .apply_ime_event(&Action::ImeInput(value.clone()), context);
                self.world
                    .get_mut()
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

    fn render(&mut self) -> (&Camera, Vec<&GlyphInstances>) {
        let mut world_instances = self.world.get().glyph_instances();
        let mut ime_instances = self.ime.get_instances();
        world_instances.append(&mut ime_instances);
        (self.world.get().camera(), world_instances)
    }

    fn shutdown(&mut self) {
        self.world.graceful_exit();
    }
}
