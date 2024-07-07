/*#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]*/
/* ↑は Windows で実行する時にコマンドプロンプトが開かないようにするためのもの。 */
mod categorized_memos;
mod kashikishi_actions;
mod local_datetime_format;
mod memos;

use arboard::Clipboard;
use categorized_memos::CategorizedMemos;
use font_collector::FontCollector;
use stroke_parser::{action_store_parser::parse_setting, Action, ActionArgument, ActionStore};
use text_buffer::action::EditorOperation;

use font_rasterizer::{
    camera::{Camera, CameraAdjustment, CameraOperation},
    color_theme::ColorTheme,
    context::{StateContext, WindowSize},
    font_buffer::GlyphVertexBuffer,
    instances::GlyphInstances,
    layout_engine::{HorizontalWorld, Model, ModelOperation, World},
    rasterizer_pipeline::Quarity,
    support::{run_support, Flags, InputResult, SimpleStateCallback, SimpleStateSupport},
    time::set_clock_mode,
    ui::{caret_char, ime_chars, ime_input::ImeInput, textedit::TextEdit},
};
use kashikishi_actions::{add_category_ui, change_theme_select};
use log::info;
use std::collections::HashSet;
use winit::event::WindowEvent;

use crate::{
    kashikishi_actions::{
        change_memos_category, command_palette_select, insert_date_select,
        select_move_memo_category,
    },
    memos::Memos,
};

//const ICON_IMAGE: &[u8] = include_bytes!("kashikishi-logo.png");

const FONT_DATA: &[u8] =
    include_bytes!("../../font_rasterizer/examples/font/HackGenConsole-Regular.ttf");
const EMOJI_FONT_DATA: &[u8] =
    include_bytes!("../../font_rasterizer/examples/font/NotoEmoji-Regular.ttf");

pub fn main() {
    //std::env::set_var("RUST_LOG", "simple_text=debug");
    //std::env::set_var("RUST_LOG", "font_rasterizer::ui::textedit=info");
    //std::env::set_var("RUST_LOG", "font_rasterizer::ui::view_element_state=debug");
    //std::env::set_var("RUST_LOG", "font_rasterizer::layout_engine=info");
    //std::env::set_var("FONT_RASTERIZER_DEBUG", "debug");
    pollster::block_on(run());
}

const COLOR_THEME: ColorTheme = ColorTheme::SolarizedDark;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    // setup icon
    // TODO 正式公開前にアイコンを作る必要がありそう
    //let icon_image = image::load_from_memory(ICON_IMAGE).unwrap().to_rgba8();
    //let icon = Icon::from_rgba(icon_image.to_vec(), icon_image.width(), icon_image.height()).ok();
    let icon = None;

    // setup font
    let mut collector = FontCollector::default();
    collector.add_system_fonts();
    let kyokasho_font = collector.load_font("UD デジタル 教科書体 N-R");
    let mut font_binaries = Vec::new();
    if let Some(kyokasho_font) = kyokasho_font {
        font_binaries.push(kyokasho_font);
    }
    font_binaries.push(collector.convert_font(FONT_DATA.to_vec(), None).unwrap());
    font_binaries.push(
        collector
            .convert_font(EMOJI_FONT_DATA.to_vec(), None)
            .unwrap(),
    );

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
    };
    run_support(support).await;
}

struct KashikishiCallback {
    store: ActionStore,
    world: Box<dyn World>,
    ime: ImeInput,
    new_chars: HashSet<char>,
    categorized_memos: CategorizedMemos,
}

impl KashikishiCallback {
    fn new(window_size: WindowSize) -> Self {
        let mut store: ActionStore = Default::default();
        let key_setting = include_str!("key-settings.txt");
        info!("{}", key_setting);
        let keybinds = parse_setting(key_setting);
        keybinds
            .iter()
            .for_each(|k| store.register_keybind(k.clone()));
        let ime = ImeInput::new();

        let world = Box::new(HorizontalWorld::new(window_size));
        let new_chars = HashSet::new();

        let categorized_memos = CategorizedMemos::load_memos();

        let mut result = Self {
            store,
            world,
            ime,
            new_chars,
            categorized_memos,
        };
        result.reset_world(window_size);
        result
    }

    // ワールドを今のカテゴリでリセットする
    fn reset_world(&mut self, window_size: WindowSize) {
        let mut world = Box::new(HorizontalWorld::new(window_size));
        for memo in self
            .categorized_memos
            .get_current_memos()
            .unwrap()
            .memos
            .iter()
        {
            let mut textedit = TextEdit::default();
            textedit.editor_operation(&EditorOperation::InsertString(memo.to_string()));
            textedit.editor_operation(&EditorOperation::BufferHead);
            let model = Box::new(textedit);
            world.add(model);
        }
        let look_at = 0;
        world.look_at(look_at, CameraAdjustment::FitBoth);
        world.re_layout();
        self.world = world;
        // world にすでに表示されるグリフを追加する
        let chars = self
            .world
            .strings()
            .join("")
            .chars()
            .collect::<HashSet<char>>();
        self.new_chars.extend(chars);
    }
}

impl SimpleStateCallback for KashikishiCallback {
    fn init(&mut self, glyph_vertex_buffer: &mut GlyphVertexBuffer, context: &StateContext) {
        // world にすでに表示されるグリフを追加する
        let mut chars = self
            .world
            .strings()
            .join("")
            .chars()
            .collect::<HashSet<char>>();

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
        self.world.look_at(0, CameraAdjustment::FitBoth);
    }

    fn resize(&mut self, window_size: WindowSize) {
        self.world.change_window_size(window_size);
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

    fn input(
        &mut self,
        glyph_vertex_buffer: &GlyphVertexBuffer,
        context: &StateContext,
        event: &WindowEvent,
    ) -> InputResult {
        if let Some(action) = self.store.winit_window_event_to_action(event) {
            self.action(glyph_vertex_buffer, context, action)
        } else {
            InputResult::Noop
        }
    }

    fn action(
        &mut self,
        glyph_vertex_buffer: &GlyphVertexBuffer,
        context: &StateContext,
        action: Action,
    ) -> InputResult {
        fn add_modal(
            chars: &mut HashSet<char>,
            world: &mut Box<dyn World>,
            model: Box<dyn Model>,
        ) -> InputResult {
            chars.extend(model.to_string().chars());
            world.add_next(model);
            world.re_layout();
            world.look_next(CameraAdjustment::NoCare);
            InputResult::InputConsumed
        }

        match action {
            Action::Command(category, name, argument) => match &*category.to_string() {
                "system" => {
                    let action = match &*name.to_string() {
                        "exit" => {
                            self.categorized_memos
                                .update_current_memos(Memos::from(&*self.world));
                            self.categorized_memos.save_memos().unwrap();
                            return InputResult::SendExit;
                        }
                        "command-palette" => {
                            return add_modal(
                                &mut self.new_chars,
                                &mut self.world,
                                Box::new(command_palette_select(
                                    context.action_queue_sender.clone(),
                                )),
                            );
                        }
                        "toggle-fullscreen" => {
                            return InputResult::ToggleFullScreen;
                        }
                        "select-theme" => {
                            return add_modal(
                                &mut self.new_chars,
                                &mut self.world,
                                Box::new(change_theme_select(context.action_queue_sender.clone())),
                            )
                        }
                        "change-theme" => match argument {
                            ActionArgument::String(value) => match &*value.to_string() {
                                "black" => {
                                    return InputResult::ChangeColorTheme(
                                        ColorTheme::SolarizedBlackback,
                                    )
                                }
                                "dark" => {
                                    return InputResult::ChangeColorTheme(ColorTheme::SolarizedDark)
                                }
                                "light" => {
                                    return InputResult::ChangeColorTheme(
                                        ColorTheme::SolarizedLight,
                                    )
                                }
                                _ => EditorOperation::Noop,
                            },
                            _ => EditorOperation::Noop,
                        },
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
                        "paste" => {
                            match Clipboard::new().and_then(|mut context| context.get_text()) {
                                Ok(text) => {
                                    self.new_chars.extend(text.chars());
                                    EditorOperation::InsertString(text)
                                }
                                Err(_) => EditorOperation::Noop,
                            }
                        }
                        "copy" => EditorOperation::Copy(|text| {
                            let _ = Clipboard::new().and_then(|mut context| context.set_text(text));
                        }),
                        "cut" => EditorOperation::Cut(|text| {
                            let _ = Clipboard::new().and_then(|mut context| context.set_text(text));
                        }),
                        "mark" => EditorOperation::Mark,
                        "unmark" => EditorOperation::UnMark,
                        _ => EditorOperation::Noop,
                    };
                    self.world.editor_operation(&action);

                    InputResult::InputConsumed
                }
                "world" => {
                    match &*name.to_string() {
                        "remove-current" => {
                            self.world.remove_current();
                            self.world.re_layout();
                            self.world.look_prev(CameraAdjustment::NoCare);
                        }
                        "reset-zoom" => self.world.look_current(CameraAdjustment::FitBoth),
                        "look-current" => self.world.look_current(CameraAdjustment::NoCare),
                        "look-next" => self.world.look_next(CameraAdjustment::NoCare),
                        "look-prev" => self.world.look_prev(CameraAdjustment::NoCare),
                        "swap-next" => self.world.swap_next(),
                        "swap-prev" => self.world.swap_prev(),
                        "fit-width" => self.world.look_current(CameraAdjustment::FitWidth),
                        "fit-height" => self.world.look_current(CameraAdjustment::FitHeight),
                        "forward" => self.world.camera_operation(CameraOperation::Forward),
                        "back" => self.world.camera_operation(CameraOperation::Backward),
                        "change-direction" => {
                            self.world.model_operation(&ModelOperation::ChangeDirection)
                        }
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
                        "copy-display" => {
                            self.world
                                .model_operation(&ModelOperation::CopyDisplayString(
                                    glyph_vertex_buffer,
                                    |text| {
                                        let _ = Clipboard::new()
                                            .and_then(|mut context| context.set_text(text));
                                    },
                                ))
                        }
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
                    InputResult::InputConsumed
                }
                "kashikishi" => {
                    match &*name.to_string() {
                        "save" => {
                            self.categorized_memos
                                .update_current_memos(Memos::from(&*self.world));
                            self.categorized_memos.save_memos().unwrap();
                        }
                        "add-memo" => {
                            let textedit = TextEdit::default();
                            let model = Box::new(textedit);
                            self.world.add(model);
                            self.world.re_layout();
                            self.world
                                .look_at(self.world.model_length() - 1, CameraAdjustment::NoCare);
                        }
                        "remove-memo" => {
                            self.world.remove_current();
                            self.world.re_layout();
                            self.world.look_prev(CameraAdjustment::NoCare);
                        }
                        "insert-date" => {
                            return add_modal(
                                &mut self.new_chars,
                                &mut self.world,
                                Box::new(insert_date_select(context.action_queue_sender.clone())),
                            )
                        }
                        "select-category" => {
                            return add_modal(
                                &mut self.new_chars,
                                &mut self.world,
                                Box::new(change_memos_category(
                                    &self.categorized_memos,
                                    context.action_queue_sender.clone(),
                                )),
                            )
                        }
                        "select-move-memo-category" => {
                            return add_modal(
                                &mut self.new_chars,
                                &mut self.world,
                                Box::new(select_move_memo_category(
                                    &self.categorized_memos,
                                    context.action_queue_sender.clone(),
                                )),
                            )
                        }
                        "change-memos-category" => match argument {
                            ActionArgument::String(category) => {
                                if self.categorized_memos.current_category == category {
                                    return InputResult::InputConsumed;
                                }
                                self.categorized_memos
                                    .update_current_memos(Memos::from(&*self.world));
                                self.categorized_memos.current_category = category;
                                self.reset_world(context.window_size);
                            }
                            _ => { /* noop */ }
                        },
                        "move-memo" => match argument {
                            ActionArgument::String(category) => {
                                if self.categorized_memos.current_category == category {
                                    return InputResult::InputConsumed;
                                }
                                self.categorized_memos
                                    .add_memo(Some(&category), self.world.current_string());
                                context
                                    .action_queue_sender
                                    .send(Action::new_command("world", "remove-current"))
                                    .unwrap();
                            }
                            _ => { /* noop */ }
                        },
                        "add-category-ui" => {
                            return add_modal(
                                &mut self.new_chars,
                                &mut self.world,
                                Box::new(add_category_ui(context.action_queue_sender.clone())),
                            );
                        }
                        "add-category" => {
                            if let ActionArgument::String(category) = argument {
                                if self.categorized_memos.categories().contains(&category) {
                                    self.categorized_memos
                                        .add_memo(Some(&category), String::new());
                                }
                            }
                        }
                        _ => {}
                    };
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

    fn render(&mut self) -> (&Camera, Vec<&GlyphInstances>) {
        let mut world_instances = self.world.glyph_instances();
        let mut ime_instances = self.ime.get_instances();
        world_instances.append(&mut ime_instances);
        (self.world.camera(), world_instances)
    }
}
