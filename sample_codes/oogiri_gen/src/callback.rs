use font_rasterizer::context::WindowSize;
use stroke_parser::Action;
use text_buffer::action::EditorOperation;
use ui_support::{
    InputResult, SimpleStateCallback,
    action::ActionProcessorStore,
    action_recorder::{ActionRecorder, FileActionRecordRepository},
    camera::CameraAdjustment,
    layout_engine::{DefaultWorld, World},
    ui::{ImeInput, TextEdit, caret_char},
    ui_context::CharEasingsPreset,
};

pub(crate) struct Callback {
    world: DefaultWorld,
    action_processor_store: ActionProcessorStore,
    recorder: ActionRecorder,
    ime: ImeInput,
}

impl Callback {
    pub fn new(window_size: WindowSize) -> Self {
        let mut action_processor_store = ActionProcessorStore::default();
        action_processor_store.add_default_system_processors();
        action_processor_store.add_default_edit_processors();
        action_processor_store.add_default_world_processors();

        let repository = FileActionRecordRepository::default();
        let mut recorder = ActionRecorder::new_with_time(Box::new(repository), 0);
        recorder.start_replay();

        Self {
            world: DefaultWorld::new(window_size),
            action_processor_store,
            recorder,
            ime: ImeInput::default(),
        }
    }
}

impl SimpleStateCallback for Callback {
    fn init(&mut self, context: &ui_support::ui_context::UiContext) {
        context.register_string("[]".to_string());
        let text_edit = TextEdit::default();
        self.world.add(Box::new(text_edit));
        self.world.look_current(CameraAdjustment::FitBoth);
        self.world
            .change_char_easings_preset(CharEasingsPreset::Default);
    }

    fn resize(&mut self, _size: WindowSize) {}

    fn update(&mut self, context: &ui_support::ui_context::UiContext) {
        self.world.look_current(CameraAdjustment::FitBoth);
        self.recorder.replay(context);
        self.world.update(context);
        self.ime.update(context);
    }

    fn input(
        &mut self,
        _context: &ui_support::ui_context::UiContext,
        _event: &winit::event::WindowEvent,
    ) -> ui_support::InputResult {
        ui_support::InputResult::Noop
    }

    fn action(
        &mut self,
        context: &ui_support::ui_context::UiContext,
        action: stroke_parser::Action,
    ) -> ui_support::InputResult {
        let result = self
            .action_processor_store
            .process(&action, context, &mut self.world);
        if result != InputResult::Noop {
            return result;
        }

        match action {
            Action::Keytype(c) => {
                context.register_string(c.to_string());
                self.world.editor_operation(&EditorOperation::InsertChar(c));
                InputResult::InputConsumed
            }
            Action::ImeInput(value) => {
                context.register_string(value.clone());
                self.ime
                    .apply_ime_event(&Action::ImeInput(value.clone()), context);
                self.world
                    .editor_operation(&EditorOperation::InsertString(value));
                InputResult::InputConsumed
            }
            Action::ImePreedit(value, position) => {
                context.register_string(value.clone());
                self.ime
                    .apply_ime_event(&Action::ImePreedit(value, position), context);
                InputResult::InputConsumed
            }
            Action::ImeEnable => InputResult::Noop,
            Action::ImeDisable => InputResult::Noop,
            Action::Command(..) => InputResult::Noop,
        }
    }

    fn render(&'_ mut self) -> ui_support::RenderData<'_> {
        let mut world_instances = self.world.glyph_instances();
        let mut ime_instances = self.ime.get_instances();
        world_instances.append(&mut ime_instances);
        ui_support::RenderData {
            camera: self.world.camera(),
            glyph_instances: world_instances,
            vector_instances: self.world.vector_instances(),
            glyph_instances_for_modal: vec![],
            vector_instances_for_modal: vec![],
        }
    }

    fn shutdown(&mut self) {}
}
