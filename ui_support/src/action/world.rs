use stroke_parser::{Action, ActionArgument, CommandName, CommandNamespace};
use text_buffer::action::EditorOperation;

use font_rasterizer::{context::StateContext, glyph_vertex_buffer::Direction};

use crate::{
    action::add_model_to_world,
    camera::{CameraAdjustment, CameraOperation},
    layout_engine::{ModelBorder, ModelOperation, World, WorldLayout},
    ui::TextInput,
};

use super::{ActionProcessor, InputResult};

macro_rules! world_processor {
    ( $proc_name:ident, $name:expr, $world_operation:ident ) => {
        pub struct $proc_name;
        impl ActionProcessor for $proc_name {
            fn namespace(&self) -> CommandNamespace {
                "world".into()
            }

            fn name(&self) -> CommandName {
                $name.into()
            }

            fn process(
                &self,
                arg: &ActionArgument,
                context: &StateContext,
                world: &mut dyn World,
            ) -> InputResult {
                $world_operation(arg, context, world);
                InputResult::InputConsumed
            }
        }
    };
}

world_processor!(WorldRemoveCurrent, "remove-current", remove_current);
fn remove_current(_arg: &ActionArgument, _context: &StateContext, world: &mut dyn World) {
    world.remove_current();
    world.re_layout();
    world.look_prev(CameraAdjustment::NoCare);
}

world_processor!(WorldResetZoom, "reset-zoom", reset_zoom);
fn reset_zoom(_arg: &ActionArgument, _context: &StateContext, world: &mut dyn World) {
    world.look_current(CameraAdjustment::FitBoth);
}

world_processor!(WorldLookCurrent, "look-current", look_current);
fn look_current(_arg: &ActionArgument, _context: &StateContext, world: &mut dyn World) {
    world.look_current(CameraAdjustment::NoCare);
}

world_processor!(WorldLookNext, "look-next", look_next);
fn look_next(_arg: &ActionArgument, _context: &StateContext, world: &mut dyn World) {
    world.look_next(CameraAdjustment::NoCare);
}

world_processor!(WorldLookPrev, "look-prev", look_prev);
fn look_prev(_arg: &ActionArgument, _context: &StateContext, world: &mut dyn World) {
    world.look_prev(CameraAdjustment::NoCare);
}

world_processor!(WorldSwapNext, "swap-next", swap_next);
fn swap_next(_arg: &ActionArgument, _context: &StateContext, world: &mut dyn World) {
    world.swap_next();
}

world_processor!(WorldSwapPrev, "swap-prev", swap_prev);
fn swap_prev(_arg: &ActionArgument, _context: &StateContext, world: &mut dyn World) {
    world.swap_prev();
}

world_processor!(WorldFitWidth, "fit-width", fit_width);
fn fit_width(_arg: &ActionArgument, _context: &StateContext, world: &mut dyn World) {
    world.look_current(CameraAdjustment::FitWidth);
}

world_processor!(WorldFitHeight, "fit-height", fit_height);
fn fit_height(_arg: &ActionArgument, _context: &StateContext, world: &mut dyn World) {
    world.look_current(CameraAdjustment::FitHeight);
}

world_processor!(WorldFitByDirection, "fit-by-direction", fit_by_direction);
fn fit_by_direction(_arg: &ActionArgument, context: &StateContext, world: &mut dyn World) {
    if context.global_direction == Direction::Horizontal {
        world.look_current(CameraAdjustment::FitWidth)
    } else {
        world.look_current(CameraAdjustment::FitHeight)
    }
}

world_processor!(WorldForward, "forward", forward);
fn forward(_arg: &ActionArgument, _context: &StateContext, world: &mut dyn World) {
    world.camera_operation(CameraOperation::Forward);
}

world_processor!(WorldBack, "back", back);
fn back(_arg: &ActionArgument, _context: &StateContext, world: &mut dyn World) {
    world.camera_operation(CameraOperation::Backward);
}

world_processor!(WorldChangeDirection, "change-direction", change_direction);
fn change_direction(_arg: &ActionArgument, _context: &StateContext, world: &mut dyn World) {
    world.model_operation(&ModelOperation::ChangeDirection(None));
}

world_processor!(
    WorldIncreaseRowInterval,
    "increase-row-interval",
    increase_row_interval
);
fn increase_row_interval(_arg: &ActionArgument, _context: &StateContext, world: &mut dyn World) {
    world.model_operation(&ModelOperation::IncreaseRowInterval);
}

world_processor!(
    WorldDecreaseRowInterval,
    "decrease-row-interval",
    decrease_row_interval
);
fn decrease_row_interval(_arg: &ActionArgument, _context: &StateContext, world: &mut dyn World) {
    world.model_operation(&ModelOperation::DecreaseRowInterval);
}

world_processor!(
    WorldIncreaseColInterval,
    "increase-col-interval",
    increase_col_interval
);
fn increase_col_interval(_arg: &ActionArgument, _context: &StateContext, world: &mut dyn World) {
    world.model_operation(&ModelOperation::IncreaseColInterval);
}

world_processor!(
    WorldDecreaseColInterval,
    "decrease-col-interval",
    decrease_col_interval
);
fn decrease_col_interval(_arg: &ActionArgument, _context: &StateContext, world: &mut dyn World) {
    world.model_operation(&ModelOperation::DecreaseColInterval);
}

world_processor!(
    WorldIncreaseColScale,
    "increase-col-scale",
    increase_col_scale
);
fn increase_col_scale(_arg: &ActionArgument, _context: &StateContext, world: &mut dyn World) {
    world.model_operation(&ModelOperation::IncreaseColScale);
}

world_processor!(
    WorldDecreaseColScale,
    "decrease-col-scale",
    decrease_col_scale
);
fn decrease_col_scale(_arg: &ActionArgument, _context: &StateContext, world: &mut dyn World) {
    world.model_operation(&ModelOperation::DecreaseColScale);
}

world_processor!(
    WorldIncreaseRowScale,
    "increase-row-scale",
    increase_row_scale
);
fn increase_row_scale(_arg: &ActionArgument, _context: &StateContext, world: &mut dyn World) {
    world.model_operation(&ModelOperation::IncreaseRowScale);
}

world_processor!(
    WorldDecreaseRowScale,
    "decrease-row-scale",
    decrease_row_scale
);
fn decrease_row_scale(_arg: &ActionArgument, _context: &StateContext, world: &mut dyn World) {
    world.model_operation(&ModelOperation::DecreaseRowScale);
}

world_processor!(WorldToggleMinBound, "toggle-min-bound", toggle_min_bound);
fn toggle_min_bound(_arg: &ActionArgument, _context: &StateContext, world: &mut dyn World) {
    world.model_operation(&ModelOperation::ToggleMinBound);
}

world_processor!(
    WorldTogglePsychedelic,
    "toggle-psychedelic",
    toggle_psychedelic
);
fn toggle_psychedelic(_arg: &ActionArgument, _context: &StateContext, world: &mut dyn World) {
    world.model_operation(&ModelOperation::TogglePsychedelic);
}

world_processor!(WorldMoveToClick, "move-to-click", move_to_click);
fn move_to_click(arg: &ActionArgument, context: &StateContext, world: &mut dyn World) {
    match arg {
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

world_processor!(
    WorldMoveToClickWithMark,
    "move-to-click-with-mark",
    move_to_click_with_mark
);
fn move_to_click_with_mark(arg: &ActionArgument, context: &StateContext, world: &mut dyn World) {
    match arg {
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

world_processor!(WorldChangeLayout, "change-layout", change_layout);
fn change_layout(arg: &ActionArgument, _context: &StateContext, world: &mut dyn World) {
    match arg {
        ActionArgument::String(layout_name) => {
            let layout = match layout_name.as_str() {
                //"grid" => world.model_operation(&ModelOperation::ChangeLayout("grid")),
                "line" => WorldLayout::Liner,
                "circle" => WorldLayout::Circle,
                _ => WorldLayout::Liner,
            };
            world.change_layout(layout);
        }
        _ => world.change_layout(world.layout().next()),
    }
}

world_processor!(WorldSetModelBorder, "set-model-border", set_model_border);
fn set_model_border(_arg: &ActionArgument, _context: &StateContext, world: &mut dyn World) {
    world.model_operation(&ModelOperation::SetModelBorder(ModelBorder::Rounded));
}

world_processor!(
    WorldUnsetModelBorder,
    "unset-model-border",
    unset_model_border
);
fn unset_model_border(_arg: &ActionArgument, _context: &StateContext, world: &mut dyn World) {
    world.model_operation(&ModelOperation::SetModelBorder(ModelBorder::None));
}

world_processor!(WorldChangeMaxColUi, "change-max-col-ui", change_max_col_ui);
fn change_max_col_ui(_arg: &ActionArgument, context: &StateContext, world: &mut dyn World) {
    log::info!("fooo");
    let modal = TextInput::new(
        context,
        "変更後の文字数を指定してください".into(),
        Some("60".into()),
        Action::new_command("world", "change-max-col"),
    );
    add_model_to_world(context, world, Box::new(modal));
}

world_processor!(WorldChangeMaxCol, "change-max-col", change_max_col);
fn change_max_col(arg: &ActionArgument, _context: &StateContext, world: &mut dyn World) {
    log::info!("Changing max col. {}", arg);
    match arg {
        ActionArgument::String2(new_max_col, _) => {
            let new_max_col = new_max_col.parse::<usize>().unwrap_or(60);
            world.model_operation(&ModelOperation::SetMaxCol(new_max_col));
        }
        _ => {
            // Handle invalid argument
        }
    }
}

world_processor!(WorldIncreaseMaxCol, "increase-max-col", increase_max_col);
fn increase_max_col(_arg: &ActionArgument, _context: &StateContext, world: &mut dyn World) {
    world.model_operation(&ModelOperation::IncreaseMaxCol);
}

world_processor!(WorldDecreaseMaxCol, "decrease-max-col", decrease_max_col);
fn decrease_max_col(_arg: &ActionArgument, _context: &StateContext, world: &mut dyn World) {
    world.model_operation(&ModelOperation::DecreaseMaxCol);
}
