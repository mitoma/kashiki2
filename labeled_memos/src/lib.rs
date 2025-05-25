use serde_derive::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
#[serde(transparent)]
pub struct WorkspaceId(Uuid);

impl WorkspaceId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl From<Uuid> for WorkspaceId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl TryFrom<String> for WorkspaceId {
    type Error = uuid::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Uuid::parse_str(&value).map(Self)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
#[serde(transparent)]
pub struct MemoId(Uuid);

impl MemoId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl From<Uuid> for MemoId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl TryFrom<String> for MemoId {
    type Error = uuid::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Uuid::parse_str(&value).map(Self)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LabeledMemos {
    pub current_workspace_id: WorkspaceId,
    pub workspaces: Vec<Workspace>,
    pub memos: Vec<Memo>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub selected_labels: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub memos_order: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Memo {
    pub id: MemoId,
    pub title: String,
    pub content: String,
    pub labels: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[cfg(test)]
mod tests {
    use crate::{LabeledMemos, Memo, MemoId, Workspace, WorkspaceId};

    #[test]
    fn test_taggable_memos_serialization() {
        // テストデータを作成
        let workspaces = vec![Workspace {
            id: WorkspaceId::new(),
            name: "Default Workspace".to_string(),
            selected_labels: vec!["tag1".to_string(), "tag2".to_string()],
            created_at: "2023-10-01T12:00:00Z".to_string(),
            updated_at: "2023-10-01T12:00:00Z".to_string(),
            memos_order: vec!["memo1".to_string()],
        }];

        let memos = vec![Memo {
            id: MemoId::new(),
            title: "Test Memo".to_string(),
            content: "This is a test memo.".to_string(),
            created_at: "2023-10-01T12:00:00Z".to_string(),
            updated_at: "2023-10-01T12:00:00Z".to_string(),
            labels: vec!["tag1".to_string()],
        }];

        let taggable_memos = LabeledMemos {
            current_workspace_id: workspaces[0].id.clone(),
            workspaces,
            memos,
        };

        // シリアライズ
        let serialized = serde_json::to_string_pretty(&taggable_memos).unwrap();
        println!("Serialized: {}", serialized);

        // デシリアライズ
        let deserialized: LabeledMemos = serde_json::from_str(&serialized).unwrap();

        // 検証
        assert_eq!(
            taggable_memos.current_workspace_id,
            deserialized.current_workspace_id
        );
        assert_eq!(
            taggable_memos.workspaces.len(),
            deserialized.workspaces.len()
        );
        assert_eq!(taggable_memos.memos.len(), deserialized.memos.len());
    }
}
