//!524B80 arrival publication on the same installed Regiment Mind.
//!The saved collection owner is reused; fresh fields transfer once from the
//!actual526260 remainder, never from a synthetic default or component clone.
use crate::actor_world::{ActorAccess, RuntimeComponent};
use aa_actors::{ActorId, components::ComponentSlot};
use aa_mind::{Arena, MindId, construct::Kind};
fn error(e: impl std::fmt::Display) -> aa_mind::Error {
    aa_mind::Error::World(e.to_string())
}
pub(crate) fn ensure(
    actors: &mut ActorAccess<'_>,
    arena: &Arena,
    owner: ActorId,
    mind: MindId,
) -> aa_mind::Result<()> {
    let Some(RuntimeComponent::Mind(installed)) = actors
        .component(owner, ComponentSlot::Mind)
        .map_err(error)?
    else {
        return Err(error("CAI arrival actual installed Mind is absent"));
    };
    if installed.kind != Kind::Regiment
        || installed.mind_id != mind
        || arena.mind(mind)?.actor != owner.raw()
    {
        return Err(error(
            "CAI arrival actual Installed/Arena/Actor owner differs",
        ));
    }
    if actors.regiment_owned.contains_key(&mind) {
        if [0x2bc, 0x2c0]
            .iter()
            .any(|off| installed.words.contains_key(off))
        {
            return Err(error(
                "CAI arrival retained list has duplicate Installed head/tail owners",
            ));
        }
        return Ok(());
    }
    let Some(RuntimeComponent::Mind(installed)) = actors
        .component_mut(owner, ComponentSlot::Mind)
        .map_err(error)?
    else {
        unreachable!("same borrowed actor has no callbacks")
    };
    let owned = aa_mind::regiment_load::take_constructed_owned(installed).map_err(error)?;
    let previous = actors.regiment_owned.insert(mind, owned);
    debug_assert!(
        previous.is_none(),
        "same borrowed map cannot change without a callback"
    );
    Ok(())
}
///The actual524B80 tail, after522890 and the lazy SessionE4!=3 predicate.
///GTID is the raw Actor14 value captured by that caller; no re-resolution or
///validity filtering is added to the native4D0900 payload store.
pub(crate) fn push_front(
    actors: &mut ActorAccess<'_>,
    arena: &Arena,
    owner: ActorId,
    mind: MindId,
    gtid: u32,
) -> aa_mind::Result<()> {
    ensure(actors, arena, owner, mind)?;
    actors
        .regiment_owned
        .get_mut(&mind)
        .expect("validated same owner")
        .tracked_words
        .push_front(gtid);
    Ok(())
}
///For the concrete CAISleep consumer: registry resolution of this value must
///precede pop_front, matching524F62..524FB3, then membership/522890 callbacks.
pub(crate) fn front(
    actors: &mut ActorAccess<'_>,
    arena: &Arena,
    owner: ActorId,
    mind: MindId,
) -> aa_mind::Result<Option<u32>> {
    ensure(actors, arena, owner, mind)?;
    Ok(actors
        .regiment_owned
        .get(&mind)
        .expect("validated same owner")
        .tracked_words
        .front())
}
pub(crate) fn pop_front(
    actors: &mut ActorAccess<'_>,
    arena: &Arena,
    owner: ActorId,
    mind: MindId,
) -> aa_mind::Result<Option<u32>> {
    ensure(actors, arena, owner, mind)?;
    Ok(actors
        .regiment_owned
        .get_mut(&mind)
        .expect("validated same owner")
        .tracked_words
        .pop_front())
}
///524910's final cleanup reaches the SAME list after its current Think. The
///Update caller owns the native ordering; nodes drop in head-to-tail order.
pub(crate) fn clear(
    actors: &mut ActorAccess<'_>,
    arena: &Arena,
    owner: ActorId,
    mind: MindId,
) -> aa_mind::Result<()> {
    ensure(actors, arena, owner, mind)?;
    let list = &mut actors
        .regiment_owned
        .get_mut(&mind)
        .expect("validated same owner")
        .tracked_words;
    while list.pop_front().is_some() {}
    Ok(())
}
