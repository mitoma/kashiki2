use font_rasterizer::time::{ClockMode, increment_fixed_clock, set_clock_mode};
use image::{ImageBuffer, Rgba};
use stroke_parser::Action;
use web_time::Duration;

use crate::{
    Flags, InputResult, RenderTargetResponse, SimpleStateSupport,
    action_recorder::as_snapshot_name,
    handle_action_result,
    render_state::{RenderState, RenderTargetRequest},
};

pub struct HeadlessOptions {
    /// 1 フレームで固定時計を進める量
    pub frame_gain: Duration,
    /// このフレーム数を超えても終了しなければタイムアウトとする
    pub max_frames: u32,
}

/// `action_recorder:snapshot` で要求されたスナップショット
pub struct Snapshot {
    pub name: String,
    pub frame: u32,
    pub image: ImageBuffer<Rgba<u8>, Vec<u8>>,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadlessExit {
    /// `system:exit` などで終了した
    Exited { frames: u32 },
    /// `max_frames` を超えても終了しなかった
    Timeout { frames: u32 },
}

#[derive(Default)]
struct FrameRequests {
    snapshots: Vec<String>,
    exit: bool,
}

/// ウィンドウを開かずに固定時計で描画を進める。
/// action キューに流れてきた `action_recorder:snapshot` を横取りし、そのフレームの描画結果を `on_snapshot` に渡す。
pub async fn run_headless<F>(
    support: SimpleStateSupport,
    options: HeadlessOptions,
    mut on_snapshot: F,
) -> HeadlessExit
where
    F: FnMut(Snapshot),
{
    set_clock_mode(ClockMode::Fixed);

    let mut state = RenderState::new(
        RenderTargetRequest::Image {
            window_size: support.window_size,
        },
        support.quarity,
        support.color_theme,
        support.callback,
        support.font_repository,
        support.performance_mode,
        support.flags.contains(Flags::TRANCEPARENT),
        support.background_image,
        support.shader_art,
    )
    .await;
    state.resize(support.window_size);

    let mut result = HeadlessExit::Timeout {
        frames: options.max_frames,
    };
    for frame in 0..options.max_frames {
        let mut requests = FrameRequests::default();
        while let Ok(action) = state.action_queue_receiver.try_recv() {
            process_action(&mut state, action, &mut requests);
        }

        state.update();
        let image = match state.async_render().await {
            Ok(RenderTargetResponse::Image(image)) => image,
            _ => panic!("image is not found"),
        };
        for name in requests.snapshots.drain(..) {
            on_snapshot(Snapshot {
                name,
                frame,
                image: image.clone(),
                text: state.snapshot_text(),
            });
        }
        increment_fixed_clock(options.frame_gain);

        while let Ok(action) = state.post_action_queue_receiver.try_recv() {
            process_action(&mut state, action, &mut requests);
        }
        // post action 中に要求されたスナップショットは次のフレームで撮る
        if !requests.snapshots.is_empty() {
            let pending = std::mem::take(&mut requests.snapshots);
            for name in pending {
                state
                    .context
                    .register_action(crate::action_recorder::snapshot_action(&name));
            }
        }

        if requests.exit {
            result = HeadlessExit::Exited { frames: frame + 1 };
            break;
        }
    }
    state.shutdown();
    result
}

fn process_action(state: &mut RenderState, action: Action, requests: &mut FrameRequests) {
    if let Some(name) = as_snapshot_name(&action) {
        requests.snapshots.push(name.to_string());
        return;
    }
    // 終了要求の後に流れてきた action は処理しない
    if requests.exit {
        return;
    }
    match handle_action_result(state.action(action), state) {
        Some(InputResult::SendExit) => requests.exit = true,
        Some(InputResult::ChangeWindowSize(window_size)) => state.resize(window_size),
        Some(InputResult::ChangeQuarity(quarity)) => state.change_quarity(quarity),
        // フルスクリーンやタイトルバーなどウィンドウに関する要求はヘッドレスでは無視する
        _ => {}
    }
}
