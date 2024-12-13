mod edit;
mod system;
mod world;

pub use edit::*;
pub use system::*;
pub use world::*;

use std::{collections::BTreeMap, rc::Rc, sync::Mutex};
use stroke_parser::{Action, ActionArgument, CommandName, CommandNamespace};

use font_rasterizer::context::StateContext;

use crate::layout_engine::World;

use super::InputResult;

#[derive(Default)]
pub struct ActionProcessorStore {
    namespace_processors: BTreeMap<CommandNamespace, Rc<Mutex<dyn NamespaceActionProcessors>>>,
    processors: BTreeMap<(CommandNamespace, CommandName), Box<dyn ActionProcessor>>,
}

impl ActionProcessorStore {
    pub fn add_default_system_processors(&mut self) {
        self.add_processor(Box::new(SystemExit));
        self.add_processor(Box::new(SystemToggleFullscreen));
        self.add_processor(Box::new(SystemToggleTitlebar));
        self.add_processor(Box::new(SystemChangeGlobalDirection));
        self.add_processor(Box::new(SystemChangeThemeUi));
        self.add_processor(Box::new(SystemChangeTheme));
        self.add_processor(Box::new(SystemChangeFontUi));
        self.add_processor(Box::new(SystemChangeFont));
        self.add_processor(Box::new(SystemChangeWindowSizeUi));
        self.add_processor(Box::new(SystemChangeWindowSize));
    }

    pub fn add_default_edit_processors(&mut self) {
        self.add_processor(Box::new(EditReturn));
        self.add_processor(Box::new(EditBackspace));
        self.add_processor(Box::new(EditBackspaceWord));
        self.add_processor(Box::new(EditDelete));
        self.add_processor(Box::new(EditDeleteWord));
        self.add_processor(Box::new(EditPrevious));
        self.add_processor(Box::new(EditNext));
        self.add_processor(Box::new(EditBack));
        self.add_processor(Box::new(EditForward));
        self.add_processor(Box::new(EditBackWord));
        self.add_processor(Box::new(EditForwardWord));
        self.add_processor(Box::new(EditHead));
        self.add_processor(Box::new(EditLast));
        self.add_processor(Box::new(EditUndo));
        self.add_processor(Box::new(EditBufferHead));
        self.add_processor(Box::new(EditBufferLast));
        self.add_processor(Box::new(EditMark));
        self.add_processor(Box::new(EditUnmark));
        self.add_processor(Box::new(EditCopy));
        self.add_processor(Box::new(EditPaste));
        self.add_processor(Box::new(EditCut));
        self.add_processor(Box::new(EditHighlightUi));
        self.add_processor(Box::new(EditHighlight));
    }

    pub fn add_default_world_processors(&mut self) {
        self.add_processor(Box::new(WorldRemoveCurrent));
        self.add_processor(Box::new(WorldResetZoom));
        self.add_processor(Box::new(WorldLookCurrent));
        self.add_processor(Box::new(WorldLookNext));
        self.add_processor(Box::new(WorldLookPrev));
        self.add_processor(Box::new(WorldSwapNext));
        self.add_processor(Box::new(WorldSwapPrev));
        self.add_processor(Box::new(WorldFitWidth));
        self.add_processor(Box::new(WorldFitHeight));
        self.add_processor(Box::new(WorldFitByDirection));
        self.add_processor(Box::new(WorldForward));
        self.add_processor(Box::new(WorldBack));
        self.add_processor(Box::new(WorldChangeDirection));
        self.add_processor(Box::new(WorldIncreaseRowInterval));
        self.add_processor(Box::new(WorldDecreaseRowInterval));
        self.add_processor(Box::new(WorldIncreaseColInterval));
        self.add_processor(Box::new(WorldDecreaseColInterval));
        self.add_processor(Box::new(WorldIncreaseRowScale));
        self.add_processor(Box::new(WorldDecreaseRowScale));
        self.add_processor(Box::new(WorldIncreaseColScale));
        self.add_processor(Box::new(WorldDecreaseColScale));
        self.add_processor(Box::new(WorldTogglePsychedelic));
        self.add_processor(Box::new(WorldMoveToClick));
        self.add_processor(Box::new(WorldMoveToClickWithMark));
        self.add_processor(Box::new(WorldChangeLayout));
        self.add_processor(Box::new(WorldToggleMinBound));
    }

    pub fn add_lambda_processor(
        &mut self,
        namespace: &str,
        name: &str,
        f: fn(&ActionArgument, &StateContext, &mut dyn World) -> InputResult,
    ) {
        self.processors.insert(
            (CommandNamespace::from(namespace), CommandName::from(name)),
            Box::new(LambdaActionProcessor {
                namespace: CommandNamespace::from(namespace),
                name: CommandName::from(name),
                f,
            }),
        );
    }

    pub fn add_namespace_processors(
        &mut self,
        processor: Rc<Mutex<dyn NamespaceActionProcessors>>,
    ) {
        let namespace = processor.lock().unwrap().namespace().clone();
        self.namespace_processors.insert(namespace, processor);
    }

    pub fn add_processor(&mut self, processor: Box<dyn ActionProcessor>) {
        self.processors.insert(
            (processor.namespace().clone(), processor.name().clone()),
            processor,
        );
    }

    pub fn remove_processor(&mut self, namespace: &CommandNamespace, name: &CommandName) {
        self.processors.remove(&(namespace.clone(), name.clone()));
    }

    pub fn process(
        &self,
        action: &Action,
        context: &StateContext,
        world: &mut dyn World,
    ) -> InputResult {
        if let Action::Command(namespace, name, argument) = action {
            if let Some(processor) = self.namespace_processors.get(namespace) {
                let mut processor = processor.lock().unwrap();
                if processor.names().contains(name) {
                    return processor.process(name, argument, context, world);
                }
            }
            if let Some(processor) = self.processors.get(&(namespace.clone(), name.clone())) {
                return processor.process(argument, context, world);
            }
        }
        InputResult::Noop
    }

    pub fn is_registerd(&self, namespace: &CommandNamespace, name: &CommandName) -> bool {
        self.namespace_processors
            .get(namespace)
            .map(|p| p.lock().unwrap().names().contains(name))
            .unwrap_or_else(|| {
                self.processors
                    .contains_key(&(namespace.clone(), name.clone()))
            })
    }
}

pub trait NamespaceActionProcessors {
    fn namespace(&self) -> CommandNamespace;
    fn names(&self) -> &[CommandName];
    fn process(
        &mut self,
        command_name: &CommandName,
        arg: &ActionArgument,
        context: &StateContext,
        world: &mut dyn World,
    ) -> InputResult;
}

pub trait ActionProcessor {
    fn namespace(&self) -> CommandNamespace;
    fn name(&self) -> CommandName;
    fn process(
        &self,
        arg: &ActionArgument,
        context: &StateContext,
        world: &mut dyn World,
    ) -> InputResult;
}

pub struct LambdaActionProcessor<F>
where
    F: Fn(&ActionArgument, &StateContext, &mut dyn World) -> InputResult,
{
    namespace: CommandNamespace,
    name: CommandName,
    f: F,
}

impl ActionProcessor
    for LambdaActionProcessor<fn(&ActionArgument, &StateContext, &mut dyn World) -> InputResult>
{
    fn namespace(&self) -> CommandNamespace {
        self.namespace.clone()
    }

    fn name(&self) -> CommandName {
        self.name.clone()
    }

    fn process(
        &self,
        arg: &ActionArgument,
        context: &StateContext,
        world: &mut dyn World,
    ) -> InputResult {
        (self.f)(arg, context, world)
    }
}
