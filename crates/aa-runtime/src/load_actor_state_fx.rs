//! Actual matched4D0A00 ->4293C0 event on the held Actor allocation and the
//! existing World's presentation queue. No-match never calls this publisher.
use crate::{
    catalog::Catalog,
    fx_graph::{self, Command, CreationInputs, MatchedState, StateCreationInputs},
    fx_manager,
    live_object_init::Record,
};
type Result<T> = std::result::Result<T, String>;

/// The caller must supply the actual B28114 global when the constructor
/// reaches it. An unavailable owner is an error, never an assumed zero vector.
pub fn publish(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    matched: MatchedState,
    presentation: &mut fx_manager::State<Command>,
    global_position: &mut dyn FnMut() -> Result<[u32; 3]>,
) -> Result<()> {
    crate::load_creation_fx::validate(record)?;
    matched.graph.require_catalog(catalog)?;
    if !matched
        .graph
        .owner()
        .eq_ignore_ascii_case(&record.definition.proof.id)
    {
        return Err("state FX graph differs from held ActorData source".into());
    }
    let message = fx_graph::construct_actor_state(
        matched,
        &mut Inputs {
            catalog,
            record,
            presentation,
            global_position,
        },
    )?;
    if let Some(message) = message {
        presentation.submit(Some(message), 0, &mut RequiredPresentation)?;
    }
    Ok(())
}
struct Inputs<'a, 'source, 'record> {
    catalog: &'a Catalog<'source>,
    record: &'a Record<'record>,
    presentation: &'a fx_manager::State<Command>,
    global_position: &'a mut dyn FnMut() -> Result<[u32; 3]>,
}
impl CreationInputs for Inputs<'_, '_, '_> {
    fn muted(&mut self) -> Result<bool> {
        Ok(self.presentation.muted())
    }
    fn creator_kingdom(&mut self) -> Result<Option<aa_actors::ActorId>> {
        crate::load_creation_fx::creator(self.record)
    }
    fn current_position(&mut self) -> Result<[u32; 3]> {
        Ok(self.record.shell.thing.position_bits)
    }
    fn current_thing(&mut self) -> Result<Option<aa_actors::ActorId>> {
        Ok(Some(self.record.owner))
    }
    fn current_os_thread_id(&mut self) -> Result<u32> {
        let mut execution = self
            .catalog
            .load_execution
            .try_borrow_mut()
            .map_err(|e| e.to_string())?;
        execution
            .as_deref_mut()
            .ok_or("state FX requires actual running resource worker")?
            .current_os_thread_id()
    }
    fn application_os_thread_id(&mut self) -> Result<u32> {
        let execution = self
            .catalog
            .load_execution
            .try_borrow()
            .map_err(|e| e.to_string())?;
        Ok(execution
            .as_deref()
            .ok_or("state FX requires actual application OS identity owner")?
            .application_os_thread_id())
    }
    fn view_time(&mut self) -> Result<u32> {
        Err("state FX on application thread requires actual View+8C clock".into())
    }
    fn world_time(&mut self) -> Result<u32> {
        self.catalog
            .load_clock
            .map(|v| v.begin.0)
            .ok_or("state FX requires actual World+E0 clock".into())
    }
}
impl StateCreationInputs for Inputs<'_, '_, '_> {
    fn global_position(&mut self) -> Result<[u32; 3]> {
        (self.global_position)()
    }
}
struct RequiredPresentation;
impl fx_manager::Effects<Command> for RequiredPresentation {
    fn deliver(&mut self, _view: u64, _message: Option<Command>) -> Result<()> {
        Err("state FX requires attached View43B870 delivery owner".into())
    }
    fn destroy(&mut self, _message: Command) -> Result<()> {
        Err("state FX requires actual428350 destruction effects".into())
    }
}
