use std::collections::HashSet;

use chrono::{DateTime, Local, SubsecRound};
use errors::LabeledMemosError;
use serde_derive::{Deserialize, Serialize};
use uuid::Uuid;

mod errors;

fn now() -> DateTime<Local> {
    Local::now().round_subsecs(0)
}

macro_rules! define_id_of_uuid_v7 {
    ($name:ident) => {
        #[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy, Hash)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

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

pub enum OrderBy {
    CreatedAsc,
    CreatedDesc,
    UpdatedAsc,
    UpdatedDesc,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LabeledMemos {
    pub current_workspace_id: WorkspaceId,
    pub workspaces: Vec<Workspace>,
    pub memos: MemoRepository,
    pub labels: Vec<Label>,
}

impl Default for LabeledMemos {
    fn default() -> Self {
        Self::new()
    }
}

impl LabeledMemos {
    pub fn new() -> Self {
        let labels = vec![Label {
            id: UNLABELED_ID,
            name: "未分類".to_string(),
        }];
        let mut memo = Memo::new();
        memo.labels.push(labels[0].id);
        let mut memo_repository = MemoRepository::default();
        memo_repository.upsert(memo);

        let mut workspace = Workspace::new("".to_string());
        workspace
            .memos_order
            .append(&mut memo_repository.list_memo_ids(&[]));

        Self {
            current_workspace_id: workspace.id,
            workspaces: vec![workspace],
            memos: memo_repository,
            labels,
        }
    }

    fn current_workspace(&self) -> &Workspace {
        self.find_workspace(self.current_workspace_id)
            .expect("Current workspace not found")
    }

    fn current_workspace_mut(&mut self) -> &mut Workspace {
        self.find_workspace_mut(self.current_workspace_id)
            .expect("Current workspace not found")
    }

    fn find_workspace(&self, id: WorkspaceId) -> Option<&Workspace> {
        self.workspaces.iter().find(|workspace| workspace.id == id)
    }

    fn find_workspace_mut(&mut self, id: WorkspaceId) -> Option<&mut Workspace> {
        self.workspaces
            .iter_mut()
            .find(|workspace| workspace.id == id)
    }

    pub fn workspace_memos(&self) -> Vec<&Memo> {
        self.memos.list_memos(&self.current_workspace().memos_order)
    }

    pub fn add_memo(&mut self, memo: Memo) {
        self.current_workspace_mut().memos_order.push(memo.id);
        self.memos.upsert(memo);
    }

    /// 新規メモを作成してワークスペースに追加する
    pub fn add_new_memo(&mut self, title: String, content: String) -> MemoId {
        let mut memo = Memo::new();
        memo.title = title;
        memo.content = content;
        // デフォルトでは未分類ラベルを付与
        if memo.labels.is_empty() {
            memo.labels.push(UNLABELED_ID);
        }
        let memo_id = memo.id;
        self.add_memo(memo);
        memo_id
    }

    /// ワークスペースからメモを削除する
    /// ワークスペースから削除するだけで、メモリポジトリからは削除しない
    pub fn remove_memo_from_workspace(&mut self, memo_id: MemoId) -> Result<(), LabeledMemosError> {
        let workspace = self.current_workspace_mut();
        if let Some(pos) = workspace.memos_order.iter().position(|id| *id == memo_id) {
            workspace.memos_order.remove(pos);
            Ok(())
        } else {
            Err(LabeledMemosError::MemoIdNotFound(memo_id))
        }
    }

    pub fn remove_memo(&mut self, memo_id: MemoId) -> Result<(), LabeledMemosError> {
        self.remove_memo_from_workspace(memo_id)?;
        self.memos.remove(memo_id);
        Ok(())
    }

    /// 既存のメモをワークスペースに追加する
    pub fn add_existing_memo_to_workspace(
        &mut self,
        memo_id: MemoId,
    ) -> Result<(), LabeledMemosError> {
        // メモが存在するか確認
        if !self.memos.contain(memo_id) {
            return Err(LabeledMemosError::MemoIdNotFound(memo_id));
        }

        // 既にワークスペースに含まれているか確認
        let workspace = self.current_workspace_mut();
        if workspace.memos_order.contains(&memo_id) {
            return Ok(());
        }

        // ワークスペースに追加
        workspace.memos_order.push(memo_id);
        Ok(())
    }

    /// 条件で絞り込んだメモを一括でワークスペースに追加する
    pub fn add_memos_by_condition(&mut self, label_ids: &[LabelId]) -> usize {
        // 指定されたラベルを持つメモを検索
        let memo_ids: Vec<MemoId> = self.memos.list_memo_ids(label_ids);

        let mut added_count = 0;

        // ワークスペースに追加
        for memo_id in memo_ids {
            // 既にワークスペース内にないメモだけを追加
            if !self.current_workspace().memos_order.contains(&memo_id) {
                self.current_workspace_mut().memos_order.push(memo_id);
                added_count += 1;
            }
        }

        added_count
    }

    /// ワークスペースのメモをラベルで絞り込む
    pub fn filter_memos_by_labels(&self, label_ids: &[LabelId]) -> Vec<&Memo> {
        // ラベルが指定されていない場合は全てのメモを返す
        if label_ids.is_empty() {
            return self.workspace_memos();
        }

        // 現在のワークスペースに含まれるメモのうち、指定されたラベルのいずれかを持つメモを返す
        self.workspace_memos()
            .into_iter()
            .filter(|memo| {
                memo.labels
                    .iter()
                    .any(|label_id| label_ids.contains(label_id))
            })
            .collect()
    }

    /// ワークスペースのメモを時系列で並べ替える
    pub fn sort_memos_by_time(&mut self, order: OrderBy) {
        let mut workspace_memos = self.memos.list_memos(&self.current_workspace().memos_order);

        workspace_memos.sort_by(|l, r| match order {
            OrderBy::CreatedAsc => l.created_at.cmp(&r.created_at),
            OrderBy::CreatedDesc => r.created_at.cmp(&l.created_at),
            OrderBy::UpdatedAsc => l.updated_at.cmp(&r.updated_at),
            OrderBy::UpdatedDesc => r.updated_at.cmp(&l.updated_at),
        });

        // ソート結果をワークスペースに反映
        let sorted_ids: Vec<MemoId> = workspace_memos.iter().map(|memo| memo.id).collect();
        self.current_workspace_mut().memos_order = sorted_ids;
    }

    /// ワークスペースのメモを任意に並べ替える
    pub fn reorder_workspace_memos(
        &mut self,
        memo_ids: Vec<MemoId>,
    ) -> Result<(), LabeledMemosError> {
        // 指定されたメモIDがすべて現在のワークスペースに存在するかを確認
        let current_ids: HashSet<MemoId> = self
            .current_workspace()
            .memos_order
            .iter()
            .cloned()
            .collect();
        let new_ids: HashSet<MemoId> = memo_ids.iter().cloned().collect();

        // ワークスペースに存在しないメモIDが含まれている場合はエラー
        if !new_ids.is_subset(&current_ids) {
            return Err(LabeledMemosError::ConflictMemoIds);
        }

        // 元のワークスペースに含まれるメモIDがすべて指定されているかを確認
        if new_ids.len() != current_ids.len() {
            return Err(LabeledMemosError::ConflictMemoIds);
        }

        // 並べ替え実行
        self.current_workspace_mut().memos_order = memo_ids;
        Ok(())
    }

    /// ワークスペースのメモを全て削除する
    pub fn clear_workspace(&mut self) {
        self.current_workspace_mut().memos_order.clear();
    }

    /// ワークスペースのメモを全て削除し、ラベルで絞り込んだメモと入れ替える
    pub fn replace_workspace_memos_by_labels(&mut self, label_ids: &[LabelId]) {
        // ワークスペースをクリア
        self.clear_workspace();

        // 指定されたラベルを持つメモを検索して追加
        self.add_memos_by_condition(label_ids);
    }

    pub fn list_labels(&self) -> &[Label] {
        &self.labels
    }

    pub fn upsert_labels(&mut self, label: Label) {
        self.remove_label(label.id);
        self.labels.push(label);
    }

    pub fn remove_label(&mut self, label_id: LabelId) {
        self.labels.retain(|l| l.id != label_id);
    }
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct MemoRepository(Vec<Memo>);

impl MemoRepository {
    pub fn upsert(&mut self, memo: Memo) {
        let memo_id = memo.id;
        if self.contain(memo_id) {
            self.remove(memo_id);
        }
        self.0.push(memo);
    }

    pub fn contain(&self, memo_id: MemoId) -> bool {
        self.get(memo_id).is_some()
    }

    pub fn remove(&mut self, memo_id: MemoId) {
        self.0.retain(|memo| memo.id != memo_id);
    }

    pub fn list_memo_ids(&self, labels: &[LabelId]) -> Vec<MemoId> {
        self.0
            .iter()
            .filter(|memo| {
                labels.is_empty() || labels.iter().any(|label| memo.labels.contains(label))
            })
            .map(|memo| memo.id)
            .collect()
    }

    pub fn list_memos(&self, memo_ids: &[MemoId]) -> Vec<&Memo> {
        self.0
            .iter()
            .filter(|memo| memo_ids.contains(&memo.id))
            .collect()
    }

    pub fn get(&self, memo_id: MemoId) -> Option<&Memo> {
        self.0.iter().find(|memo| memo.id == memo_id)
    }

    pub fn clear(&mut self) {
        self.0.clear();
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Memo {
    pub id: MemoId,
    pub title: String,
    pub content: String,
    pub labels: Vec<LabelId>,
    pub created_at: DateTime<Local>,
    pub updated_at: DateTime<Local>,
}

impl Default for Memo {
    fn default() -> Self {
        Self::new()
    }
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

impl Label {
    pub fn new(name: &str) -> Self {
        Self {
            id: LabelId::new(),
            name: name.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Local, SubsecRound, TimeDelta};

    use crate::{
        Label, LabelId, LabeledMemos, Memo, MemoId, MemoRepository, OrderBy, UNLABELED_ID,
        Workspace, WorkspaceId, errors::LabeledMemosError,
    };

    #[test]
    fn test_taggable_memos_serialization() {
        // テストデータを作成
        let labels = vec![Label::new("label1"), Label::new("label2")];

        let mut memos = MemoRepository::default();
        memos.upsert(Memo {
            id: MemoId::new(),
            title: "Test Memo".to_string(),
            content: "This is a test memo.".to_string(),
            created_at: Local::now().round_subsecs(0),
            updated_at: Local::now().round_subsecs(0),
            labels: vec![labels[0].id],
        });

        let workspaces = vec![Workspace {
            id: WorkspaceId::new(),
            name: "Default Workspace".to_string(),
            selected_labels: vec![labels[0].id, labels[1].id],
            created_at: Local::now().round_subsecs(0),
            updated_at: Local::now().round_subsecs(0),
            memos_order: memos.list_memo_ids(&[labels[0].id, labels[1].id]),
        }];

        let labeled_memos = LabeledMemos {
            current_workspace_id: workspaces[0].id,
            workspaces,
            memos,
            labels,
        };

        // シリアライズ
        let serialized = serde_json::to_string_pretty(&labeled_memos).unwrap();
        println!("Serialized: {}", serialized);

        // デシリアライズ
        let deserialized: LabeledMemos = serde_json::from_str(&serialized).unwrap();

        // 検証
        assert_eq!(
            labeled_memos.current_workspace_id,
            deserialized.current_workspace_id
        );
        assert_eq!(
            labeled_memos.workspaces.len(),
            deserialized.workspaces.len()
        );
        //assert_eq!(labeled_memos.memos, deserialized.memos);
    }

    #[test]
    fn test_add_new_memo() {
        let mut labeled_memos = LabeledMemos::new();
        let title = "新しいメモ".to_string();
        let content = "これは新しいメモの内容です。".to_string();

        let memo_id = labeled_memos.add_new_memo(title.clone(), content.clone());

        // ワークスペースのメモ一覧にメモが追加されたことを確認
        let workspace = labeled_memos.current_workspace();
        assert!(workspace.memos_order.contains(&memo_id));

        // メモが実際に追加されたことを確認
        let added_memo = labeled_memos.memos.get(memo_id);
        assert!(added_memo.is_some());
        let added_memo = added_memo.unwrap();
        assert_eq!(added_memo.title, title);
        assert_eq!(added_memo.content, content);
        assert!(added_memo.labels.contains(&UNLABELED_ID));
    }

    #[test]
    fn test_add_existing_memo_to_workspace() {
        let mut labeled_memos = LabeledMemos::new();

        // 新しいメモを作成
        let title = "既存メモ".to_string();
        let content = "これは既存メモの内容です。".to_string();
        let memo_id = labeled_memos.add_new_memo(title, content);

        // メモをワークスペースから削除
        let _ = labeled_memos.remove_memo_from_workspace(memo_id);

        // 削除されたことを確認
        assert!(
            !labeled_memos
                .current_workspace()
                .memos_order
                .contains(&memo_id)
        );

        // 既存メモをワークスペースに再追加
        let result = labeled_memos.add_existing_memo_to_workspace(memo_id);
        assert!(result.is_ok());

        // 再追加されたことを確認
        assert!(
            labeled_memos
                .current_workspace()
                .memos_order
                .contains(&memo_id)
        );

        // 同じメモを再度追加しようとするとエラーになることを確認
        let result = labeled_memos.add_existing_memo_to_workspace(memo_id);
        assert!(result.is_ok());

        // 存在しないメモを追加しようとするとエラーになることを確認
        let non_existent_memo_id = MemoId::new();
        let result = labeled_memos.add_existing_memo_to_workspace(non_existent_memo_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_memos_by_condition() {
        let mut labeled_memos = LabeledMemos::new();

        // ラベルを作成
        let label1 = Label::new("ラベル1");
        let label1_id = label1.id;
        let label2 = Label::new("ラベル2");
        let label2_id = label2.id;

        labeled_memos.labels.push(label1);
        labeled_memos.labels.push(label2);

        // 複数のメモを作成し、異なるラベルを付与
        let mut memo1 = Memo::new();
        memo1.title = "メモ1".to_string();
        memo1.content = "内容1".to_string();
        memo1.labels = vec![label1_id];
        let memo1_id = memo1.id;

        let mut memo2 = Memo::new();
        memo2.title = "メモ2".to_string();
        memo2.content = "内容2".to_string();
        memo2.labels = vec![label2_id];
        let memo2_id = memo2.id;

        let mut memo3 = Memo::new();
        memo3.title = "メモ3".to_string();
        memo3.content = "内容3".to_string();
        memo3.labels = vec![label1_id, label2_id];
        let memo3_id = memo3.id;

        // メモをリポジトリに追加
        labeled_memos.memos.upsert(memo1);
        labeled_memos.memos.upsert(memo2);
        labeled_memos.memos.upsert(memo3);

        // 現在のワークスペースのメモを全て削除
        let workspace = labeled_memos.current_workspace_mut();
        workspace.memos_order.clear();

        // label1でメモを追加
        let added = labeled_memos.add_memos_by_condition(&[label1_id]);
        assert_eq!(added, 2); // memo1とmemo3が追加される

        // 追加されたメモを確認
        let workspace = labeled_memos.current_workspace();
        assert!(workspace.memos_order.contains(&memo1_id));
        assert!(!workspace.memos_order.contains(&memo2_id)); // label1を持たないmemo2は追加されない
        assert!(workspace.memos_order.contains(&memo3_id));

        // 再度同じ条件で追加しても新たに追加されるメモはない
        let added = labeled_memos.add_memos_by_condition(&[label1_id]);
        assert_eq!(added, 0);

        // ワークスペースをクリア
        labeled_memos.current_workspace_mut().memos_order.clear();

        // 空のラベル配列で検索すると全てのメモが追加される
        let added = labeled_memos.add_memos_by_condition(&[]);
        assert_eq!(added, 4); // ラベルなしの初期のメモも追加されるため 4
    }

    #[test]
    fn test_filter_memos_by_labels() {
        let mut labeled_memos = LabeledMemos::new();

        // ラベルを作成
        let label1 = LabelId::new();
        let label2 = LabelId::new();

        labeled_memos.labels.push(Label::new("ラベル1"));
        labeled_memos.labels.push(Label::new("ラベル2"));

        // 複数のメモを作成し、異なるラベルを付与
        let mut memo1 = Memo::new();
        memo1.title = "メモ1".to_string();
        memo1.content = "内容1".to_string();
        memo1.labels = vec![label1];
        let memo1_id = memo1.id;

        let mut memo2 = Memo::new();
        memo2.title = "メモ2".to_string();
        memo2.content = "内容2".to_string();
        memo2.labels = vec![label2];
        let memo2_id = memo2.id;

        let mut memo3 = Memo::new();
        memo3.title = "メモ3".to_string();
        memo3.content = "内容3".to_string();
        memo3.labels = vec![label1, label2];
        let memo3_id = memo3.id;

        // メモをリポジトリとワークスペースに追加
        labeled_memos
            .memos
            .list_memo_ids(&[])
            .iter()
            .for_each(|memo_id| {
                let _ = labeled_memos.remove_memo(*memo_id);
            });
        labeled_memos.memos.upsert(memo1);
        labeled_memos.memos.upsert(memo2);
        labeled_memos.memos.upsert(memo3);

        labeled_memos.current_workspace_mut().memos_order.clear(); // デフォルトのメモをクリア
        labeled_memos
            .current_workspace_mut()
            .memos_order
            .push(memo1_id);
        labeled_memos
            .current_workspace_mut()
            .memos_order
            .push(memo2_id);
        labeled_memos
            .current_workspace_mut()
            .memos_order
            .push(memo3_id);

        // label1でフィルタリング
        let filtered_memos = labeled_memos.filter_memos_by_labels(&[label1]);
        assert_eq!(filtered_memos.len(), 2); // memo1とmemo3が該当
        assert!(filtered_memos.iter().any(|memo| memo.id == memo1_id));
        assert!(!filtered_memos.iter().any(|memo| memo.id == memo2_id)); // label1を持たないmemo2は含まれない
        assert!(filtered_memos.iter().any(|memo| memo.id == memo3_id));

        // label2でフィルタリング
        let filtered_memos = labeled_memos.filter_memos_by_labels(&[label2]);
        assert_eq!(filtered_memos.len(), 2); // memo2とmemo3が該当
        assert!(!filtered_memos.iter().any(|memo| memo.id == memo1_id)); // label2を持たないmemo1は含まれない
        assert!(filtered_memos.iter().any(|memo| memo.id == memo2_id));
        assert!(filtered_memos.iter().any(|memo| memo.id == memo3_id));

        // label1とlabel2でフィルタリング（OR条件）
        let filtered_memos = labeled_memos.filter_memos_by_labels(&[label1, label2]);
        assert_eq!(filtered_memos.len(), 3); // 全てのメモが該当

        // 空のラベル配列でフィルタリング
        let filtered_memos = labeled_memos.filter_memos_by_labels(&[]);
        assert_eq!(filtered_memos.len(), 3); // 全てのメモが該当
    }

    #[test]
    fn test_remove_memo_from_workspace() {
        let mut labeled_memos = LabeledMemos::new();

        // 新しいメモを作成
        let title = "削除対象メモ".to_string();
        let content = "これは削除されるメモの内容です。".to_string();
        let memo_id = labeled_memos.add_new_memo(title, content);

        // 追加されたことを確認
        assert!(
            labeled_memos
                .current_workspace()
                .memos_order
                .contains(&memo_id)
        );

        // メモをワークスペースから削除
        let result = labeled_memos.remove_memo_from_workspace(memo_id);
        assert!(result.is_ok());

        // ワークスペースから削除されたことを確認
        assert!(
            !labeled_memos
                .current_workspace()
                .memos_order
                .contains(&memo_id)
        );

        // メモ自体はまだリポジトリに存在していることを確認
        assert!(labeled_memos.memos.contain(memo_id));

        // 存在しないメモを削除しようとするとエラーになることを確認
        let result = labeled_memos.remove_memo_from_workspace(memo_id);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            LabeledMemosError::MemoIdNotFound(memo_id)
        );
    }

    #[test]
    fn test_sort_memos_by_time() {
        use std::thread::sleep;
        use std::time::Duration;

        let mut labeled_memos = LabeledMemos::new();

        // 現在のワークスペースのメモを全て削除
        labeled_memos.current_workspace_mut().memos_order.clear();
        labeled_memos.memos.clear();

        // 異なる時間に3つのメモを作成
        let mut memo1 = Memo::new();
        memo1.title = "一番古いメモ".to_string();
        let memo1_id = memo1.id;
        labeled_memos.memos.upsert(memo1);
        labeled_memos
            .current_workspace_mut()
            .memos_order
            .push(memo1_id);

        // 少し待機して時間差を作る
        sleep(Duration::from_millis(100));

        let mut memo2 = Memo::new();
        memo2.title = "2番目に古いメモ".to_string();
        memo2.created_at += TimeDelta::seconds(1);
        memo2.updated_at += TimeDelta::seconds(1);
        let memo2_id = memo2.id;
        labeled_memos.memos.upsert(memo2);
        labeled_memos
            .current_workspace_mut()
            .memos_order
            .push(memo2_id);

        sleep(Duration::from_millis(100));

        let mut memo3 = Memo::new();
        memo3.title = "最新のメモ".to_string();
        memo3.created_at += TimeDelta::seconds(2);
        memo3.updated_at += TimeDelta::seconds(2);
        let memo3_id = memo3.id;
        labeled_memos.memos.upsert(memo3);
        labeled_memos
            .current_workspace_mut()
            .memos_order
            .push(memo3_id);

        // 意図的に順序を変更
        labeled_memos.current_workspace_mut().memos_order = vec![memo2_id, memo1_id, memo3_id];

        // 作成日時でソート
        labeled_memos.sort_memos_by_time(OrderBy::CreatedDesc);
        {
            // 新しい順にソートされていることを確認
            let workspace = labeled_memos.current_workspace();
            assert_eq!(workspace.memos_order[0], memo3_id);
            assert_eq!(workspace.memos_order[1], memo2_id);
            assert_eq!(workspace.memos_order[2], memo1_id);
        }
        labeled_memos.sort_memos_by_time(OrderBy::CreatedAsc);
        {
            // 新しい順にソートされていることを確認
            let workspace = labeled_memos.current_workspace();
            assert_eq!(workspace.memos_order[0], memo1_id);
            assert_eq!(workspace.memos_order[1], memo2_id);
            assert_eq!(workspace.memos_order[2], memo3_id);
        }

        // 順序を再度変更
        labeled_memos.current_workspace_mut().memos_order = vec![memo2_id, memo1_id, memo3_id];

        // memo1の更新日時を最新にする
        let mut memo1 = labeled_memos.memos.get(memo1_id).unwrap().clone();
        memo1.updated_at += TimeDelta::seconds(10);
        labeled_memos.memos.upsert(memo1);

        // 更新日時でソート
        labeled_memos.sort_memos_by_time(OrderBy::UpdatedDesc);
        {
            // 更新日時の新しい順にソートされていることを確認
            let workspace = labeled_memos.current_workspace();
            assert_eq!(workspace.memos_order[0], memo1_id);
            assert_eq!(workspace.memos_order[1], memo3_id);
            assert_eq!(workspace.memos_order[2], memo2_id);
        }
        labeled_memos.sort_memos_by_time(OrderBy::UpdatedAsc);
        {
            // 更新日時の新しい順にソートされていることを確認
            let workspace = labeled_memos.current_workspace();
            assert_eq!(workspace.memos_order[0], memo2_id);
            assert_eq!(workspace.memos_order[1], memo3_id);
            assert_eq!(workspace.memos_order[2], memo1_id);
        }
    }

    #[test]
    fn test_reorder_workspace_memos() {
        let mut labeled_memos = LabeledMemos::new();

        // 現在のワークスペースのメモを全て削除して新しいメモを3つ作成
        labeled_memos.current_workspace_mut().memos_order.clear();
        labeled_memos.memos.clear();

        // 3つのメモを作成
        let memo1 = labeled_memos.add_new_memo("メモ1".to_string(), "内容1".to_string());
        let memo2 = labeled_memos.add_new_memo("メモ2".to_string(), "内容2".to_string());
        let memo3 = labeled_memos.add_new_memo("メモ3".to_string(), "内容3".to_string());

        // 現在の並び順を確認
        let current_order = labeled_memos.current_workspace().memos_order.clone();
        assert_eq!(current_order, vec![memo1, memo2, memo3]);

        // 並び順を変更
        let new_order = vec![memo3, memo1, memo2];
        let result = labeled_memos.reorder_workspace_memos(new_order.clone());
        assert!(result.is_ok());

        // 並び順が変更されていることを確認
        let updated_order = labeled_memos.current_workspace().memos_order.clone();
        assert_eq!(updated_order, new_order);

        // 存在しないメモIDを含む場合はエラーになることを確認
        let non_existent_id = MemoId::new();
        let invalid_order = vec![memo3, non_existent_id, memo2];
        let result = labeled_memos.reorder_workspace_memos(invalid_order);
        assert!(result.is_err());

        // メモの数が一致しない場合はエラーになることを確認
        let incomplete_order = vec![memo3, memo1];
        let result = labeled_memos.reorder_workspace_memos(incomplete_order);
        assert!(result.is_err());
    }

    #[test]
    fn test_clear_workspace() {
        let mut labeled_memos = LabeledMemos::new();
        labeled_memos.clear_workspace();
        labeled_memos.memos.clear();
        assert_eq!(labeled_memos.memos.list_memo_ids(&[]).len(), 0);

        // 現在のワークスペースに複数のメモを追加
        labeled_memos.add_new_memo("メモ1".to_string(), "内容1".to_string());
        labeled_memos.add_new_memo("メモ2".to_string(), "内容2".to_string());
        labeled_memos.add_new_memo("メモ3".to_string(), "内容3".to_string());
        assert_eq!(labeled_memos.memos.list_memo_ids(&[]).len(), 3);

        // メモが追加されていることを確認
        assert!(!labeled_memos.current_workspace().memos_order.is_empty());
        assert_eq!(labeled_memos.current_workspace().memos_order.len(), 3);

        // ワークスペースのメモを全て削除
        labeled_memos.clear_workspace();

        // ワークスペースが空になっていることを確認
        assert!(labeled_memos.current_workspace().memos_order.is_empty());

        // メモ自体はリポジトリにまだ存在していることを確認
        assert_eq!(labeled_memos.memos.list_memo_ids(&[]).len(), 3);
    }

    #[test]
    fn test_replace_workspace_memos_by_labels() {
        let mut labeled_memos = LabeledMemos::new();

        // ラベルを作成
        let label1 = LabelId::new();
        let label2 = LabelId::new();

        labeled_memos.labels.push(Label::new("ラベル1"));
        labeled_memos.labels.push(Label::new("ラベル2"));

        // メモを全てクリアして新しいメモを作成
        labeled_memos.memos.clear();

        // 複数のメモを作成し、異なるラベルを付与
        let mut memo1 = Memo::new();
        memo1.title = "メモ1".to_string();
        memo1.content = "内容1".to_string();
        memo1.labels = vec![label1];
        let memo1_id = memo1.id;

        let mut memo2 = Memo::new();
        memo2.title = "メモ2".to_string();
        memo2.content = "内容2".to_string();
        memo2.labels = vec![label2];
        let memo2_id = memo2.id;

        let mut memo3 = Memo::new();
        memo3.title = "メモ3".to_string();
        memo3.content = "内容3".to_string();
        memo3.labels = vec![label1, label2];
        let memo3_id = memo3.id;

        // メモをリポジトリに追加
        labeled_memos.memos.upsert(memo1);
        labeled_memos.memos.upsert(memo2);
        labeled_memos.memos.upsert(memo3);

        // 現在のワークスペースのメモを全て削除して別のメモを追加
        labeled_memos.current_workspace_mut().memos_order.clear();
        labeled_memos
            .current_workspace_mut()
            .memos_order
            .push(memo2_id);

        // ワークスペース内のメモがmemo2のみであることを確認
        assert_eq!(labeled_memos.current_workspace().memos_order.len(), 1);
        assert!(
            labeled_memos
                .current_workspace()
                .memos_order
                .contains(&memo2_id)
        );

        // label1でフィルタリングして入れ替え
        labeled_memos.replace_workspace_memos_by_labels(&[label1]);

        // ワークスペースにlabel1を持つメモ（memo1とmemo3）のみが含まれていることを確認
        assert_eq!(labeled_memos.current_workspace().memos_order.len(), 2);
        assert!(
            labeled_memos
                .current_workspace()
                .memos_order
                .contains(&memo1_id)
        );
        assert!(
            !labeled_memos
                .current_workspace()
                .memos_order
                .contains(&memo2_id)
        );
        assert!(
            labeled_memos
                .current_workspace()
                .memos_order
                .contains(&memo3_id)
        );

        // 空のラベル配列でフィルタリングして入れ替え
        labeled_memos.replace_workspace_memos_by_labels(&[]);

        // 全てのメモがワークスペースに含まれていることを確認
        assert_eq!(labeled_memos.current_workspace().memos_order.len(), 3);
    }

    #[test]
    fn test_list_labels() {
        let mut labeled_memos = LabeledMemos::new();
        
        // 初期状態では未分類ラベルのみ
        let initial_labels = labeled_memos.list_labels();
        assert_eq!(initial_labels.len(), 1);
        assert_eq!(initial_labels[0].id, UNLABELED_ID);
        assert_eq!(initial_labels[0].name, "未分類");

        // 新しいラベルを追加
        labeled_memos.labels.push(Label::new("ラベル1"));
        labeled_memos.labels.push(Label::new("ラベル2"));

        // リストが正しく返されること
        let labels = labeled_memos.list_labels();
        assert_eq!(labels.len(), 3); // 未分類 + 追加した2つ
        assert_eq!(labels[0].name, "未分類");
        assert_eq!(labels[1].name, "ラベル1");
        assert_eq!(labels[2].name, "ラベル2");
    }

    #[test]
    fn test_upsert_labels() {
        let mut labeled_memos = LabeledMemos::new();

        // 新しいラベルの追加
        let new_label = Label::new("新しいラベル");
        let label_id = new_label.id;
        
        labeled_memos.upsert_labels(new_label);
        
        // ラベルが追加されたか確認
        assert_eq!(labeled_memos.labels.len(), 2); // 未分類 + 追加した1つ
        assert_eq!(labeled_memos.labels[1].name, "新しいラベル");
        
        // 同じIDで名前を更新
        let updated_label = Label {
            id: label_id,
            name: "更新されたラベル".to_string(),
        };
        
        labeled_memos.upsert_labels(updated_label);
        
        // ラベル数は変わらず名前だけ更新されていることを確認
        assert_eq!(labeled_memos.labels.len(), 2);
        assert_eq!(labeled_memos.labels[1].name, "更新されたラベル");
        
        // IDが同じラベルを確認
        let found_label = labeled_memos.labels.iter().find(|l| l.id == label_id);
        assert!(found_label.is_some());
        assert_eq!(found_label.unwrap().name, "更新されたラベル");
    }

    #[test]
    fn test_remove_label() {
        let mut labeled_memos = LabeledMemos::new();
        
        // 新しいラベルを追加
        let label1 = Label::new("ラベル1");
        let label1_id = label1.id;
        
        let label2 = Label::new("ラベル2");
        let label2_id = label2.id;
        
        labeled_memos.upsert_labels(label1);
        labeled_memos.upsert_labels(label2);
        
        // 追加されたか確認
        assert_eq!(labeled_memos.labels.len(), 3); // 未分類 + 追加した2つ
        
        // ラベル1を削除
        labeled_memos.remove_label(label1_id);
        
        // 削除されたか確認
        assert_eq!(labeled_memos.labels.len(), 2);
        assert!(labeled_memos.labels.iter().all(|l| l.id != label1_id));
        assert!(labeled_memos.labels.iter().any(|l| l.id == label2_id));
        assert!(labeled_memos.labels.iter().any(|l| l.id == UNLABELED_ID));
        
        // 存在しないラベルIDを削除しても何も起こらない
        let non_existent_id = LabelId::new();
        let initial_len = labeled_memos.labels.len();
        
        labeled_memos.remove_label(non_existent_id);
        
        assert_eq!(labeled_memos.labels.len(), initial_len);
        
        // 未分類ラベルの削除
        labeled_memos.remove_label(UNLABELED_ID);
        
        // 未分類ラベルも削除可能
        assert_eq!(labeled_memos.labels.len(), 1);
        assert!(labeled_memos.labels.iter().all(|l| l.id != UNLABELED_ID));
    }
}
