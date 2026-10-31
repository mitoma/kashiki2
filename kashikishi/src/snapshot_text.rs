use std::fmt::Write;

use ui_support::layout_engine::{DebugModelDetails, DebugModelNode, World};

/// E2E テスト用に World の論理状態をテキストにする。
/// 座標などの浮動小数点値は GPU や環境による差が出やすいので含めず、構造・テキスト・選択状態だけを出力する。
pub(crate) fn world_snapshot_text(world_name: &str, world: &dyn World) -> String {
    let snapshot = world.debug_snapshot();
    let mut out = String::new();
    let _ = writeln!(out, "world: {}", world_name);
    let _ = writeln!(out, "layout: {}", snapshot.layout);
    let _ = writeln!(out, "direction: {}", snapshot.direction);
    let _ = writeln!(out, "focus: {}", snapshot.focus);
    let _ = writeln!(out, "models: {}", snapshot.models.len());
    for (index, model) in snapshot.models.iter().enumerate() {
        write_node(&mut out, model, 1, &format!("[{}]", index));
    }
    let _ = writeln!(out, "modal_models: {}", snapshot.modal_models.len());
    for (index, model) in snapshot.modal_models.iter().enumerate() {
        write_node(&mut out, model, 1, &format!("[{}]", index));
    }
    out
}

fn write_node(out: &mut String, node: &DebugModelNode, depth: usize, label: &str) {
    let indent = "  ".repeat(depth);
    let _ = writeln!(out, "{}{} {}", indent, label, node.name);
    let detail_indent = "  ".repeat(depth + 1);
    match &node.details {
        DebugModelDetails::None | DebugModelDetails::SingleSvg(_) => {}
        DebugModelDetails::StackLayout(stack) => {
            let _ = writeln!(
                out,
                "{}focus_model_index: {:?}",
                detail_indent, stack.focus_model_index
            );
        }
        DebugModelDetails::TextEdit(text_edit) => {
            let _ = writeln!(out, "{}direction: {}", detail_indent, text_edit.direction);
            let _ = writeln!(out, "{}text: {:?}", detail_indent, text_edit.text);
        }
        DebugModelDetails::TextInput(text_input) => {
            let _ = writeln!(
                out,
                "{}default_input: {:?}",
                detail_indent, text_input.default_input
            );
        }
        DebugModelDetails::SelectBox(select_box) => {
            let _ = writeln!(
                out,
                "{}selection: {} / narrowed: {} / options: {}",
                detail_indent,
                select_box.current_selection,
                select_box.narrowed_option_count,
                select_box.option_count
            );
        }
    }
    for (index, child) in node.children.iter().enumerate() {
        write_node(out, child, depth + 1, &format!("[{}]", index));
    }
}
