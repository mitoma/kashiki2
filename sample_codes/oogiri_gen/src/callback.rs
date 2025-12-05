use std::collections::BTreeMap;

use font_rasterizer::{context::WindowSize, glyph_vertex_buffer::Direction, time::now_millis};
use glam::Quat;
use text_buffer::action::EditorOperation;
use ui_support::{
    SimpleStateCallback,
    camera::{Camera, CameraAdjustment},
    layout_engine::{DefaultWorld, Model, ModelOperation, World},
    ui::{self, TextEdit},
};

pub(crate) struct Callback {
    world: ui_support::layout_engine::DefaultWorld,
    scenario: BTreeMap<u32, Vec<EditorOperation>>,
}

impl Callback {
    pub fn new(window_size: WindowSize, scenario: BTreeMap<u32, Vec<EditorOperation>>) -> Self {
        Self {
            world: DefaultWorld::new(window_size),
            scenario,
        }
    }
}

impl SimpleStateCallback for Callback {
    fn init(&mut self, context: &ui_support::ui_context::UiContext) {
        context.register_string("hello, world".into());
        let text_edit = TextEdit::default();
        self.world.add(Box::new(text_edit));
        self.world
            .look_current(CameraAdjustment::FitBothAndCentering);
    }

    fn resize(&mut self, _size: WindowSize) {}

    fn update(&mut self, context: &ui_support::ui_context::UiContext) {
        let time = now_millis();
        if let Some(operations) = self.scenario.get(&time) {
            println!("apply operations at time {}: {:?}", time, operations);

            for op in operations {
                match op {
                    EditorOperation::InsertString(str) => context.register_string(str.to_string()),
                    EditorOperation::InsertChar(c) => context.register_string(c.to_string()),
                    _ => {}
                }

                self.world.editor_operation(op);
            }
            self.world.look_current(CameraAdjustment::FitBothAndCentering);
        }

        self.world.update(context);
    }

    fn input(
        &mut self,
        context: &ui_support::ui_context::UiContext,
        event: &winit::event::WindowEvent,
    ) -> ui_support::InputResult {
        ui_support::InputResult::Noop
    }

    fn action(
        &mut self,
        context: &ui_support::ui_context::UiContext,
        action: stroke_parser::Action,
    ) -> ui_support::InputResult {
        ui_support::InputResult::Noop
    }

    fn render(&'_ mut self) -> ui_support::RenderData<'_> {
        ui_support::RenderData {
            camera: &self.world.camera(),
            glyph_instances: self.world.glyph_instances(),
            vector_instances: self.world.vector_instances(),
            glyph_instances_for_modal: vec![],
            vector_instances_for_modal: vec![],
        }
    }

    fn shutdown(&mut self) {}
}
