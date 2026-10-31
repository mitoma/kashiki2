use std::{
    collections::VecDeque,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    sync::LazyLock,
};

use font_rasterizer::time::now_millis;
use serde_jsonlines::write_json_lines;
use stroke_parser::{Action, ActionArgument, CommandName};
use thiserror::Error;

use crate::{InputResult, action::NamespaceActionProcessors, ui_context::UiContext};

const DEFAULT_SCRIPT_NAME: &str = "record.jsonl";

/// 記録・再生用の特殊コマンドの名前空間。
/// 既存の記録ファイルとの互換のため、NamespaceActionProcessors の `action-recorder` とは表記が異なる。
pub const RECORDER_NAMESPACE: &str = "action_recorder";
/// 再生時に指定ミリ秒だけ待つ特殊コマンド
pub const WAIT_COMMAND: &str = "wait";
/// 再生時にスナップショットを要求する特殊コマンド
pub const SNAPSHOT_COMMAND: &str = "snapshot";

pub fn wait_action(millis: u32) -> Action {
    Action::Command(
        RECORDER_NAMESPACE.into(),
        WAIT_COMMAND.into(),
        ActionArgument::Integer(millis as i32),
    )
}

pub fn snapshot_action(name: &str) -> Action {
    Action::Command(
        RECORDER_NAMESPACE.into(),
        SNAPSHOT_COMMAND.into(),
        ActionArgument::String(name.to_string()),
    )
}

/// スナップショット要求の Action であれば、そのスナップショット名を返す
pub fn as_snapshot_name(action: &Action) -> Option<&str> {
    match action {
        Action::Command(namespace, name, ActionArgument::String(snapshot_name))
            if namespace.as_str() == RECORDER_NAMESPACE && name.as_str() == SNAPSHOT_COMMAND =>
        {
            Some(snapshot_name.as_str())
        }
        _ => None,
    }
}

fn as_wait_millis(action: &Action) -> Option<i32> {
    match action {
        Action::Command(namespace, name, ActionArgument::Integer(millis))
            if namespace.as_str() == RECORDER_NAMESPACE && name.as_str() == WAIT_COMMAND =>
        {
            Some(*millis)
        }
        _ => None,
    }
}

#[derive(Debug, Error)]
pub enum ActionRecordError {
    #[error("failed to read action record {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse action record {path} line {line}: {source}")]
    Parse {
        path: PathBuf,
        line: usize,
        source: serde_json::Error,
    },
}

/// JSON Lines 形式の記録ファイルを読み込む。空行は無視する。
pub fn load_action_records(path: &Path) -> Result<Vec<Action>, ActionRecordError> {
    let file = std::fs::File::open(path).map_err(|source| ActionRecordError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut actions = Vec::new();
    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|source| ActionRecordError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        if line.trim().is_empty() {
            continue;
        }
        let action = serde_json::from_str(&line).map_err(|source| ActionRecordError::Parse {
            path: path.to_path_buf(),
            line: index + 1,
            source,
        })?;
        actions.push(action);
    }
    Ok(actions)
}

pub trait ActionRecordRepository {
    fn save(&mut self, action: &[Action]);
    fn load(&self) -> Vec<Action>;
}

#[derive(Debug)]
pub struct FileActionRecordRepository {
    path: PathBuf,
}

impl FileActionRecordRepository {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl Default for FileActionRecordRepository {
    fn default() -> Self {
        Self::new(DEFAULT_SCRIPT_NAME)
    }
}

impl ActionRecordRepository for FileActionRecordRepository {
    fn save(&mut self, action: &[Action]) {
        if let Err(err) = write_json_lines(&self.path, action) {
            log::error!(
                "failed to save action record {}: {}",
                self.path.display(),
                err
            );
        }
    }

    fn load(&self) -> Vec<Action> {
        load_action_records(&self.path).unwrap_or_else(|err| {
            log::error!("{}", err);
            Vec::new()
        })
    }
}

#[derive(Debug, Default)]
pub struct InMemoryActionRecordRepository {
    records: Vec<Action>,
}

impl InMemoryActionRecordRepository {
    pub fn new(records: Vec<Action>) -> Self {
        Self { records }
    }
}

impl ActionRecordRepository for InMemoryActionRecordRepository {
    fn save(&mut self, action: &[Action]) {
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

    pub fn new_with_time(repository: Box<dyn ActionRecordRepository>, time: u32) -> Self {
        Self {
            mode: RecorderMode::None,
            replay_mode: ReplayMode::Normal,
            repository,
            record_data: Vec::new(),
            replay_queue: VecDeque::new(),
            pre_time: time,
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
        let duration = now.saturating_sub(self.pre_time);
        self.pre_time = now;
        self.record_data.push(wait_action(duration));
        self.record_data.push(action.clone());
    }

    pub fn replay(&mut self, context: &UiContext) {
        if self.mode != RecorderMode::Replay {
            return;
        }
        loop {
            let Some(action) = self.replay_queue.pop_front() else {
                self.mode = RecorderMode::None;
                break;
            };
            // wait は特別な action なので送信しないで replay のタイミングを調整する
            if let Some(time) = as_wait_millis(&action) {
                let now = now_millis();
                log::debug!("now: {}, pre: {}", now, self.pre_time);
                let duration = now.saturating_sub(self.pre_time);
                self.pre_time = now;

                let duration = match self.replay_mode {
                    ReplayMode::Normal => duration,
                    ReplayMode::Fast => duration / 10,
                };
                if duration < time as u32 {
                    self.replay_queue
                        .push_front(wait_action(time as u32 - duration));
                    break;
                }
                continue;
            }
            let is_snapshot = as_snapshot_name(&action).is_some();
            context.register_action(action);
            // スナップショットの後続 action が同じフレームで処理されると、
            // スナップショットに後続 action の結果が混ざるので、このフレームの再生はここで区切る
            if is_snapshot {
                break;
            }
        }
    }

    pub fn is_replaying(&self) -> bool {
        self.mode == RecorderMode::Replay
    }

    pub fn start_record(&mut self) {
        self.mode = RecorderMode::Record;
        self.record_data.clear();
        self.pre_time = now_millis();
    }

    pub fn stop_record(&mut self) {
        // 記録中以外(再生中など)に呼ばれた場合に、空の記録で上書きしないようにする
        if self.mode != RecorderMode::Record {
            return;
        }
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

static NAMES: LazyLock<Vec<CommandName>> = LazyLock::new(|| {
    vec![
        "start-record".into(),
        "stop-record".into(),
        "start-replay".into(),
        "stop-replay".into(),
        "set-replay-mode".into(),
    ]
});

impl NamespaceActionProcessors for ActionRecorder {
    fn namespace(&self) -> stroke_parser::CommandNamespace {
        "action-recorder".into()
    }

    fn names(&self) -> &[CommandName] {
        NAMES.as_slice()
    }

    fn process(
        &mut self,
        command_name: &CommandName,
        arg: &ActionArgument,
        _context: &UiContext,
        _world: &mut dyn crate::layout_engine::World,
    ) -> InputResult {
        match command_name.as_str() {
            "start-record" => {
                self.start_record();
                InputResult::InputConsumed
            }
            "stop-record" => {
                self.stop_record();
                InputResult::InputConsumed
            }
            "start-replay" => {
                self.start_replay();
                InputResult::InputConsumed
            }
            "stop-replay" => {
                self.stop_replay();
                InputResult::InputConsumed
            }
            "set-replay-mode" => {
                if let ActionArgument::String(replay_mode) = arg {
                    match replay_mode.as_str() {
                        "normal" => self.set_replay_mode(ReplayMode::Normal),
                        "fast" => self.set_replay_mode(ReplayMode::Fast),
                        _ => {}
                    }
                }
                InputResult::InputConsumed
            }
            _ => InputResult::Noop,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str, content: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("ui_support_action_recorder_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn load_action_records_skips_blank_lines() {
        let path = temp_file(
            "ok.jsonl",
            "{\"Command\":[\"action_recorder\",\"wait\",{\"Integer\":100}]}\n\n{\"Keytype\":\"a\"}\n",
        );
        let actions = load_action_records(&path).unwrap();
        assert_eq!(actions, vec![wait_action(100), Action::Keytype('a')]);
    }

    #[test]
    fn load_action_records_reports_line_number() {
        let path = temp_file("ng.jsonl", "{\"Keytype\":\"a\"}\n{broken}\n");
        match load_action_records(&path) {
            Err(ActionRecordError::Parse { line, .. }) => assert_eq!(line, 2),
            other => panic!("unexpected result: {:?}", other),
        }
    }

    #[test]
    fn snapshot_action_roundtrip() {
        let action = snapshot_action("01_start");
        assert_eq!(as_snapshot_name(&action), Some("01_start"));
        assert_eq!(as_snapshot_name(&wait_action(10)), None);
        assert_eq!(
            serde_json::to_string(&action).unwrap(),
            r#"{"Command":["action_recorder","snapshot",{"String":"01_start"}]}"#
        );
    }
}
