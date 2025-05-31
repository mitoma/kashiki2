use thiserror::Error;

use crate::MemoId;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum LabeledMemosError {
    #[error("MemoId is not found. MemoId: {0:?}")]
    MemoIdNotFound(MemoId),

    #[error("Conflict MemoId")]
    ConflictMemoIds,
}
