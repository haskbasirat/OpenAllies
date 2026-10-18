//! Actual43B870 command acceptance into the retained VWorld list. This does
//! not execute later FX update/render or instantiate an absent FXL definition.
use crate::{fx_graph::Command, view_fx_command_list::{Key, List, Message}};

pub struct State { list: List<Command> }
impl State {
    pub(crate) fn from_native_constructor() -> Self {
        Self { list: List::from_native_constructor() }
    }
    pub fn head(&self) -> Option<&Key> { self.list.head() }
    pub fn tail(&self) -> Option<&Key> { self.list.tail() }
    pub fn native_count(&self) -> u32 { self.list.native_count() }
    pub fn owned_count(&self) -> usize { self.list.owned_count() }
    pub fn command(&self, key: &Key) -> Option<&Command> { self.list.get(key) }
    ///Moves the original payload only after native429020's reached work.
    ///The ordinary constructor-proven +60=null branch requires no VFX sink.
    pub fn accept_43b870(&mut self, message: &mut Option<Command>) -> Result<Key, String> {
        self.list.append(message, |command| {
            if command.fxl_definition().is_some() {
                Err("429020 requires actual FXL definition virtualBC and45A940 retention owner".into())
            } else { Ok(()) }
        })
    }
}
impl Message for Command {
    fn set_next(&mut self, next: Option<Key>) { self.set_view_next(next); }
}

#[cfg(test)]
#[path="view_fx_commands_tests.rs"]
mod tests;
