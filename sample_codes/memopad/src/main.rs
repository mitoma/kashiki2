#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use clipboard::{ClipboardContext, ClipboardProvider};
use font_collector::FontCollector;
use stroke_parser::{action_store_parser::parse_setting, Action, ActionStore};
use text_buffer::action::EditorOperation;

use font_rasterizer::{
    camera::{Camera, CameraAdjustment, CameraOperation},
    color_theme::ColorTheme,
    font_buffer::GlyphVertexBuffer,
    instances::GlyphInstances,
    layout_engine::{HorizontalWorld, Model, ModelOperation, World},
    rasterizer_pipeline::Quarity,
    support::{run_support, Flags, InputResult, SimpleStateCallback, SimpleStateSupport},
    ui::{ime_input::ImeInput, textedit::TextEdit},
};
use log::info;
use std::{collections::HashSet, path::Path};
use std::{fs, path::PathBuf};
use winit::{event::WindowEvent, window::Icon};

const ICON_IMAGE: &[u8] = include_bytes!("memopad-logo.png");

const FONT_DATA: &[u8] =
    include_bytes!("../../../font_rasterizer/examples/font/HackGenConsole-Regular.ttf");
const EMOJI_FONT_DATA: &[u8] =
    include_bytes!("../../../font_rasterizer/examples/font/NotoEmoji-Regular.ttf");

pub fn main() {
    //std::env::set_var("RUST_LOG", "simple_text=debug");
    //std::env::set_var("RUST_LOG", "font_rasterizer::ui::textedit=debug");
    //std::env::set_var("FONT_RASTERIZER_DEBUG", "debug");
    pollster::block_on(run());
}

const COLOR_THEME: ColorTheme = ColorTheme::SolarizedDark;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    // setup icon
    let icon_image = image::load_from_memory(ICON_IMAGE).unwrap().to_rgba8();
    let icon = Icon::from_rgba(icon_image.to_vec(), icon_image.width(), icon_image.height()).ok();

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

    let callback = MemoPadCallback::new();
    let support = SimpleStateSupport {
        window_icon: icon,
        window_title: "Memopad".to_string(),
        window_size: (800, 600),
        callback: Box::new(callback),
        quarity: Quarity::CappedVeryHigh((1920.0 * 1.5) as u32, (1200.0 * 1.5) as u32),
        color_theme: COLOR_THEME,
        flags: Flags::DEFAULT,
        font_binaries,
    };
    run_support(support).await;
}

struct MemoPadCallback {
    store: ActionStore,
    world: Box<dyn World>,
    ime: ImeInput,
}

impl MemoPadCallback {
    fn new() -> Self {
        let mut store: ActionStore = Default::default();
        let key_setting = include_str!("key-settings.txt");
        info!("{}", key_setting);
        let keybinds = parse_setting(String::from(key_setting));
        keybinds
            .iter()
            .for_each(|k| store.register_keybind(k.clone()));
        let ime = ImeInput::new();

        let mut world = Box::new(HorizontalWorld::new(800, 600));
        let memos = load_memos();
        for memo in memos.memos {
            let mut textedit = TextEdit::default();
            textedit.editor_operation(&EditorOperation::InsertString(memo));
            textedit.editor_operation(&EditorOperation::BufferHead);
            let model = Box::new(textedit);
            world.add(model);
        }
        let look_at = 0;
        world.look_at(look_at, CameraAdjustment::FitBoth);
        world.re_layout();

        Self { store, world, ime }
    }
}

impl SimpleStateCallback for MemoPadCallback {
    fn init(
        &mut self,
        glyph_vertex_buffer: &mut GlyphVertexBuffer,
        _color_theme: &ColorTheme,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let chars = self
            .world
            .strings()
            .join("")
            .chars()
            .collect::<HashSet<char>>();
        glyph_vertex_buffer
            .append_glyph(device, queue, chars)
            .unwrap();
        self.world.look_at(0, CameraAdjustment::FitBoth);
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.world.change_window_size((width, height));
    }

    fn update(
        &mut self,
        glyph_vertex_buffer: &mut GlyphVertexBuffer,
        color_theme: &ColorTheme,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        self.world
            .update(color_theme, glyph_vertex_buffer, device, queue);
        self.ime
            .update(color_theme, glyph_vertex_buffer, device, queue);
    }

    fn input(&mut self, event: &WindowEvent) -> InputResult {
        match self.store.winit_window_event_to_action(event) {
            Some(Action::Command(category, name)) => match &*category.to_string() {
                "system" => {
                    let action = match &*name.to_string() {
                        "exit" => {
                            let memos = Memos {
                                memos: self.world.strings(),
                            };
                            save_memos(memos).unwrap();
                            return InputResult::SendExit;
                        }
                        "toggle-fullscreen" => {
                            return InputResult::ToggleFullScreen;
                        }
                        "change-theme-dark" => {
                            return InputResult::ChangeColorTheme(ColorTheme::SolarizedDark);
                        }
                        "change-theme-light" => {
                            return InputResult::ChangeColorTheme(ColorTheme::SolarizedLight);
                        }
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
                        "buffer-head" => EditorOperation::BufferHead,
                        "buffer-last" => EditorOperation::BufferLast,
                        "paste" => {
                            match ClipboardContext::new()
                                .and_then(|mut context| context.get_contents())
                            {
                                Ok(text) => EditorOperation::InsertString(text),
                                Err(_) => EditorOperation::Noop,
                            }
                        }
                        "copy" => EditorOperation::Copy(|text| {
                            let _ = ClipboardContext::new()
                                .and_then(|mut context| context.set_contents(text));
                        }),
                        "cut" => EditorOperation::Cut(|text| {
                            let _ = ClipboardContext::new()
                                .and_then(|mut context| context.set_contents(text));
                        }),
                        "mark" => EditorOperation::Mark,
                        _ => EditorOperation::Noop,
                    };
                    self.world.editor_operation(&action);

                    InputResult::InputConsumed
                }
                "world" => {
                    match &*name.to_string() {
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
                        "increase-char-interval" => self
                            .world
                            .model_operation(&ModelOperation::IncreaseCharInterval),
                        "decrease-char-interval" => self
                            .world
                            .model_operation(&ModelOperation::DecreaseCharInterval),
                        _ => {}
                    };
                    InputResult::InputConsumed
                }
                "memopad" => {
                    match &*name.to_string() {
                        "save" => {
                            let memos = Memos {
                                memos: self.world.strings(),
                            };
                            save_memos(memos).unwrap();
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
                        _ => {}
                    };
                    InputResult::InputConsumed
                }
                _ => InputResult::Noop,
            },
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

struct Memos {
    memos: Vec<String>,
}

fn memos_file() -> PathBuf {
    // いわゆるホームディレクトリのパスを取得する
    let home_dir = dirs::home_dir().unwrap();
    Path::new(&home_dir).join(".config/memopad/memos.json")
}

// $HOME/.config/memopad/memos.json に保存されたメモを読み込む
fn load_memos() -> Memos {
    let memos_file = memos_file();
    let memos: Vec<String>;

    if memos_file.exists() {
        // Read memos from file
        let memos_json = fs::read_to_string(memos_file).unwrap();
        memos = serde_json::from_str(&memos_json).unwrap();
    } else {
        // ファイルが存在しない時は、親ディレクトリまで作成してからファイルを作る
        let memos_dir = memos_file.parent().unwrap();
        fs::create_dir_all(memos_dir).unwrap();

        // Set memos to [""] and save to file
        memos = vec!["".to_string()];
        let memos_json = serde_json::to_string(&memos).unwrap();
        fs::write(memos_file, memos_json).unwrap();
    }
    Memos { memos }
}

fn save_memos(memos: Memos) -> Result<(), std::io::Error> {
    if load_memos().memos == memos.memos {
        return Ok(());
    }

    let memos_file = memos_file();
    // 上記のファイルを memos.[現在日時].json にリネームして保存する
    let now = chrono::Local::now();
    let memos_file_backup =
        memos_file.with_extension(format!("{}.json", now.format("%Y%m%d%H%M%S")));
    fs::rename(&memos_file, memos_file_backup)?;

    let memos_json = serde_json::to_string(&memos.memos).unwrap();
    fs::write(memos_file, memos_json)
}
