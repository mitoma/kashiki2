use std::collections::HashSet;

use text_buffer::action::EditorOperation;

use font_rasterizer::{
    context::{StateContext, WindowSize},
    glyph_instances::GlyphInstances,
    vector_instances::VectorInstances,
};

use crate::{
    camera::{Camera, CameraAdjustment, CameraOperation},
    layout_engine::{Model, ModelOperation, WorldLayout},
};

// 画面全体を表す
pub trait World {
    // model を追加する
    fn add(&mut self, model: Box<dyn Model>);
    // model を現在のモデルの次に追加する
    fn add_next(&mut self, model: Box<dyn Model>);
    // モーダルなモデルを追加する
    fn add_modal(&mut self, model: Box<dyn Model>);
    // 現在参照している model を削除する
    fn remove_current(&mut self);

    // 再レイアウトする update するときに呼び出すとよさそう
    fn re_layout(&mut self);

    fn update(&mut self, context: &StateContext);

    // この World にいくつモデルを配置されているかを返す
    fn model_length(&self) -> usize;
    // 何番目のモデルに視点を移すか
    fn look_at(&mut self, model_num: usize, adjustment: CameraAdjustment);

    // 現在のモデルに再度視点を移す
    fn look_current(&mut self, adjustment: CameraAdjustment);
    // 次のモデルに視点を移す
    fn look_next(&mut self, adjustment: CameraAdjustment);
    // 前のモデルに視点を移す
    fn look_prev(&mut self, adjustment: CameraAdjustment);
    // 次のモデルに視点を移す
    fn swap_next(&mut self);
    // 次のモデルに視点を移す
    fn swap_prev(&mut self);
    // カメラの参照を返す
    fn camera(&self) -> &Camera;
    // カメラを動かす
    fn camera_operation(&mut self, camera_operation: CameraOperation);
    // ウィンドウサイズ変更の通知を受け取る
    fn change_window_size(&mut self, window_size: WindowSize);
    // レイアウトを変更する
    fn change_layout(&mut self, layout: WorldLayout);
    // レイアウトを返す
    fn layout(&self) -> &WorldLayout;
    // glyph_instances を返す
    fn glyph_instances(&self) -> Vec<&GlyphInstances>;
    // vector_instances を返す
    fn vector_instances(&self) -> Vec<&VectorInstances<String>>;
    // モーダル時のインスタンスを返す
    fn modal_instances(&self) -> (Vec<&GlyphInstances>, Vec<&VectorInstances<String>>);

    fn editor_operation(&mut self, op: &EditorOperation);
    fn model_operation(&mut self, op: &ModelOperation);
    fn current_string(&self) -> String;
    fn strings(&self) -> Vec<String>;
    fn chars(&self) -> HashSet<char>;

    // 今フォーカスが当たっているモデルのモードを返す
    //fn current_model_mode(&self) -> Option<ModelMode>;

    // カメラの位置を変更する。x_ratio, y_ratio はそれぞれ -1.0 から 1.0 までの値をとり、
    // アプリケーションのウインドウ上の位置を表す。(0.0, 0.0) はウインドウの中心を表す。
    fn move_to_position(&mut self, x_ratio: f32, y_ratio: f32);
}
