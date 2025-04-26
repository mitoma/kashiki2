use chrono::{DateTime, Local, SubsecRound};
use serde_derive::{Deserialize, Serialize};
use uuid::Uuid;

fn now() -> DateTime<Local> {
    Local::now().round_subsecs(0)
}

macro_rules! define_id_of_uuid_v7 {
    ($name:ident) => {
        #[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }
        }

        impl From<Uuid> for $name {
            fn from(uuid: Uuid) -> Self {
                Self(uuid)
            }
        }

        impl TryFrom<String> for $name {
            type Error = uuid::Error;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Uuid::parse_str(&value).map(Self)
            }
        }
    };
}

// マクロを使用して構造体を定義
define_id_of_uuid_v7!(WorkspaceId);
define_id_of_uuid_v7!(MemoId);
define_id_of_uuid_v7!(LabelId);

// 定数の定義
const UNLABELED_ID: LabelId = LabelId(Uuid::from_u128(0));

#[derive(Serialize, Deserialize, Debug)]
pub struct LabeledMemos {
    pub current_workspace_id: WorkspaceId,
    pub workspaces: Vec<Workspace>,
    pub memos: Vec<Memo>,
    pub labels: Vec<Label>,
}

impl LabeledMemos {
    pub fn new() -> Self {
        let labels = vec![Label {
            id: UNLABELED_ID,
            name: "未分類".to_string(),
        }];
        let mut memo = Memo::new();
        memo.labels.push(labels[0].id);
        let mut workspace = Workspace::new("".to_string());

        workspace.memos_order.push(memo.id);

        Self {
            current_workspace_id: workspace.id,
            workspaces: vec![workspace],
            memos: vec![],
            labels: vec![],
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub selected_labels: Vec<LabelId>,
    pub created_at: DateTime<Local>,
    pub updated_at: DateTime<Local>,
    pub memos_order: Vec<MemoId>,
}

impl Workspace {
    pub fn new(name: String) -> Self {
        Self {
            id: WorkspaceId::new(),
            name,
            selected_labels: vec![],
            created_at: now(),
            updated_at: now(),
            memos_order: vec![],
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Memo {
    pub id: MemoId,
    pub title: String,
    pub content: String,
    pub labels: Vec<LabelId>,
    pub created_at: DateTime<Local>,
    pub updated_at: DateTime<Local>,
}

impl Memo {
    pub fn new() -> Self {
        Self {
            id: MemoId::new(),
            title: "".to_string(),
            content: "".to_string(),
            labels: vec![],
            created_at: now(),
            updated_at: now(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Label {
    pub id: LabelId,
    pub name: String,
}

#[cfg(test)]
mod tests {
    use chrono::{Local, SubsecRound};

    use crate::{Label, LabelId, LabeledMemos, Memo, MemoId, Workspace, WorkspaceId};

    #[test]
    fn test_taggable_memos_serialization() {
        // テストデータを作成
        let labels = vec![
            Label {
                id: LabelId::new(),
                name: "label1".to_string(),
            },
            Label {
                id: LabelId::new(),
                name: "label2".to_string(),
            },
        ];

        let memos = vec![Memo {
            id: MemoId::new(),
            title: "Test Memo".to_string(),
            content: "This is a test memo.".to_string(),
            created_at: Local::now().round_subsecs(0),
            updated_at: Local::now().round_subsecs(0),
            labels: vec![labels[0].id],
        }];

        let workspaces = vec![Workspace {
            id: WorkspaceId::new(),
            name: "Default Workspace".to_string(),
            selected_labels: vec![labels[0].id, labels[1].id],
            created_at: Local::now().round_subsecs(0),
            updated_at: Local::now().round_subsecs(0),
            memos_order: vec![memos[0].id],
        }];

        let taggable_memos = LabeledMemos {
            current_workspace_id: workspaces[0].id.clone(),
            workspaces,
            memos,
            labels,
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
