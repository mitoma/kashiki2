use chrono::Days;
use stroke_parser::Action;
use ui_support::ui::{FileChooser, SelectBox, SelectOption, TextInput};
use ui_support::ui_context::UiContext;

use crate::{
    action_repository::{ActionNamespace, ActionRepository},
    categorized_memos::CategorizedMemos,
};

pub(crate) fn command_palette_select(context: &UiContext, narrow: Option<String>) -> SelectBox {
    let mut options = Vec::new();

    let action_repository = ActionRepository::default();

    for namespace in [
        ActionNamespace::Mode,
        ActionNamespace::System,
        ActionNamespace::Edit,
        ActionNamespace::World,
        ActionNamespace::ActionRecorder,
        ActionNamespace::Kashikishi,
    ] {
        for action_definition in action_repository.load_actions(namespace) {
            options.push(SelectOption::new(
                action_definition.description.clone(),
                action_definition.to_action(),
            ));
        }
    }
    SelectBox::new(context, "アクションの選択".to_string(), options, narrow)
}

pub(crate) fn insert_date_select(context: &UiContext) -> SelectBox {
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
    SelectBox::new(context, "挿入したい日付を選択".to_string(), options, None)
}

pub(crate) fn move_category_ui(
    context: &UiContext,
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

    SelectBox::new_without_action_name(
        context,
        "メモの移動先カテゴリーを選択".to_string(),
        options,
        None,
    )
}

pub(crate) fn move_memo_ui(context: &UiContext, categorized_memos: &CategorizedMemos) -> SelectBox {
    let mut options = Vec::new();
    for category in categorized_memos.categories() {
        options.push(SelectOption::new(
            category.clone(),
            Action::new_command_with_argument("kashikishi", "move-memo", &category),
        ));
    }
    SelectBox::new_without_action_name(
        context,
        "移動先のカテゴリーを選択".to_string(),
        options,
        None,
    )
}

pub(crate) fn add_category_ui(context: &UiContext) -> TextInput {
    TextInput::new(
        context,
        "追加するカテゴリーを選択".to_string(),
        None,
        Action::new_command("kashikishi", "add-category"),
    )
}

pub(crate) fn rename_category_select_ui(
    context: &UiContext,
    categorized_memos: &CategorizedMemos,
) -> SelectBox {
    let mut options = Vec::new();
    for category in categorized_memos.categories() {
        options.push(SelectOption::new(
            category.clone(),
            Action::new_command_with_argument("kashikishi", "rename-category-ui", &category),
        ));
    }
    SelectBox::new_without_action_name(
        context,
        "名前を変更するカテゴリーを選択".to_string(),
        options,
        None,
    )
}

pub(crate) fn rename_category_ui(context: &UiContext, category_name: &str) -> TextInput {
    TextInput::new(
        context,
        "変更後の名前を入力".to_string(),
        Some(category_name.to_string()),
        Action::new_command("kashikishi", "rename-category"),
    )
}

pub(crate) fn remove_category_ui(
    context: &UiContext,
    categorized_memos: &CategorizedMemos,
) -> SelectBox {
    let mut options = Vec::new();
    for category in categorized_memos.categories() {
        options.push(SelectOption::new(
            category.clone(),
            Action::new_command_with_argument("kashikishi", "remove-category", &category),
        ));
    }
    SelectBox::new_without_action_name(
        context,
        "削除するカテゴリーを選択(中の文書はdefualtに移動します)".to_string(),
        options,
        None,
    )
}

pub(crate) fn open_file_ui(context: &UiContext) -> FileChooser {
    FileChooser::new(
        context,
        &std::env::current_dir().unwrap(),
        "ファイルを開く",
        Action::new_command("kashikishi", "open-file"),
    )
}
