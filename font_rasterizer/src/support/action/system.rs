use stroke_parser::{Action, ActionArgument, CommandName, CommandNamespace};

use crate::{
    camera::CameraAdjustment,
    color_theme::ColorTheme,
    context::StateContext,
    font_buffer::Direction,
    layout_engine::{Model, World},
    support::InputResult,
    ui::{SelectBox, SelectOption},
};

use super::ActionProcessor;

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
