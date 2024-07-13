use std::{path::PathBuf, sync::mpsc::Sender};

use chrono::Days;
use font_rasterizer::{
    context::StateContext,
    ui::{select_option::SelectOption, selectbox::SelectBox, text_input::TextInput},
};
use stroke_parser::Action;

use crate::categorized_memos::CategorizedMemos;

pub(crate) fn command_palette_select(context: &StateContext) -> SelectBox {
    let options = vec![
        SelectOption::new(
            "炊紙を終了する".to_string(),
            Action::new_command("system", "exit"),
        ),
        SelectOption::new(
            "ファイルを開く".to_string(),
            Action::new_command("kashikishi", "open-file-ui"),
        ),
        SelectOption::new(
            "フルスクリーンの切り替え".to_string(),
            Action::new_command("system", "toggle-fullscreen"),
        ),
        SelectOption::new(
            "カラーテーマの変更".to_string(),
            Action::new_command("system", "change-theme-ui"),
        ),
        SelectOption::new(
            "日付の挿入".to_string(),
            Action::new_command("kashikishi", "insert-date"),
        ),
        SelectOption::new(
            "カテゴリを移動".to_string(),
            Action::new_command("kashikishi", "move-category-ui"),
        ),
        SelectOption::new(
            "カテゴリの追加".to_string(),
            Action::new_command("kashikishi", "add-category-ui"),
        ),
        SelectOption::new(
            "カテゴリの削除".to_string(),
            Action::new_command("kashikishi", "remove-category-ui"),
        ),
        SelectOption::new(
            "編集中のメモの移動".to_string(),
            Action::new_command("kashikishi", "move-memo-ui"),
        ),
    ];
    SelectBox::new(context, "アクションの選択".to_string(), options)
}

pub(crate) fn insert_date_select(context: &StateContext) -> SelectBox {
    let now = chrono::Local::now();
    let today_date = now.format("%Y/%m/%d").to_string();
    let today_datetime = now.format("%Y/%m/%d %H:%M:%S").to_string();
    let yesterday_date = now
        .checked_sub_days(Days::new(1))
        .unwrap()
        .format("%Y/%m/%d")
        .to_string();
    let tomorrow_date = now
        .checked_add_days(Days::new(1))
        .unwrap()
        .format("%Y/%m/%d")
        .to_string();

    let options = vec![
        SelectOption::new(
            format!("現在({})", today_datetime),
            Action::ImeInput(today_datetime),
        ),
        SelectOption::new(
            format!("今日({})", today_date),
            Action::ImeInput(today_date),
        ),
        SelectOption::new(
            format!("昨日({})", yesterday_date),
            Action::ImeInput(yesterday_date),
        ),
        SelectOption::new(
            format!("明日({})", tomorrow_date),
            Action::ImeInput(tomorrow_date),
        ),
    ];
    SelectBox::new(context, "挿入したい日付を選択".to_string(), options)
}

pub(crate) fn change_theme_ui(context: &StateContext) -> SelectBox {
    let options = vec![
        SelectOption::new(
            "Solarized Blackback".to_string(),
            Action::new_command_with_argument("system", "change-theme", "black"),
        ),
        SelectOption::new(
            "Solarized Dark".to_string(),
            Action::new_command_with_argument("system", "change-theme", "dark"),
        ),
        SelectOption::new(
            "Solarized Light".to_string(),
            Action::new_command_with_argument("system", "change-theme", "light"),
        ),
    ];
    SelectBox::new(context, "カラーテーマを選択して下さい".to_string(), options)
}

pub(crate) fn move_category_ui(
    context: &StateContext,
    categorized_memos: &CategorizedMemos,
) -> SelectBox {
    let mut options = vec![];

    let mut has_archive = false;
    for category in categorized_memos.categories() {
        if category == "archive" {
            has_archive = true;
        }
        options.push(SelectOption::new_multiple(
            category.clone(),
            vec![Action::new_command_with_argument(
                "kashikishi",
                "move-category",
                &category,
            )],
        ));
    }
    if !has_archive {
        options.push(SelectOption::new_multiple(
            "archive".to_string(),
            vec![Action::new_command_with_argument(
                "kashikishi",
                "move-category",
                "archive",
            )],
        ));
    }

    SelectBox::new(context, "メモの移動先カテゴリーを選択".to_string(), options)
}

pub(crate) fn move_memo_ui(
    context: &StateContext,
    categorized_memos: &CategorizedMemos,
) -> SelectBox {
    let mut options = Vec::new();
    for category in categorized_memos.categories() {
        options.push(SelectOption::new(
            category.clone(),
            Action::new_command_with_argument("kashikishi", "move-memo", &category),
        ));
    }
    SelectBox::new(context, "移動先のカテゴリーを選択".to_string(), options)
}

pub(crate) fn add_category_ui(action_queue_sender: Sender<Action>) -> TextInput {
    TextInput::new(
        action_queue_sender,
        "追加するカテゴリーを選択".to_string(),
        Action::new_command("kashikishi", "add-category"),
    )
}

pub(crate) fn remove_category_ui(
    context: &StateContext,
    categorized_memos: &CategorizedMemos,
) -> SelectBox {
    let mut options = Vec::new();
    for category in categorized_memos.categories() {
        options.push(SelectOption::new(
            category.clone(),
            Action::new_command_with_argument("kashikishi", "remove-category", &category),
        ));
    }
    SelectBox::new(
        context,
        "削除するカテゴリーを選択(中の文書はdefualtに移動します)".to_string(),
        options,
    )
}

pub(crate) fn open_file_ui(context: &StateContext, path: Option<&str>) -> SelectBox {
    let mut options = Vec::new();
    // current directory のファイル一覧を取得
    let current_dir = if let Some(path) = path {
        PathBuf::from(path)
    } else {
        std::env::current_dir().unwrap()
    };

    if let Some(parent_dir) = current_dir.parent() {
        options.push(SelectOption::new(
            "📁 ../".to_string(),
            Action::new_command_with_argument(
                "kashikishi",
                "open-file-ui",
                parent_dir.to_str().unwrap(),
            ),
        ));
    }

    for entry in std::fs::read_dir(current_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            options.push(SelectOption::new(
                format!("📁 {}", path.file_name().unwrap().to_str().unwrap()),
                Action::new_command_with_argument(
                    "kashikishi",
                    "open-file-ui",
                    path.to_str().unwrap(),
                ),
            ));
        } else if path.is_file() {
            options.push(SelectOption::new(
                format!("📄 {}", path.file_name().unwrap().to_str().unwrap()),
                Action::new_command_with_argument(
                    "kashikishi",
                    "open-file",
                    path.to_str().unwrap(),
                ),
            ));
        }
    }
    SelectBox::new_without_action_name(context, "ファイルを開く".to_string(), options)
}
