use std::sync::mpsc::Sender;

use chrono::Days;
use font_rasterizer::ui::selectbox::{SelectBox, SelectOption};
use stroke_parser::Action;

use crate::categorized_memos::CategorizedMemos;

pub(crate) fn command_palette_select(action_queue_sender: Sender<Action>) -> SelectBox {
    let options = vec![
        SelectOption::new(
            "炊紙を終了する".to_string(),
            Action::new_command("system", "exit"),
        ),
        SelectOption::new(
            "フルスクリーンの切り替え".to_string(),
            Action::new_command("system", "toggle-fullscreen"),
        ),
        SelectOption::new(
            "カラーテーマの変更".to_string(),
            Action::new_command("system", "select-theme"),
        ),
        SelectOption::new(
            "日付の挿入".to_string(),
            Action::new_command("kashikishi", "insert-date"),
        ),
        SelectOption::new(
            "カテゴリを移動".to_string(),
            Action::new_command("kashikishi", "select-category"),
        ),
        SelectOption::new(
            "編集中のメモの移動".to_string(),
            Action::new_command("kashikishi", "select-move-memo-category"),
        ),
    ];
    SelectBox::new(action_queue_sender, "アクションの選択".to_string(), options)
}

pub(crate) fn insert_date_select(action_queue_sender: Sender<Action>) -> SelectBox {
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
    SelectBox::new(
        action_queue_sender,
        "挿入したい日付を選択".to_string(),
        options,
    )
}

pub(crate) fn change_theme_select(action_queue_sender: Sender<Action>) -> SelectBox {
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
    SelectBox::new(
        action_queue_sender,
        "カラーテーマを選択して下さい".to_string(),
        options,
    )
}

pub(crate) fn select_move_memo_category(
    categorized_memos: &CategorizedMemos,
    action_queue_sender: Sender<Action>,
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
                "move-memo",
                &category,
            )],
        ));
    }
    if !has_archive {
        options.push(SelectOption::new_multiple(
            "archive".to_string(),
            vec![Action::new_command_with_argument(
                "kashikishi",
                "move-memo",
                "archive",
            )],
        ));
    }

    SelectBox::new(
        action_queue_sender,
        "メモの移動先カテゴリーを選択".to_string(),
        options,
    )
}

pub(crate) fn change_memos_category(
    categorized_memos: &CategorizedMemos,
    action_queue_sender: Sender<Action>,
) -> SelectBox {
    let mut options = Vec::new();
    for category in categorized_memos.categories() {
        options.push(SelectOption::new(
            category.clone(),
            Action::new_command_with_argument("kashikishi", "change-memos-category", &category),
        ));
    }
    SelectBox::new(
        action_queue_sender,
        "移動先のカテゴリーを選択".to_string(),
        options,
    )
}
