use stroke_parser::{ActionArgument, CommandName, CommandNamespace};
use text_buffer::action::EditorOperation;

use crate::{context::StateContext, layout_engine::World, support::InputResult};

use super::ActionProcessor;

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

// 編集系の処理。Copy, Paste, Cut は OS 依存の処理なのでここでは定義しない。
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
