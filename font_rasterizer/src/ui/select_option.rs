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

    pub fn option_string_short(&self) -> String {
        self.text.to_string()
    }

    pub fn option_string(&self, padding: usize) -> String {
        if self.actions.len() == 1 {
            if let Action::Command(namespace, name, arg) = &self.actions[0] {
                match arg {
                    ActionArgument::String(_)
                    | ActionArgument::Integer(_)
                    | ActionArgument::Float(_)
                    | ActionArgument::Point(_) => {
                        return format!(
                            "{} {padding}{}:{}({})",
                            self.text,
                            **namespace,
                            **name,
                            arg,
                            padding = " ".repeat(padding)
                        );
                    }
                    ActionArgument::None => {
                        return format!(
                            "{} {padding}{}:{}",
                            self.text,
                            **namespace,
                            **name,
                            padding = " ".repeat(padding)
                        )
                    }
                }
            }
        }
        self.text.to_string()
    }

    pub fn contains_all_for_short(&self, keywords: &[&str]) -> bool {
        Self::inner_contains_all(self.option_string_short(), keywords)
    }

    pub fn contains_all(&self, keywords: &[&str]) -> bool {
        Self::inner_contains_all(self.option_string(0), keywords)
    }

    fn inner_contains_all(text: String, keywords: &[&str]) -> bool {
        // 大文字小文字、無視してキーワードが含まれているか
        let text = text.to_lowercase();
        keywords
            .iter()
            .map(|keyword| keyword.to_lowercase())
            .all(|keyword| text.contains(&keyword))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_contains_all() {
        let option =
            SelectOption::new("test".to_string(), Action::new_command("namespace", "name"));
        assert_eq!(option.contains_all(&["test"]), true);
        assert_eq!(option.contains_all(&["test", "test2"]), false);
        assert_eq!(option.contains_all(&["namespace:name"]), true);
        assert_eq!(option.contains_all(&["name"]), true);
    }

    #[test]
    fn test_contains_all_for_short() {
        let option =
            SelectOption::new("test".to_string(), Action::new_command("namespace", "name"));
        assert_eq!(option.contains_all_for_short(&["test"]), true);
        assert_eq!(option.contains_all_for_short(&["test", "test2"]), false);
        assert_eq!(option.contains_all_for_short(&["namespace:name"]), false);
        assert_eq!(option.contains_all_for_short(&["name"]), false);
    }

}
