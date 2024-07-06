use stroke_parser::{Action, ActionArgument};

pub struct SelectOption {
    pub(crate) text: String,
    pub(crate) actions: Vec<Action>,
}

impl SelectOption {
    pub fn new(text: String, action: Action) -> Self {
        Self {
            text,
            actions: vec![action],
        }
    }

    pub fn new_multiple(text: String, actions: Vec<Action>) -> Self {
        Self { text, actions }
    }

    pub fn option_string(&self) -> String {
        if self.actions.len() == 1 {
            if let Action::Command(namespace, name, arg) = &self.actions[0] {
                match arg {
                    ActionArgument::String(_)
                    | ActionArgument::Integer(_)
                    | ActionArgument::Float(_)
                    | ActionArgument::Point(_) => {
                        return format!("{} ({}:{}({}))", self.text, **namespace, **name, arg)
                    }
                    ActionArgument::None => {
                        return format!("{} ({}:{})", self.text, **namespace, **name)
                    }
                }
            }
        }
        self.text.to_string()
    }

    pub fn contains_all(&self, keywords: &[&str]) -> bool {
        // 大文字小文字、無視してキーワードが含まれているか
        let text = self.option_string().to_lowercase();
        keywords
            .iter()
            .map(|keyword| keyword.to_lowercase())
            .all(|keyword| text.contains(&keyword))
    }
}
