use std::{collections::VecDeque, io::BufReader, path::Path, thread};

use font_rasterizer::{context::StateContext, time::now_millis};
use serde_jsonlines::{write_json_lines, BufReadExt};
use stroke_parser::{Action, ActionArgument};

const SCRIPT_NAME: &str = "record.jsonl";

pub trait ActionRecordRepository {
    fn save(&mut self, action: &Vec<Action>);
    fn load(&self) -> Vec<Action>;
}

#[derive(Debug, Default)]
pub struct FileActionRecordRepository;

impl ActionRecordRepository for FileActionRecordRepository {
    fn save(&mut self, action: &Vec<Action>) {
        let path = Path::new(SCRIPT_NAME);
        write_json_lines(path, action).unwrap();
    }

    fn load(&self) -> Vec<Action> {
        BufReader::new(std::fs::File::open(SCRIPT_NAME).unwrap())
            .json_lines::<Action>()
            .flat_map(|action| action)
            .collect()
    }
}

#[derive(Debug, Default)]
pub struct InMemoryActionRecordRepository {
    records: Vec<Action>,
}

impl ActionRecordRepository for InMemoryActionRecordRepository {
    fn save(&mut self, action: &Vec<Action>) {
        self.records.extend(action.iter().cloned());
    }

    fn load(&self) -> Vec<Action> {
        self.records.clone()
    }
}

pub struct ActionRecorder {
    mode: RecorderMode,
    replay_mode: ReplayMode,
    repository: Box<dyn ActionRecordRepository>,
    record_data: Vec<Action>,
    replay_queue: VecDeque<Action>,
    pre_time: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum RecorderMode {
    None,
    Record,
    Replay,
}

pub enum ReplayMode {
    Normal,
    Fast,
}

impl ActionRecorder {
    pub fn new(repository: Box<dyn ActionRecordRepository>) -> Self {
        Self {
            mode: RecorderMode::None,
            replay_mode: ReplayMode::Normal,
            repository,
            record_data: Vec::new(),
            replay_queue: VecDeque::new(),
            pre_time: now_millis(),
        }
    }

    pub fn record(&mut self, action: &Action) {
        if self.mode != RecorderMode::Record {
            return;
        }
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
        self.record_data.push(wait);
        self.record_data.push(action.clone());
    }

    pub fn replay(&mut self, context: &StateContext) {
        if self.mode != RecorderMode::Replay {
            return;
        }
        if let Some(action) = self.replay_queue.pop_front() {
            match action {
                // action_recorder::wait は特別な action なので送信しないで replay のタイミングを調整する
                Action::Command(command_namespace, command_name, ActionArgument::Integer(time))
                    if command_namespace == "action_recorder".into()
                        && command_name == "wait".into() =>
                {
                    let now = now_millis();
                    let duration = now - self.pre_time;
                    self.pre_time = now;

                    let duration = match self.replay_mode {
                        ReplayMode::Normal => duration,
                        ReplayMode::Fast => duration / 10,
                    };
                    if duration < time as u32 {
                        self.replay_queue.push_front(Action::Command(
                            command_namespace,
                            command_name,
                            ActionArgument::Integer(time - duration as i32),
                        ));
                    }
                }
                other => context.action_queue_sender.send(other).unwrap(),
            }
        } else {
            self.mode = RecorderMode::None;
        }
    }

    pub fn start_record(&mut self) {
        self.mode = RecorderMode::Record;
        self.record_data.clear();
        self.pre_time = now_millis();
    }

    pub fn stop_record(&mut self) {
        self.mode = RecorderMode::None;
        self.repository.save(&self.record_data);
        self.record_data.clear();
    }

    pub fn start_replay(&mut self) {
        self.mode = RecorderMode::Replay;
        self.replay_queue.clear();
        self.replay_queue.extend(self.repository.load());
        self.pre_time = now_millis();
    }

    pub fn stop_replay(&mut self) {
        self.mode = RecorderMode::None;
        self.replay_queue.clear();
    }

    pub fn set_replay_mode(&mut self, replay_mode: ReplayMode) {
        self.replay_mode = replay_mode;
    }
}
