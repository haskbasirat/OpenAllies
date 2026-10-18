//! Concrete4296B0 Created graph→428480 event→same50C800 owner.
//! The held shell/registry, immutable compiled source graph, real OS identity
//! borrower and actual World+E0 clock remain their existing owners.
use crate::{
    catalog::Catalog,
    definition_fx::ListKind,
    execution_context::ExecutionIds,
    fx_graph::{self, Command, CreationInputs, Graph},
    fx_manager,
    live_object_init::Record,
};
use aa_actors::{loading::LoadKind, ActorId};
use aa_mapworld::restore::RestoredRecord;
use std::sync::Arc;
type Result<T> = std::result::Result<T, String>;

/// Invoked only at the reached Created publication. The caller retains its
/// exact current held record and single World presentation manager.
pub fn publish(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    execution: &mut dyn ExecutionIds,
    presentation: &mut fx_manager::State<Command>,
    flag: u8,
) -> Result<()> {
    validate(record)?;
    let graph = Arc::new(
        Graph::compile(catalog, record.definition, ListKind::Created)
            .map_err(|failure| failure.error)?,
    );
    graph.require_catalog(catalog)?;
    let message = fx_graph::construct_created(
        graph,
        &mut Inputs {
            catalog,
            record,
            execution,
            presentation,
        },
    )?;
    //4296B0's empty/mute branch returns before50C800; it does not enqueue NULL.
    if let Some(message) = message {
        presentation.submit(Some(message), flag, &mut RequiredPresentation)?;
    }
    Ok(())
}
fn validate(record: &Record<'_>) -> Result<()> {
    if record.owner.is_null()
        || !record
            .registry
            .get(record.owner)
            .is_some_and(|entry| matches!(entry.value, RestoredRecord::Loading))
    {
        return Err(
            "Created FX current allocation must retain its exact Loading registry owner".into(),
        );
    }
    if !crate::load_spatial::same_proof(&record.shell.thing.definition, &record.definition.proof) {
        return Err("Created FX held shell and canonical source proof differ".into());
    }
    let valid = match record.definition.native_class {
        2 | 6 | 7 | 9 => {
            record.shell.kind == LoadKind::Object
                && record.shell.object.is_some()
                && record.shell.actor.is_none()
        }
        3 | 4 => {
            record.shell.kind == LoadKind::Actor
                && record.shell.object.is_some()
                && record.shell.actor.is_some()
        }
        _ => false,
    };
    if !valid {
        return Err(
            "Created FX requires original Object/Actor constructor storage and class virtuals"
                .into(),
        );
    }
    Ok(())
}
fn creator(record: &Record<'_>) -> Result<Option<ActorId>> {
    //428F40: Kingdom virtual+A8 returns the current Thing GTID immediately.
    //Ordinary Object/Area/Camera/Doodad virtual+B4 is false, skipping+108.
    match record.definition.native_class {
        4 => Ok(Some(record.owner)),
        2 | 6 | 7 | 9 => Ok(None),
        3 => {
            let raw = record
                .shell
                .actor
                .as_ref()
                .and_then(|actor| actor.words.get(&0xe8))
                .copied()
                .ok_or("Created FX Actor virtual108 requires its actual E8 owner field")?;
            if raw == 0 {
                return Ok(None);
            }
            let owner = ActorId::from_raw(raw);
            let entry = record
                .registry
                .get(owner)
                .ok_or("Created FX Actor E8 generation is not current")?;
            //The native+108 resolution supplies the current Thing, then
            //428F40 reads its+14 GTID. Do not return an unchecked raw handle.
            let id = match &entry.value {
                RestoredRecord::Thing(e) | RestoredRecord::Object(e) | RestoredRecord::Actor(e) => {
                    e.source.serialized_id
                }
                RestoredRecord::Team(e) => {
                    e.runtime
                        .as_ref()
                        .ok_or("Created FX Actor owner Team allocation is absent")?;
                    e.source.serialized_id
                }
                RestoredRecord::Kingdom(e) => {
                    e.runtime
                        .as_ref()
                        .ok_or("Created FX Actor owner Kingdom allocation is absent")?;
                    e.source.serialized_id
                }
                RestoredRecord::Loading => {
                    return Err(
                        "Created FX Actor owner currently has no held/stored Thing allocation"
                            .into(),
                    )
                }
            };
            if id != raw {
                return Err("Created FX Actor owner Thing identity differs".into());
            }
            Ok(Some(owner))
        }
        _ => Err("Created FX creator class virtuals are unavailable".into()),
    }
}
struct Inputs<'a, 'source, 'record> {
    catalog: &'a Catalog<'source>,
    record: &'a Record<'record>,
    execution: &'a mut dyn ExecutionIds,
    presentation: &'a fx_manager::State<Command>,
}
impl CreationInputs for Inputs<'_, '_, '_> {
    fn muted(&mut self) -> Result<bool> {
        Ok(self.presentation.muted())
    }
    fn creator_kingdom(&mut self) -> Result<Option<ActorId>> {
        creator(self.record)
    }
    fn current_position(&mut self) -> Result<[u32; 3]> {
        Ok(self.record.shell.thing.position_bits)
    }
    fn current_thing(&mut self) -> Result<Option<ActorId>> {
        Ok(Some(self.record.owner))
    }
    fn current_os_thread_id(&mut self) -> Result<u32> {
        self.execution.current_os_thread_id()
    }
    fn application_os_thread_id(&mut self) -> Result<u32> {
        Ok(self.execution.application_os_thread_id())
    }
    fn view_time(&mut self) -> Result<u32> {
        Err("Created FX on the actual application OS thread requires the real View+8C owner".into())
    }
    fn world_time(&mut self) -> Result<u32> {
        self.catalog
            .load_clock
            .map(|clock| clock.begin.0)
            .ok_or_else(|| {
                "Created FX off the application thread requires the actual World+E0 clock owner"
                    .into()
            })
    }
}
struct RequiredPresentation;
impl fx_manager::Effects<Command> for RequiredPresentation {
    fn deliver(&mut self, _view: u64, _message: Option<Command>) -> Result<()> {
        Err("Created FX requires the actual attached view43B870 delivery owner".into())
    }
    fn destroy(&mut self, _message: Command) -> Result<()> {
        Err(
            "Created FX reached428350 command destruction without its actual presentation effects"
                .into(),
        )
    }
}
