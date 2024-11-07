use font_rasterizer::context::StateContext;
use stroke_parser::{ActionArgument, CommandName, CommandNamespace};
use text_buffer::action::EditorOperation;

use crate::layout_engine::World;

use super::{ActionProcessor, InputResult};

macro_rules! edit_processor {
    ( $proc_name:ident, $name:expr, $editor_operation:ident ) => {
        pub struct $proc_name;
        impl ActionProcessor for $proc_name {
            fn namespace(&self) -> CommandNamespace {
                "edit".into()
            }

            fn name(&self) -> CommandName {
                $name.into()
            }

            fn process(
                &self,
                _arg: &ActionArgument,
                _context: &StateContext,
                world: &mut dyn World,
            ) -> InputResult {
                world.editor_operation(&EditorOperation::$editor_operation);
                InputResult::InputConsumed
            }
        }
    };
}

edit_processor!(EditReturn, "return", InsertEnter);
edit_processor!(EditBackspace, "backspace", Backspace);
edit_processor!(EditBackspaceWord, "backspace-word", BackspaceWord);
edit_processor!(EditDelete, "delete", Delete);
edit_processor!(EditDeleteWord, "delete-word", DeleteWord);
edit_processor!(EditPrevious, "previous", Previous);
edit_processor!(EditNext, "next", Next);
edit_processor!(EditBack, "back", Back);
edit_processor!(EditForward, "forward", Forward);
edit_processor!(EditBackWord, "back-word", BackWord);
edit_processor!(EditForwardWord, "forward-word", ForwardWord);
edit_processor!(EditHead, "head", Head);
edit_processor!(EditLast, "last", Last);
edit_processor!(EditUndo, "undo", Undo);
edit_processor!(EditBufferHead, "buffer-head", BufferHead);
edit_processor!(EditBufferLast, "buffer-last", BufferLast);
edit_processor!(EditMark, "mark", Mark);
edit_processor!(EditUnmark, "unmark", UnMark);

// Copy, Paste, Cut は OS 依存の処理なのでここ分岐して定義する。
// wasm で clipboard にアクセスするのは権限周りで制限があるのでひとまず internal な処理にする。

#[cfg(target_arch = "wasm32")]
static CLIPBOARD_FOR_WASM: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

pub struct EditCopy;
impl ActionProcessor for EditCopy {
    fn namespace(&self) -> CommandNamespace {
        "edit".into()
    }

    fn name(&self) -> CommandName {
        "copy".into()
    }

    fn process(
        &self,
        _arg: &ActionArgument,
        _context: &StateContext,
        world: &mut dyn World,
    ) -> InputResult {
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                world.editor_operation(&EditorOperation::Copy(|text| {
                    let mut clipboard = CLIPBOARD_FOR_WASM.lock().unwrap();
                    *clipboard = text;
                }));
            } else {
                world.editor_operation(&EditorOperation::Copy(|text| {
                    let _ = arboard::Clipboard::new().and_then(|mut context| context.set_text(text));
                }));
            }
        }
        InputResult::InputConsumed
    }
}

pub struct EditPaste;
impl ActionProcessor for EditPaste {
    fn namespace(&self) -> CommandNamespace {
        "edit".into()
    }

    fn name(&self) -> CommandName {
        "paste".into()
    }

    fn process(
        &self,
        _arg: &ActionArgument,
        context: &StateContext,
        world: &mut dyn World,
    ) -> InputResult {
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                let clipboard = CLIPBOARD_FOR_WASM.lock().unwrap();
                let text = clipboard.clone();
                world.editor_operation(&EditorOperation::InsertString(text));
            } else {
                match arboard::Clipboard::new().and_then(|mut context| context.get_text()) {
                    Ok(text) => {
                        context.ui_string_sender.send(text.clone()).unwrap();
                        world.editor_operation(&EditorOperation::InsertString(text));
                    }
                    Err(_) => return InputResult::Noop,
                }
            }
        }
        InputResult::InputConsumed
    }
}

pub struct EditCut;
impl ActionProcessor for EditCut {
    fn namespace(&self) -> CommandNamespace {
        "edit".into()
    }

    fn name(&self) -> CommandName {
        "cut".into()
    }

    fn process(
        &self,
        _arg: &ActionArgument,
        _context: &StateContext,
        world: &mut dyn World,
    ) -> InputResult {
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                world.editor_operation(&EditorOperation::Cut(|text| {
                    let mut clipboard = CLIPBOARD_FOR_WASM.lock().unwrap();
                    *clipboard = text;
                }));
            } else {
                world.editor_operation(&EditorOperation::Cut(|text| {
                    let _ = arboard::Clipboard::new().and_then(|mut context| context.set_text(text));
                }));
            }
        }
        InputResult::InputConsumed
    }
}
