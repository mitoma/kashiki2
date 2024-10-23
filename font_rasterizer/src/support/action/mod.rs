mod system;
pub use system::*;

use stroke_parser::{Action, ActionArgument, CommandName, CommandNamespace};

use crate::{context::StateContext, layout_engine::World};

use super::InputResult;

#[derive(Default)]
pub struct ActionProcessorStore {
    processors: Vec<Box<dyn ActionProcessor>>,
}

impl ActionProcessorStore {
    pub fn add_default_system_processors(&mut self) {
        self.add_processor(Box::new(SystemExit));
        self.add_processor(Box::new(SystemToggleFullscreen));
        self.add_processor(Box::new(SystemToggleTitlebar));
        self.add_processor(Box::new(SystemChangeGlobalDirection));
        self.add_processor(Box::new(SystemChangeThemeUi));
        self.add_processor(Box::new(SystemChangeTheme));
    }

    pub fn add_lambda_processor(
        &mut self,
        namespace: &str,
        name: &str,
        f: fn(&ActionArgument, &StateContext, &mut dyn World) -> InputResult,
    ) {
        self.processors.push(Box::new(LambdaActionProcessor {
            namespace: CommandNamespace::from(namespace),
            name: CommandName::from(name),
            f,
        }));
    }

    pub fn add_processor(&mut self, processor: Box<dyn ActionProcessor>) {
        self.processors.push(processor);
    }

    pub fn remove_processor(&mut self, namespace: &CommandNamespace, name: &CommandName) {
        self.processors
            .retain(|processor| processor.namespace() != *namespace || processor.name() != *name);
    }

    pub fn process(
        &self,
        action: &Action,
        context: &StateContext,
        world: &mut dyn World,
    ) -> InputResult {
        if let Action::Command(namespace, name, argument) = action {
            for processor in &self.processors {
                if processor.namespace() == *namespace && processor.name() == *name {
                    return processor.process(argument, context, world);
                }
            }
        }
        InputResult::Noop
    }
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

// ----- impl system -----

// ----- impl edit -----
/*
pub struct EditForward;
impl ActionProcessor for EditForward {
    fn namespace(&self) -> CommandNamespace {
        "edit".into()
    }

    fn name(&self) -> CommandName {
        "forward".into()
    }

    fn process(
        &self,
        _arg: &ActionArgument,
        _context: &StateContext,
        world: &mut dyn World,
    ) -> Option<InputResult> {
        world.editor_operation(&EditorOperation::Forward);
        Some(InputResult::InputConsumed)
    }
}
 */
