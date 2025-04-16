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
