//! Native4F57F0's actual no-merge-name path over retained major Kingdom slots.
//! The initial SyncA marker is committed before slot reads. A reached positive
//! alias requires4F48E0 and its later ownership/destruction services; it cannot
//! silently become an empty merge. Allocation/SEH parity is not claimed.
use crate::{
    actor_world::{ActorStore, Source, SpecialSource},
    catalog::Catalog,
    live_clocked::SyncLog,
    restoration::Staged,
};
use aa_actors::ActorId;
type Result<T> = std::result::Result<T, String>;

fn marker(sync: &mut SyncLog<'_>) -> Result<()> {
    let gate = if sync.owner.enabled && sync.owner.suspend < 1 {
        (sync.gate)()?
    } else {
        false
    };
    sync.owner.log_marker(gate, "MergeKingdoms");
    Ok(())
}

fn slots<'a>(
    major: &[Option<ActorId>],
    mut name: impl FnMut(ActorId) -> Result<&'a [u16]>,
) -> Result<()> {
    //4A5BC0/4F0680 and4F5998 skip actual holes, preserving vector order.
    for kingdom in major.iter().flatten() {
        //Native tests CString pointer and stored length, not its first unit.
        //A retained empty CString is enough for either native empty branch.
        if !name(*kingdom)?.is_empty() {
            return Err(format!(
                "4F57F0 Kingdom {} merge alias requires actual4F48E0 lookup, reverse membership transfer and destruction",
                kingdom.raw()
            ));
        }
    }
    Ok(())
}
fn merge_name(actors: &ActorStore, id: ActorId) -> Result<&[u16]> {
    let key = actors
        .key(id)
        .ok_or("MergeKingdoms major slot has no current GTID")?;
    let entity = actors.entity(key).map_err(|e| e.to_string())?;
    let Source::Special(value) = &entity.source else {
        return Err("MergeKingdoms major slot is not the retained special allocation".into());
    };
    let SpecialSource::Kingdom(saved) = &value.source else {
        return Err("MergeKingdoms major slot is not a Kingdom".into());
    };
    if saved.serialized_id != id.raw() {
        return Err("MergeKingdoms held Kingdom/GTID identity differs".into());
    }
    let shell = value
        .derived
        .as_ref()
        .ok_or("MergeKingdoms actual Kingdom derived owner absent")?;
    shell
        .strings_utf16
        .get(&0x248)
        .map(Vec::as_slice)
        .ok_or_else(|| "MergeKingdoms actual Kingdom+248 CString is unbound".into())
}
/// Called by the loaded mission owner after its resource return. The actual
/// Staged owns both the native major-slot vector and the same actor arena.
/// The borrowed Synchronizer is the process SyncA, never a new per-call log.
pub(crate) fn run(
    staged: &mut Staged,
    catalog: &Catalog<'_>,
    sync: &mut SyncLog<'_>,
) -> Result<()> {
    catalog.require_source_owner()?;
    let source = staged.definitions.combat_source()?;
    source.completed_means()?;
    if source.source_identities() != catalog.actors.sources.as_slice() {
        return Err("MergeKingdoms moved Bank belongs to another source universe".into());
    }
    let world = staged
        .load_world
        .as_ref()
        .ok_or("MergeKingdoms actual World owner absent")?;
    //77DC50 asks the host/thread gate only after these two actual log fields.
    marker(sync)?;
    slots(world.kingdoms.majors(), |id| merge_name(&staged.actors, id))
}

#[cfg(test)]
#[path = "merge_kingdoms_world_tests.rs"]
mod tests;
