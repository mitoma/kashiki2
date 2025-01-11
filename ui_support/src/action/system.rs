use std::{
    io::{BufRead, BufReader},
    path::PathBuf,
};

use log::info;
use stroke_parser::{Action, ActionArgument, CommandName, CommandNamespace};

use font_rasterizer::{
    color_theme::ColorTheme,
    context::{StateContext, WindowSize},
    glyph_vertex_buffer::Direction,
};

use crate::{
    camera::CameraAdjustment,
    layout_engine::{Model, World},
    ui::{SelectBox, SelectOption},
};

use super::{ActionProcessor, InputResult};

pub struct SystemExit;
impl ActionProcessor for SystemExit {
    fn namespace(&self) -> CommandNamespace {
        "system".into()
    }

    fn name(&self) -> CommandName {
        "exit".into()
    }

    fn process(
        &self,
        _arg: &ActionArgument,
        _context: &StateContext,
        _world: &mut dyn World,
    ) -> InputResult {
        InputResult::SendExit
    }
}

pub struct SystemToggleFullscreen;
impl ActionProcessor for SystemToggleFullscreen {
    fn namespace(&self) -> CommandNamespace {
        "system".into()
    }

    fn name(&self) -> CommandName {
        "toggle-fullscreen".into()
    }

    fn process(
        &self,
        _arg: &ActionArgument,
        _context: &StateContext,
        _world: &mut dyn World,
    ) -> InputResult {
        InputResult::ToggleFullScreen
    }
}

pub struct SystemToggleTitlebar;
impl ActionProcessor for SystemToggleTitlebar {
    fn namespace(&self) -> CommandNamespace {
        "system".into()
    }

    fn name(&self) -> CommandName {
        "toggle-titlebar".into()
    }

    fn process(
        &self,
        _arg: &ActionArgument,
        _context: &StateContext,
        _world: &mut dyn World,
    ) -> InputResult {
        InputResult::ToggleDecorations
    }
}

pub struct SystemChangeThemeUi;
impl ActionProcessor for SystemChangeThemeUi {
    fn namespace(&self) -> CommandNamespace {
        "system".into()
    }

    fn name(&self) -> CommandName {
        "change-theme-ui".into()
    }

    fn process(
        &self,
        _arg: &ActionArgument,
        context: &StateContext,
        world: &mut dyn World,
    ) -> InputResult {
        let options = vec![
            SelectOption::new(
                "Solarized Blackback".to_string(),
                Action::new_command_with_argument("system", "change-theme", "black"),
            ),
            SelectOption::new(
                "Solarized Dark".to_string(),
                Action::new_command_with_argument("system", "change-theme", "dark"),
            ),
            SelectOption::new(
                "Solarized Light".to_string(),
                Action::new_command_with_argument("system", "change-theme", "light"),
            ),
        ];
        let model = SelectBox::new(
            context,
            "カラーテーマを選択して下さい".to_string(),
            options,
            None,
        );
        context.ui_string_sender.send(model.to_string()).unwrap();
        world.add_next(Box::new(model));
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

pub struct SystemChangeTheme;
impl ActionProcessor for SystemChangeTheme {
    fn namespace(&self) -> CommandNamespace {
        "system".into()
    }

    fn name(&self) -> CommandName {
        "change-theme".into()
    }

    fn process(
        &self,
        arg: &ActionArgument,
        _context: &StateContext,
        _world: &mut dyn World,
    ) -> InputResult {
        if let ActionArgument::String(theme) = arg {
            let theme = match theme.as_str() {
                "light" => ColorTheme::SolarizedLight,
                "dark" => ColorTheme::SolarizedDark,
                "black" => ColorTheme::SolarizedBlackback,
                _ => return InputResult::Noop,
            };
            InputResult::ChangeColorTheme(theme)
        } else {
            InputResult::Noop
        }
    }
}

pub struct SystemChangeGlobalDirection;
impl ActionProcessor for SystemChangeGlobalDirection {
    fn namespace(&self) -> CommandNamespace {
        "system".into()
    }

    fn name(&self) -> CommandName {
        "change-global-direction".into()
    }

    fn process(
        &self,
        _arg: &ActionArgument,
        context: &StateContext,
        _world: &mut dyn World,
    ) -> InputResult {
        InputResult::ChangeGlobalDirection(context.global_direction.toggle())
    }
}

pub struct SystemChangeWindowSizeUi;
impl ActionProcessor for SystemChangeWindowSizeUi {
    fn namespace(&self) -> CommandNamespace {
        "system".into()
    }

    fn name(&self) -> CommandName {
        "change-window-size-ui".into()
    }

    fn process(
        &self,
        _arg: &ActionArgument,
        context: &StateContext,
        world: &mut dyn World,
    ) -> InputResult {
        let options = vec![
            SelectOption::new(
                "800x600 [4:3]".to_string(),
                Action::Command(
                    "system".into(),
                    "change-window-size".into(),
                    ActionArgument::Point((800.0, 600.0)),
                ),
            ),
            SelectOption::new(
                "1200x900 [4:3]".to_string(),
                Action::Command(
                    "system".into(),
                    "change-window-size".into(),
                    ActionArgument::Point((1200.0, 900.0)),
                ),
            ),
            SelectOption::new(
                "800x450 [16:9]".to_string(),
                Action::Command(
                    "system".into(),
                    "change-window-size".into(),
                    ActionArgument::Point((800.0, 450.0)),
                ),
            ),
            SelectOption::new(
                "1200x675 [16:9]".to_string(),
                Action::Command(
                    "system".into(),
                    "change-window-size".into(),
                    ActionArgument::Point((1200.0, 675.0)),
                ),
            ),
            SelectOption::new(
                "500x500 [1:1]".to_string(),
                Action::Command(
                    "system".into(),
                    "change-window-size".into(),
                    ActionArgument::Point((500.0, 500.0)),
                ),
            ),
            SelectOption::new(
                "1000x1000 [1:1]".to_string(),
                Action::Command(
                    "system".into(),
                    "change-window-size".into(),
                    ActionArgument::Point((1000.0, 1000.0)),
                ),
            ),
        ];
        let model = SelectBox::new(
            context,
            "画面サイズを選択してください".to_string(),
            options,
            None,
        );
        context.ui_string_sender.send(model.to_string()).unwrap();
        world.add_next(Box::new(model));
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

pub struct SystemChangeWindowSize;
impl ActionProcessor for SystemChangeWindowSize {
    fn namespace(&self) -> CommandNamespace {
        "system".into()
    }

    fn name(&self) -> CommandName {
        "change-window-size".into()
    }

    fn process(
        &self,
        arg: &ActionArgument,
        _context: &StateContext,
        _world: &mut dyn World,
    ) -> InputResult {
        match *arg {
            ActionArgument::Point((width, height)) => {
                InputResult::ChangeWindowSize(WindowSize::new(width as u32, height as u32))
            }
            _ => InputResult::Noop,
        }
    }
}

pub struct SystemChangeFontUi;
impl ActionProcessor for SystemChangeFontUi {
    fn namespace(&self) -> CommandNamespace {
        "system".into()
    }

    fn name(&self) -> CommandName {
        "change-font-ui".into()
    }

    fn process(
        &self,
        _arg: &ActionArgument,
        context: &StateContext,
        world: &mut dyn World,
    ) -> InputResult {
        let mut options = vec![SelectOption::new(
            "デフォルトフォント".to_string(),
            Action::new_command("system", "change-font"),
        )];
        options.extend(
            context
                .font_repository
                .list_font_names()
                .iter()
                .map(|name| {
                    SelectOption::new(
                        name.clone(),
                        Action::new_command_with_argument("system", "change-font", name),
                    )
                }),
        );

        let model = SelectBox::new_without_action_name(
            context,
            "フォントを選択してください".to_string(),
            options,
            None,
        );
        context.ui_string_sender.send(model.to_string()).unwrap();
        world.add_next(Box::new(model));
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

pub struct SystemChangeFont;
impl ActionProcessor for SystemChangeFont {
    fn namespace(&self) -> CommandNamespace {
        "system".into()
    }

    fn name(&self) -> CommandName {
        "change-font".into()
    }

    fn process(
        &self,
        arg: &ActionArgument,
        _context: &StateContext,
        _world: &mut dyn World,
    ) -> InputResult {
        if let ActionArgument::String(font_name) = arg {
            InputResult::ChangeFont(Some(font_name.clone()))
        } else {
            InputResult::ChangeFont(None)
        }
    }
}

pub struct SystemSelectBackgroundImageUi;
impl ActionProcessor for SystemSelectBackgroundImageUi {
    fn namespace(&self) -> CommandNamespace {
        "system".into()
    }

    fn name(&self) -> CommandName {
        "select-background-image-ui".into()
    }

    fn process(
        &self,
        arg: &ActionArgument,
        context: &StateContext,
        world: &mut dyn World,
    ) -> InputResult {
        let mut options = Vec::new();

        let current_dir = if let ActionArgument::String(path) = arg {
            PathBuf::from(path)
        } else {
            std::env::current_dir().unwrap()
        };

        if let Some(parent_dir) = current_dir.parent() {
            options.push(SelectOption::new(
                "📁 ../".to_string(),
                Action::new_command_with_argument(
                    "system",
                    "select-background-image-ui",
                    parent_dir.to_str().unwrap(),
                ),
            ));
        }

        let Ok(dir) = std::fs::read_dir(current_dir) else {
            // TODO ここで UI にエラーを表示できるといいな。world にエラーを表示するメソッドを追加する？
            return InputResult::Noop;
        };

        for entry in dir {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                options.push(SelectOption::new(
                    format!("📁 {}", path.file_name().unwrap().to_str().unwrap()),
                    Action::new_command_with_argument(
                        "system",
                        "select-background-image-ui",
                        path.to_str().unwrap(),
                    ),
                ));
            } else if path.is_file() {
                info!("path: {:?}", path);

                let is_image_file = std::fs::File::open(&path)
                    .map(|f| {
                        let mut buf_reader = BufReader::new(f);
                        let buf = buf_reader.fill_buf().unwrap();
                        let format = image::guess_format(buf);
                        info!("format: {:?}", format);
                        format.is_ok()
                    })
                    .unwrap_or(false);
                if !is_image_file {
                    continue;
                }
                options.push(SelectOption::new(
                    format!("🖼 {}", path.file_name().unwrap().to_str().unwrap()),
                    Action::new_command_with_argument(
                        "system",
                        "change-background-image",
                        path.to_str().unwrap(),
                    ),
                ));
            }
        }

        options.push(SelectOption::new(
            "🚫 背景画像を使わない".to_string(),
            Action::new_command("system", "change-background-image"),
        ));

        let model = SelectBox::new_without_action_name(
            context,
            "背景画像を選択する".to_string(),
            options,
            None,
        );
        let model_string = model.to_string();
        context.ui_string_sender.send(model_string).unwrap();

        world.add_next(Box::new(model));
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

pub struct SystemChangeBackgroundImage;
impl ActionProcessor for SystemChangeBackgroundImage {
    fn namespace(&self) -> CommandNamespace {
        "system".into()
    }

    fn name(&self) -> CommandName {
        "change-background-image".into()
    }

    fn process(
        &self,
        arg: &ActionArgument,
        _context: &StateContext,
        _world: &mut dyn World,
    ) -> InputResult {
        match arg {
            ActionArgument::String(path) => {
                let image =
                    image::open(path).unwrap_or_else(|_| panic!("Failed to open image: {}", path));
                InputResult::ChangeBackgroundImage(Some(image))
            }
            _ => InputResult::ChangeBackgroundImage(None),
        }
    }
}
