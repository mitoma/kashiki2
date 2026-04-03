//! 互換維持対象の公開面:
//! - editor::Editor::new
//! - editor::Editor::operation
//! - editor::Editor::to_buffer_string
//! - editor::Editor::buffer_chars
//! - editor::Editor::calc_phisical_layout
//! - action::EditorOperation
//! - editor::ChangeEvent
//! - caret::Caret / caret::CaretType
//! - buffer::BufferChar / buffer::CellPosition

pub mod action;
pub mod buffer;
pub mod caret;
pub mod char_type;
pub mod editor;
mod notifier;
