use std::{io::BufReader, path::Path, sync::mpsc::Sender, thread};

use font_rasterizer::time::now_millis;
use serde_jsonlines::{append_json_lines, json_lines, BufReadExt};
use stroke_parser::{Action, ActionArgument};

const SCRIPT_NAME: &str = "record.jsonl";

pub struct ActionRecorder {
    action_receiver: Sender<Action>,
    pre_time: u32,
}

impl ActionRecorder {
    pub fn new(action_receiver: Sender<Action>) -> Self {
        Self {
            action_receiver,
            pre_time: now_millis(),
        }
    }

    pub fn record(&mut self, action: &Action) {
        // IME の ON/OFF は記録しない
        if let Action::ImeDisable | Action::ImeEnable = action {
            return;
        }
        let now = now_millis();
        let duration = now - self.pre_time;
        self.pre_time = now;
        let wait = Action::Command(
            "action_recorder".into(),
            "wait".into(),
            stroke_parser::ActionArgument::Integer(duration as i32),
        );
        let actions = vec![&wait, action];
        append_json_lines(Path::new(SCRIPT_NAME), actions).unwrap();
    }

    pub fn replay(&mut self) -> anyhow::Result<()> {
        for action in json_lines::<Action, _>(Path::new(SCRIPT_NAME))? {
            self.internal_replay(action?)?;
        }
        Ok(())
    }

    pub fn replay_from_jsonl(&mut self, jsonl: String) -> anyhow::Result<()> {
        for action in BufReader::new(jsonl.as_bytes()).json_lines::<Action>() {
            self.internal_replay(action?)?;
        }
        Ok(())
    }

    fn internal_replay(&mut self, action: Action) -> anyhow::Result<()> {
        match action {
            // action_recorder::wait は特別な action なので送信せず sleep する
            Action::Command(command_namespace, command_name, ActionArgument::Integer(time))
                if command_namespace == "action_recorder".into()
                    && command_name == "wait".into() =>
            {
                sleep(time as u64);
            }
            other => self.action_receiver.send(other)?,
        }
        Ok(())
    }
}

//#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn sleep(ms: u64) {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            use wasm_bindgen::prelude::*;
            use web_sys;
            let promise = js_sys::Promise::new(&mut |resolve, _| {
                web_sys::window()
                    .unwrap()
                    .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
                    .unwrap();
            })
            promise.await;
        } else {
            thread::sleep(std::time::Duration::from_millis(ms as u64));
        }
    }
}
