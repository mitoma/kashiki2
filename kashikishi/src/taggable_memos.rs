use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct TaggableMemos {
    pub current_workspace_id: String,
    pub workspaces: HashMap<String, Workspace>,
    pub memos: HashMap<String, Memo>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Workspace {
    pub name: String,
    pub selected_tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub memos_order: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Memo {
    pub title: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
    pub tags: Vec<String>,
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::taggable_memos::{Memo, TaggableMemos, Workspace};

    #[test]
    fn test_taggable_memos_serialization() {
        // テストデータを作成
        let mut workspaces = HashMap::new();
        workspaces.insert(
            "workspace1".to_string(),
            Workspace {
                name: "Default Workspace".to_string(),
                selected_tags: vec!["tag1".to_string(), "tag2".to_string()],
                created_at: "2023-10-01T12:00:00Z".to_string(),
                updated_at: "2023-10-01T12:00:00Z".to_string(),
                memos_order: vec!["memo1".to_string()],
            },
        );

        let mut memos = HashMap::new();
        memos.insert(
            "memo1".to_string(),
            Memo {
                title: "Test Memo".to_string(),
                content: "This is a test memo.".to_string(),
                created_at: "2023-10-01T12:00:00Z".to_string(),
                updated_at: "2023-10-01T12:00:00Z".to_string(),
                tags: vec!["tag1".to_string()],
            },
        );

        let taggable_memos = TaggableMemos {
            current_workspace_id: "workspace1".to_string(),
            workspaces,
            memos,
        };

        // シリアライズ
        let serialized = serde_json::to_string_pretty(&taggable_memos).unwrap();
        println!("Serialized: {}", serialized);

        // デシリアライズ
        let deserialized: TaggableMemos = serde_json::from_str(&serialized).unwrap();

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
