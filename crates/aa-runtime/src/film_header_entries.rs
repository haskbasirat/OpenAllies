//!5A17D0 current Session roster -> MapHeader entries and current parameter AI
//!profile labels. The same live roster is read lazily; no player/save snapshot.
use aa_actors::ActorId;
use aa_mapworld::header::{HeaderEntry, WorldParameters};
pub type Result<T> = std::result::Result<T, String>;
pub trait Players {
    type Player;
    fn first(&mut self) -> Result<Option<Self::Player>>;
    fn next(&mut self, player: &Self::Player) -> Result<Option<Self::Player>>;
    fn ai_byte(&mut self, player: &Self::Player) -> Result<u8>;
    fn kingdom_765e00(&mut self, player: &Self::Player) -> Result<Option<ActorId>>;
    fn name_24(&mut self, player: &Self::Player) -> Result<&[u16]>;
    fn display_name_08(&mut self, player: &Self::Player) -> Result<&[u16]>;
    fn is_local(&mut self, player: &Self::Player) -> Result<bool>;
}
pub trait World {
    fn kingdom_name_18(&mut self, kingdom: ActorId) -> Result<Vec<u16>>;
    ///Actual79EA90 ascending manager-slot first match, followed by actual
    ///Player+18 definition and its IDS+8. Null player/profile returns None.
    fn ai_profile_ids(&mut self, kingdom: Option<ActorId>) -> Result<Option<Vec<u16>>>;
}
///Concrete readers over the SAME published actor arena and actual SAI manager
///slot/AIPlayer allocations. These are immutable loans, not profile snapshots.
pub struct LiveWorld<'a> {
    actors: &'a crate::actor_world::ActorStore,
    ai_manager: &'a aa_mind::sai::manager::Sai,
    ai_players: &'a crate::load_sai_players::Arena,
}
impl<'a> LiveWorld<'a> {
    pub fn new(
        actors: &'a crate::actor_world::ActorStore,
        ai_manager: &'a aa_mind::sai::manager::Sai,
        ai_players: &'a crate::load_sai_players::Arena,
    ) -> Self {
        Self {
            actors,
            ai_manager,
            ai_players,
        }
    }
}
impl World for LiveWorld<'_> {
    fn kingdom_name_18(&mut self, kingdom: ActorId) -> Result<Vec<u16>> {
        let key = self
            .actors
            .key(kingdom)
            .ok_or("film current Kingdom generation is absent")?;
        let entity = self.actors.entity(key).map_err(|e| e.to_string())?;
        if !matches!(&entity.source,crate::actor_world::Source::Special(special) if matches!(special.source,crate::actor_world::SpecialSource::Kingdom(_))&&special.derived_payload_loaded)
        {
            return Err(
                "film current player reference is not an actual completed Kingdom allocation"
                    .into(),
            );
        }
        Ok(self
            .actors
            .object(key)
            .map_err(|e| e.to_string())?
            .thing
            .name_utf16
            .clone())
    }
    fn ai_profile_ids(&mut self, kingdom: Option<ActorId>) -> Result<Option<Vec<u16>>> {
        //Every Arena Player has a validated nonnull actual Kingdom association;
        //none can match a native null Kingdom passed to79EA90.
        let Some(kingdom) = kingdom else {
            return Ok(None);
        };
        Ok(self
            .ai_players
            .profile_for_kingdom(self.ai_manager, kingdom)?
            .map(|profile| profile.ids().encode_utf16().collect()))
    }
}
fn nul_prefix(words: &[u16]) -> &[u16] {
    &words[..words.iter().position(|&u| u == 0).unwrap_or(words.len())]
}
///The same retained parameter owner supplies50DC00 lookup and the later+2C
///write. A current fresh header never needs a detached WorldParameters DTO.
pub trait ParameterTarget {
    fn find_50dc00(&self, name: &[u16]) -> Option<usize>;
    fn assign_profile_2c(&mut self, index: usize, profile: Vec<u16>) -> Result<()>;
}
impl ParameterTarget for WorldParameters {
    fn find_50dc00(&self, name: &[u16]) -> Option<usize> {
        self.kingdoms
            .iter()
            .position(|p| nul_prefix(&p.id) == nul_prefix(name))
    }
    fn assign_profile_2c(&mut self, index: usize, profile: Vec<u16>) -> Result<()> {
        self.kingdoms
            .get_mut(index)
            .ok_or("5A17D0 captured parameter allocation is absent")?
            .string_2c = profile;
        Ok(())
    }
}
pub fn capture<P: Players, W: World>(
    parameters: &mut WorldParameters,
    entries: &mut Vec<HeaderEntry>,
    players: &mut P,
    world: &mut W,
) -> Result<()> {
    capture_current(parameters, entries, players, world)
}
pub fn capture_current<T: ParameterTarget, P: Players, W: World>(
    parameters: &mut T,
    entries: &mut Vec<HeaderEntry>,
    players: &mut P,
    world: &mut W,
) -> Result<()> {
    entries.clear(); //Native +48 count is cleared before the first Session read.
    let mut next = players.first()?;
    while let Some(player) = next {
        if players.ai_byte(&player)? != 0 && players.kingdom_765e00(&player)?.is_some() {
            let kingdom = players
                .kingdom_765e00(&player)?
                .ok_or("5A17D0 second765E00 is null before4BEDF0")?;
            let name = world.kingdom_name_18(kingdom)?;
            let parameter = parameters.find_50dc00(&name);
            let kingdom = players.kingdom_765e00(&player)?;
            if let Some(profile) = world.ai_profile_ids(kingdom)? {
                let index = parameter
                    .ok_or("5A17D0 positive AI profile has no matching50DC00 KingdomParameter")?;
                parameters.assign_profile_2c(index, profile)?;
            }
        }
        let flag = players.ai_byte(&player)?;
        let string_20 = players.name_24(&player)?.to_vec();
        let string_18 = players.display_name_08(&player)?.to_vec();
        let local = u8::from(players.is_local(&player)?);
        entries.push(HeaderEntry {
            flag,
            string_20,
            string_18,
            flag_v5: Some(local),
        });
        next = players.next(&player)?;
    }
    Ok(())
}
