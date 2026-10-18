//! Native773E30 caller over the actual retained Session roster.
//! This is a procedure, not battle readiness. Every external positive owner
//! is reached lazily; required service errors preserve earlier765E10/label
//! writes. The same ActorStore, Kingdom slots and mutable text table are used.
use super::*;
use aa_runtime::load_tail_world::KingdomId as ActorId;
use aa_runtime::{
    actor_world::{ActorStore, Source, SpecialSource},
    application_mode::GameAppState,
};

pub struct Context<'a> {
    pub actors: &'a mut ActorStore,
    /// Actual GWorld148/14C slots, native order; never a GTID-sorted scan.
    pub kingdom_slots: &'a [Option<ActorId>],
    pub live_text: &'a mut Localization,
    pub application: &'a GameAppState,
}
/// Remaining external owners of the native caller, without success defaults.
/// Concrete implementations must borrow the actual creator88/AI/glyph owners.
pub trait Effects {
    type Creator;
    type Glyph;
    fn start_diagnostic(&mut self) -> Result<()>;
    fn create_local_sai(
        &mut self,
        owner: &mut LocalBootstrap,
        player: &PlayerHandle,
        world: &mut Context<'_>,
    ) -> Result<()>;
    fn bind_diagnostic(&mut self, player_name: &[u16], kingdom_name: &[u16]) -> Result<()>;
    fn editor_nation_text(&mut self, kingdom: ActorId, world: &mut Context<'_>)
    -> Result<Vec<u16>>;
    ///4F1310 reaches UserSettings+50 option1 only if actual GameAppState+C==0.
    fn mode_zero_option(&mut self) -> Result<u32>;
    ///50DC00: find actual creator88 record by Kingdom Thing18, then49AE00
    ///write that same record+4/+8 pair with player+8 display name.
    fn creator_set_name(
        &mut self,
        kingdom_name: &[u16],
        player_name: &[u16],
        table: &mut Localization,
    ) -> Result<Self::Creator>;
    fn player_glyph(
        &mut self,
        owner: &LocalBootstrap,
        player: &PlayerHandle,
    ) -> Result<Option<Self::Glyph>>;
    fn glyph_index(&mut self, glyph: &Self::Glyph) -> Result<u16>;
    fn creator_set_glyph(&mut self, creator: &Self::Creator, text: &[u16]) -> Result<()>;
}
fn eq_wcs(a: &[u16], b: &[u16]) -> bool {
    a.split(|v| *v == 0).next().unwrap_or_default()
        == b.split(|v| *v == 0).next().unwrap_or_default()
}
///4F48E0/4BEDF0 reads Thing18 and _wcscmp; first matching real slot wins.
pub fn find_kingdom(world: &Context<'_>, name: &[u16]) -> Result<Option<ActorId>> {
    find_kingdom_in_store(world.actors,world.kingdom_slots,name)
}
///The same read-only4F48E0 algorithm also serves76C2B0 after staging. No
///replacement mutable BeginPlay Context or cloned actors/slots are required.
pub fn find_kingdom_in_store(actors:&ActorStore,kingdom_slots:&[Option<ActorId>],name:&[u16])->Result<Option<ActorId>>{
    for id in kingdom_slots.iter().flatten() {
        let key = actors
            .key(*id)
            .ok_or_else(|| missing("4F48E0 Kingdom slot has no live allocation"))?;
        let entity = actors
            .entity(key)
            .map_err(|e| missing(e.to_string()))?;
        if !matches!(&entity.source,Source::Special(s) if matches!(s.source,SpecialSource::Kingdom(_)))
        {
            return Err(missing("4F48E0 retained slot is not its actual Kingdom"));
        }
        let object = actors
            .object(key)
            .map_err(|e| missing(e.to_string()))?;
        if eq_wcs(&object.thing.name_utf16, name) {
            return Ok(Some(*id));
        }
    }
    Ok(None)
}
///50E0D0's real this adjustment is+194, not base ActorDC/E0.
fn set_kingdom_label(world: &mut Context<'_>, kingdom: ActorId, supplied: &[u16]) -> Result<()> {
    let key = world
        .actors
        .key(kingdom)
        .ok_or_else(|| missing("50E0D0 Kingdom no longer live"))?;
    let entity = world
        .actors
        .entity_mut(key)
        .map_err(|e| missing(e.to_string()))?;
    let Source::Special(special) = &mut entity.source else {
        return Err(missing("50E0D0 owner is not Kingdom"));
    };
    if !matches!(special.source, SpecialSource::Kingdom(_)) {
        return Err(missing("50E0D0 owner is not Kingdom"));
    }
    let shell = special
        .derived
        .as_mut()
        .ok_or_else(|| missing("50E0D0 Kingdom derived fields absent"))?;
    let label_key = shell
        .strings_utf16
        .get(&0x194)
        .ok_or_else(|| missing("50E0D0 actual Kingdom194 key absent"))?
        .clone();
    let text = shell
        .strings_utf16
        .get_mut(&0x198)
        .ok_or_else(|| missing("50E0D0 actual Kingdom198 text absent"))?;
    aa_runtime::live_map_strings::set_existing_pair(world.live_text, &label_key, text, supplied)
        .map_err(missing)
}
impl LocalBootstrap {
    ///7664D0 is the actual staging writer of Player+24/+2C. This assigns
    ///the supplied native string; choosing/authorizing a map slot belongs to
    ///the caller's original staging/auto-assignment path.
    pub fn set_kingdom_name_7664d0(&mut self, player: &PlayerHandle, name: &[u16]) -> Result<()> {
        let target = self.player_mut(player)?;
        target.core.kingdom = name.to_vec();
        target.core.flag_2c = true;
        Ok(())
    }
    ///773E30 over the same retained allocations. Completion is not readiness.
    pub fn prepare_players_for_begin_playing(
        &mut self,
        world: &mut Context<'_>,
        effects: &mut impl Effects,
    ) -> Result<()> {
        prepare(&mut Live {
            owner: self,
            world,
            effects,
        })
    }
}

/// Shared native caller kernel. Live implements every getter against its real
///owner; fixture hosts are used only in tests to replay native leaf seams.
trait World {
    type Player;
    type Kingdom: Copy;
    type Creator;
    type Glyph;
    fn start_diagnostic(&mut self) -> Result<()>;
    fn first_player(&mut self) -> Result<Option<Self::Player>>;
    fn next_player(&mut self, p: &Self::Player) -> Result<Option<Self::Player>>;
    fn kingdom_name(&mut self, p: &Self::Player) -> Result<Vec<u16>>;
    fn find_kingdom(&mut self, name: &[u16]) -> Result<Option<Self::Kingdom>>;
    fn replace_kingdom(&mut self, p: &Self::Player, k: Option<Self::Kingdom>) -> Result<()>;
    fn ai(&mut self, p: &Self::Player) -> Result<bool>;
    fn local_link(&mut self, p: &Self::Player) -> Result<bool>;
    fn create_sai(&mut self, p: &Self::Player) -> Result<()>;
    fn diagnostic(&mut self, p: &Self::Player) -> Result<()>;
    fn application_mode(&mut self) -> Result<u32>;
    fn editor_nation_text(&mut self, k: Self::Kingdom) -> Result<Vec<u16>>;
    fn set_kingdom_label(&mut self, k: Self::Kingdom, text: &[u16]) -> Result<()>;
    fn normal_name_mode(&mut self) -> Result<u32>;
    fn display_name(&mut self, p: &Self::Player) -> Result<Vec<u16>>;
    fn kingdom_thing_name(&mut self, k: Self::Kingdom) -> Result<Vec<u16>>;
    fn creator_set_name(&mut self, kingdom: &[u16], name: &[u16]) -> Result<Self::Creator>;
    fn player_glyph(&mut self, p: &Self::Player) -> Result<Option<Self::Glyph>>;
    fn glyph_index(&mut self, g: &Self::Glyph) -> Result<u16>;
    fn creator_set_glyph(&mut self, c: &Self::Creator, text: &[u16]) -> Result<()>;
}
fn prepare<W: World>(w: &mut W) -> Result<()> {
    w.start_diagnostic()?;
    let mut next = w.first_player()?;
    while let Some(player) = next {
        let name = w.kingdom_name(&player)?;
        let kingdom = w.find_kingdom(&name)?;
        w.replace_kingdom(&player, kingdom)?;
        if w.ai(&player)? && w.local_link(&player)? {
            w.create_sai(&player)?;
        }
        w.diagnostic(&player)?;
        if w.application_mode()? == 4 && kingdom.is_some() {
            let kingdom = kingdom.unwrap();
            let text = w.editor_nation_text(kingdom)?;
            w.set_kingdom_label(kingdom, &text)?;
        } else {
            //Native evaluates4F1310 BEFORE testing the captured Kingdom null.
            let mode = w.normal_name_mode()?;
            if mode == 0
                && let Some(kingdom) = kingdom
            {
                let name = w.display_name(&player)?;
                w.set_kingdom_label(kingdom, &name)?;
                let kingdom_name = w.kingdom_thing_name(kingdom)?;
                let name = w.display_name(&player)?;
                let creator = w.creator_set_name(&kingdom_name, &name)?;
                let text = if w.player_glyph(&player)?.is_some() {
                    let glyph = w.player_glyph(&player)?.ok_or_else(|| {
                        missing("7666C0 second glyph lookup became null before native+18 read")
                    })?;
                    vec![w.glyph_index(&glyph)?]
                } else {
                    Vec::new()
                };
                w.creator_set_glyph(&creator, &text)?;
            }
        }
        //773E62 reloads actual current link+4 after the entire player body.
        next = w.next_player(&player)?;
    }
    Ok(())
}
struct Live<'a, 'world, E> {
    owner: &'a mut LocalBootstrap,
    world: &'a mut Context<'world>,
    effects: &'a mut E,
}
impl<E: Effects> Live<'_, '_, E> {
    fn at(&self, slot: usize) -> Option<PlayerHandle> {
        self.owner.session.players.get(slot).map(|p| PlayerHandle {
            owner: Arc::downgrade(&self.owner.identity),
            id: p.core.id,
            slot,
            allocation: Arc::downgrade(&p.allocation_lifetime),
        })
    }
}
impl<E: Effects> World for Live<'_, '_, E> {
    type Player = PlayerHandle;
    type Kingdom = ActorId;
    type Creator = E::Creator;
    type Glyph = E::Glyph;
    fn start_diagnostic(&mut self) -> Result<()> {
        self.effects.start_diagnostic()
    }
    fn first_player(&mut self) -> Result<Option<PlayerHandle>> {
        Ok(self.at(0))
    }
    fn next_player(&mut self, p: &PlayerHandle) -> Result<Option<PlayerHandle>> {
        self.owner.player(p)?;
        Ok(self.at(p
            .slot
            .checked_add(1)
            .ok_or_else(|| missing("player list traversal overflow"))?))
    }
    fn kingdom_name(&mut self, p: &PlayerHandle) -> Result<Vec<u16>> {
        Ok(self.owner.player(p)?.core.kingdom.clone())
    }
    fn find_kingdom(&mut self, name: &[u16]) -> Result<Option<ActorId>> {
        find_kingdom(self.world, name)
    }
    fn replace_kingdom(&mut self, p: &PlayerHandle, k: Option<ActorId>) -> Result<()> {
        self.owner
            .replace_kingdom_765e10(p, k, self.world.actors, self.world.live_text)
    }
    fn ai(&mut self, p: &PlayerHandle) -> Result<bool> {
        Ok(self.owner.player(p)?.core.ai)
    }
    fn local_link(&mut self, p: &PlayerHandle) -> Result<bool> {
        Ok(self.owner.player(p)?.core.link == PeerLink::Node)
    }
    fn create_sai(&mut self, p: &PlayerHandle) -> Result<()> {
        self.effects.create_local_sai(self.owner, p, self.world)
    }
    fn diagnostic(&mut self, p: &PlayerHandle) -> Result<()> {
        self.effects.bind_diagnostic(
            &self.owner.player(p)?.core.name,
            &self.owner.player(p)?.core.kingdom,
        )
    }
    fn application_mode(&mut self) -> Result<u32> {
        Ok(self.world.application.mode())
    }
    fn editor_nation_text(&mut self, k: ActorId) -> Result<Vec<u16>> {
        self.effects.editor_nation_text(k, self.world)
    }
    fn set_kingdom_label(&mut self, k: ActorId, text: &[u16]) -> Result<()> {
        set_kingdom_label(self.world, k, text)
    }
    fn normal_name_mode(&mut self) -> Result<u32> {
        if self.world.application.mode() == 0 {
            self.effects.mode_zero_option()
        } else {
            Ok(0)
        }
    }
    fn display_name(&mut self, p: &PlayerHandle) -> Result<Vec<u16>> {
        Ok(self.owner.player(p)?.core.name.clone())
    }
    fn kingdom_thing_name(&mut self, k: ActorId) -> Result<Vec<u16>> {
        let key = self
            .world
            .actors
            .key(k)
            .ok_or_else(|| missing("creator lookup Kingdom no longer live"))?;
        Ok(self
            .world
            .actors
            .object(key)
            .map_err(|e| missing(e.to_string()))?
            .thing
            .name_utf16
            .clone())
    }
    fn creator_set_name(&mut self, k: &[u16], name: &[u16]) -> Result<Self::Creator> {
        self.effects.creator_set_name(k, name, self.world.live_text)
    }
    fn player_glyph(&mut self, p: &PlayerHandle) -> Result<Option<Self::Glyph>> {
        self.effects.player_glyph(self.owner, p)
    }
    fn glyph_index(&mut self, g: &Self::Glyph) -> Result<u16> {
        self.effects.glyph_index(g)
    }
    fn creator_set_glyph(&mut self, c: &Self::Creator, text: &[u16]) -> Result<()> {
        self.effects.creator_set_glyph(c, text)
    }
}

#[cfg(test)]
#[path = "local_begin_play_tests.rs"]
mod tests;
