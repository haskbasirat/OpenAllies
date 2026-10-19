//!Read-only reads of the SAME live World owners for the in-game interface:
//!EconomyBar 602D10 (Kingdom +1A0/+1AC/+1B8/+1C4 through 602760/5C1760),
//!612010/60BAE0 (Kingdom +1F0 shortage ratio), FactionExperience 604130
//!(Kingdom +230 Commander +C balance and its FactionData+3C maximum),
//!TimeMessage 5D27B0 (VWorld B36538 +8C) and the current SVar
//!EconomyLimitedResourceMax. Nothing is copied into a second owner, no
//!value is defaulted, and an unwritten field is an error.
use crate::{
    actor_world::{ActorStore, Source, SpecialSource},
    load_view_variables::{CurrentValue, State},
    view_init_prefix,
    world_revision::ConstructedWorldRevision,
};
use aa_actors::ActorId;
use aa_mapworld::special_state::SpecialShell;
type Result<T> = std::result::Result<T, String>;

///The current Kingdom's derived shell (the same allocation the economy
///tick and saved-economy owners write).
pub fn kingdom_shell(actors: &ActorStore, kingdom: ActorId) -> Result<&SpecialShell> {
    let key = actors
        .key(kingdom)
        .ok_or("HUD Kingdom generation is not live")?;
    let entity = actors.entity(key).map_err(|e| e.to_string())?;
    let Source::Special(special) = &entity.source else {
        return Err("HUD Kingdom identity is not a special actor".into());
    };
    if !matches!(special.source, SpecialSource::Kingdom(_)) {
        return Err("HUD Kingdom identity is not a Kingdom".into());
    }
    special
        .derived
        .as_ref()
        .ok_or_else(|| "HUD Kingdom fields are not constructed".into())
}

///binary32 words in native registry order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KingdomEconomy {
    pub production_1a0: Vec<u32>,
    pub preview_production_1ac: Vec<u32>,
    pub upkeep_1b8: Vec<u32>,
    pub stock_1c4: Vec<u32>,
}
pub fn kingdom_economy(actors: &ActorStore, kingdom: ActorId) -> Result<KingdomEconomy> {
    let shell = kingdom_shell(actors, kingdom)?;
    let vector = |offset: u32| -> Result<Vec<u32>> {
        shell
            .vectors
            .get(&offset)
            .map(|v| v.values.clone())
            .ok_or_else(|| format!("HUD Kingdom+{offset:X} vector is not constructed"))
    };
    Ok(KingdomEconomy {
        production_1a0: vector(0x1a0)?,
        preview_production_1ac: vector(0x1ac)?,
        upkeep_1b8: vector(0x1b8)?,
        stock_1c4: vector(0x1c4)?,
    })
}
///Kingdom+1F0 binary32 shortage ratio (612010 / 60BAE0 compare it with 0.0).
pub fn kingdom_shortage_ratio_1f0(actors: &ActorStore, kingdom: ActorId) -> Result<u32> {
    kingdom_shell(actors, kingdom)?
        .words
        .get(&0x1f0)
        .copied()
        .ok_or_else(|| "HUD Kingdom+1F0 shortage ratio is unwritten".into())
}
///Kingdom+230 Commander: Some((balance +C, maximum)) as binary32 words, None
///for the native NULL (+230 == 0). An unbound owner is an error.
pub fn kingdom_commander_experience_230(
    actors: &ActorStore,
    kingdom: ActorId,
) -> Result<Option<(u32, u32)>> {
    let shell = kingdom_shell(actors, kingdom)?;
    match &shell.commander {
        Some(commander) => Ok(Some((
            commander.experience.balance.0,
            commander.experience.maximum.0,
        ))),
        None if shell.words.get(&0x230) == Some(&0) => Ok(None),
        None => Err("HUD Kingdom+230 Commander owner is unbound".into()),
    }
}
///VWorld+8C binary32 word on the SAME current VWorld of this World.
pub fn view_time_8c<G>(
    view: &view_init_prefix::State<G>,
    world: &ConstructedWorldRevision,
) -> Result<u32> {
    view.message_word_8c_for(world)
}
///Current SVar EconomyLimitedResourceMax (native Int).
pub fn economy_limited_resource_max(variables: &State) -> Result<i32> {
    let index = variables
        .registrations()
        .find(|v| v.name().eq_ignore_ascii_case("EconomyLimitedResourceMax"))
        .ok_or("current EconomyLimitedResourceMax registration absent")?
        .ordinal() as usize;
    match variables.current(index)? {
        CurrentValue::Int(v) => Ok(*v as i32),
        _ => Err("current EconomyLimitedResourceMax is not native Int".into()),
    }
}
