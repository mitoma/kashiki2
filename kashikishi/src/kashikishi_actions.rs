use std::sync::mpsc::Sender;

use chrono::Days;
use font_rasterizer::ui::selectbox::{SelectBox, SelectOption};
use stroke_parser::Action;

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
