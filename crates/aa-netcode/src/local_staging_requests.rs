//! Custom-game staging orders on the actual local Host (AA.exe), processed in
//! the order the StagingMenu sent them, between the selected-settings player
//! refresh 7791F0 and the launch 77A950:
//!
//! * TGC_CreateExplicitAIOrder (request 132 / decree 143, process 0x75E030)
//!   -> AdminNode 0x754F50: the 0x76ADE0 countdown rejection, the ordering
//!   player's link (Admin+0x284 -> 0x8569A0; NULL asserts and links NULL),
//!   `++Session+E8`, CreatePlayer 0x773710 (ai, not implicit), 0x7664B0 text
//!   +0x14 = the SAI profile, the name by 0x7664F0 (no profile) or the unique
//!   name 0x766560, then KKC_Session::SetPlayerKingdom 0x777F00(player,
//!   kingdom, 1).
//! * TGC_SetKingdomNationOrder (137/148, 0x75E1C0) -> 0x777D90 -> 0x777CD0.
//! * TGC_SetKingdomFactionOrder (138/149, 0x75E1F0) -> 0x777700 -> 0x777610.
//! * TGC_SetAIHandicapOrder (140/151, 0x75E340) -> 0x771880.
//!
//! Oracle: research/development10/custom-game/native_explicit_ai.py (every
//! listed function executed unchanged, original and v14), replayed over
//! [`Laboratory`] by the tests below; the live Host runs the same functions
//! through [`Host`] on its actual Session and Creator. Branches a fresh local
//! Host cannot reach (an AI that already owns a kingdom) stay explicit errors.
use super::*;
use aa_runtime::creator_staging_rows as rows;
use serde::{Deserialize, Serialize};

/// One staging order as its decree carries it. Definition references are
/// canonical IDS (`None` = the NULL definition).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StagingRequest {
    CreateExplicitAi { kingdom: Vec<u16>, profile: Vec<u16>, name: Vec<u16> },
    SetKingdomNation { kingdom: Vec<u16>, nation: Option<Vec<u16>> },
    SetKingdomFaction { kingdom: Vec<u16>, faction: Option<Vec<u16>> },
    SetAiHandicap { kingdom: Vec<u16>, handicap: Option<Vec<u16>> },
}

/// 0x76DA60 keys (posted as UI message kind 0x11 when the ordering player is
/// the local one).
pub const COUNTING_DOWN_KEY: &str = "session_order_rejected_currently_counting_down";
pub const ORDER_FAILED_KEY: &str = "session_order_failed";
pub const NATION_FAILED_KEY: &str = "set_kingdom_nation_failed";
pub const FACTION_FAILED_KEY: &str = "set_kingdom_faction_failed";
pub const PLAYER_KINGDOM_FAILED_KEY: &str = "set_player_kingdom_failed";

pub trait Effects {
    /// 0x76DA60(key): log, then the UI message when Admin+8 is Admin+0x284.
    fn session_message_76da60(&mut self, key: &str, shown: bool) -> Result<()>;
    /// 0x404F60 "NET: GetOrderingPlayer() is NULL" (0x755014).
    fn ordering_player_null(&mut self) -> Result<()>;
    /// 0x773710's publication diagnostic.
    fn creating_player(&mut self, id: u32, ai: bool, implicit: bool) -> Result<()>;
    /// 0x404F90 logs of 0x777CD0 / 0x777610 (kingdom, definition IDS).
    fn check_nation(&mut self, kingdom: &[u16], nation: &[u16]) -> Result<()>;
    fn check_faction(&mut self, kingdom: &[u16], faction: &[u16]) -> Result<()>;
}

/// The KingdomParameter fields these processes read.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Row {
    pub playable_28: bool,
    pub nation_18: Vec<u16>,
    pub faction_1c: Vec<u16>,
    pub handicap_24: Vec<u16>,
}

/// The Session/Admin/Creator owners the processes act on. Player slots are
/// positions in the +0xBC roster.
pub trait Host {
    fn countdown_118(&self) -> i32;
    /// Admin+0x284 and Admin+8 (player ids), and 0x74B900(Admin).
    fn ordering_284(&self) -> Option<u32>;
    fn local_8(&self) -> Option<u32>;
    fn admin_is_host_2c4(&self) -> bool;
    fn player_slot(&self, id: u32) -> Option<usize>;
    /// 0x76B920: the first player whose +24 equals the kingdom.
    fn kingdom_player_76b920(&self, kingdom: &[u16]) -> Option<usize>;
    fn player_id(&self, slot: usize) -> u32;
    fn player_link(&self, slot: usize) -> PeerLink;
    fn player_ai(&self, slot: usize) -> bool;
    fn player_kingdom(&self, slot: usize) -> Vec<u16>;
    /// `++Session+E8`.
    fn next_player_id_e8(&mut self) -> Result<u32>;
    /// 0x773710(id, link, ai, implicit) with 0x770280; the new slot.
    fn create_player_773710(&mut self, id: u32, link: PeerLink, ai: bool, implicit: bool, effects: &mut dyn FnMut(u32, bool, bool) -> Result<()>) -> Result<usize>;
    /// 0x7664B0 (+14, +2C), 0x7664F0 (+8, +2C), 0x766560, 0x7664D0 (+24, +2C).
    fn set_text_14(&mut self, slot: usize, text: &[u16]) -> Result<()>;
    fn store_name_7664f0(&mut self, slot: usize, name: &[u16]) -> Result<()>;
    fn unique_name_766560(&mut self, slot: usize, name: &[u16]) -> Result<()>;
    fn set_kingdom_7664d0(&mut self, slot: usize, kingdom: &[u16]) -> Result<()>;
    /// player +38 (not located), +40 team, +3C position.
    fn set_position(&mut self, slot: usize, flag_38: bool, team: u32, position: u32) -> Result<()>;
    fn set_text_30(&mut self, slot: usize, nation: &[u16]) -> Result<()>;
    fn set_text_34(&mut self, slot: usize, faction: &[u16]) -> Result<()>;
    /// 0x77E1F0 on Session+0x14.
    fn position_77e1f0(&self, kingdom: &[u16]) -> Result<Option<(u32, u32)>>;
    /// Session+0xC8.
    fn changed_c8(&mut self);
    /// 0x50DC00 on the current Creator (76C700) and on Session+68's header.
    fn row_index(&self, kingdom: &[u16]) -> Result<Option<usize>>;
    fn row(&self, index: usize) -> Result<Row>;
    fn base_row(&self, kingdom: &[u16]) -> Result<Option<Row>>;
    fn store_handicap(&mut self, index: usize, ids: &[u16], mark: bool) -> Result<()>;
    fn store_nation(&mut self, index: usize, ids: &[u16]) -> Result<()>;
    fn store_faction(&mut self, index: usize, ids: &[u16], mark: bool) -> Result<()>;
    fn mark_modified(&mut self, index: usize) -> Result<()>;
    /// 0x48D500 on the named registry ("Nation", "Faction").
    fn find_definition(&self, registry: &str, ids: &[u16]) -> Result<Option<Vec<u16>>>;
    /// Nation+0x48/+0x4C.
    fn nation_factions(&self, nation: &[u16]) -> Result<Vec<Vec<u16>>>;
}

fn prefix(s: &[u16]) -> &[u16] {
    &s[..s.iter().position(|&u| u == 0).unwrap_or(s.len())]
}
fn same(a: &[u16], b: &[u16]) -> bool {
    prefix(a) == prefix(b)
}

fn message(h: &impl Host, key: &str, effects: &mut impl Effects) -> Result<()> {
    // 0x76DA87..0x76DA9A: Admin+8 == Admin+0x284 (both may be NULL)
    effects.session_message_76da60(key, h.local_8() == h.ordering_284())
}

/// One staging order (its decree process).
pub fn process(h: &mut impl Host, request: &StagingRequest, effects: &mut impl Effects) -> Result<()> {
    match request {
        StagingRequest::CreateExplicitAi { kingdom, profile, name } => create_explicit_ai_754f50(h, kingdom, profile, name, effects),
        StagingRequest::SetKingdomNation { kingdom, nation } => set_kingdom_nation_777d90(h, kingdom, nation.as_deref(), effects),
        StagingRequest::SetKingdomFaction { kingdom, faction } => set_kingdom_faction_777700(h, kingdom, faction.as_deref(), effects),
        StagingRequest::SetAiHandicap { kingdom, handicap } => set_ai_handicap_771880(h, kingdom, handicap.as_deref(), effects),
    }
}

/// 0x76C560(kingdom).
fn may_change_76c560(h: &impl Host, kingdom: &[u16]) -> Result<bool> {
    let Some(ordering) = h.ordering_284() else { return Ok(false) };
    let ord = h.player_slot(ordering).ok_or_else(|| missing("76C560 Admin+284 player allocation is absent"))?;
    let Some(owner) = h.kingdom_player_76b920(kingdom) else { return Ok(false) };
    if h.local_8() == Some(ordering) && h.admin_is_host_2c4() {
        return Ok(true);
    }
    if h.player_ai(owner) && h.player_link(owner) == h.player_link(ord) {
        return Ok(true);
    }
    Ok(owner == ord)
}

/// AdminNode 0x754F50.
pub fn create_explicit_ai_754f50(h: &mut impl Host, kingdom: &[u16], profile: &[u16], name: &[u16], effects: &mut impl Effects) -> Result<()> {
    if h.countdown_118() > 0 {
        return message(h, COUNTING_DOWN_KEY, effects);
    }
    let link = match h.ordering_284() {
        None => {
            effects.ordering_player_null()?;
            PeerLink::Null
        }
        Some(id) => {
            let slot = h.player_slot(id).ok_or_else(|| missing("754F50 Admin+284 player allocation is absent"))?;
            h.player_link(slot)
        }
    };
    let id = h.next_player_id_e8()?;
    let slot = h.create_player_773710(id, link, true, false, &mut |id, ai, implicit| effects.creating_player(id, ai, implicit))?;
    h.set_text_14(slot, profile)?;
    if prefix(profile).is_empty() {
        h.store_name_7664f0(slot, name)?;
    } else {
        h.unique_name_766560(slot, name)?;
    }
    set_player_kingdom_777f00(h, slot, kingdom, true, effects)?;
    Ok(())
}

/// KKC_Session::SetPlayerKingdom 0x777F00(player, kingdom, notify).
pub fn set_player_kingdom_777f00(h: &mut impl Host, slot: usize, kingdom: &[u16], notify: bool, effects: &mut impl Effects) -> Result<bool> {
    let ai = h.player_ai(slot);
    let target = h.row_index(kingdom)?;
    if let Some(index) = target {
        if h.row(index)?.playable_28 {
            if !ai {
                h.store_handicap(index, &[], false)?;
            }
        } else if !ai {
            if notify {
                message(h, PLAYER_KINGDOM_FAILED_KEY, effects)?;
            }
            return Ok(false);
        }
    }
    let old_kingdom = h.player_kingdom(slot);
    let saved = match h.row_index(&old_kingdom)? {
        Some(i) if ai => h.row(i)?.handicap_24,
        _ => Vec::new(),
    };
    if !prefix(&old_kingdom).is_empty() {
        return Err(Error::Unsupported("777F00 reassignment of a player that already owns a kingdom (777CD0/777610 release)"));
    }
    h.set_kingdom_7664d0(slot, kingdom)?;
    let located = h.position_77e1f0(kingdom)?;
    let (team, position) = located.unwrap_or((u32::MAX, u32::MAX));
    h.set_position(slot, located.is_none(), team, position)?;
    if let Some(index) = target {
        h.store_handicap(index, &saved, false)?;
    }
    h.changed_c8();
    Ok(true)
}

/// Session 0x771880 SetAIHandicap(kingdom, handicap).
pub fn set_ai_handicap_771880(h: &mut impl Host, kingdom: &[u16], handicap: Option<&[u16]>, effects: &mut impl Effects) -> Result<()> {
    if h.countdown_118() > 0 {
        return message(h, COUNTING_DOWN_KEY, effects);
    }
    if !may_change_76c560(h, kingdom)? {
        return message(h, ORDER_FAILED_KEY, effects);
    }
    let index = h.row_index(kingdom)?.ok_or_else(|| missing("771962 a NULL current kingdom record (native NULL dereference)"))?;
    h.store_handicap(index, handicap.unwrap_or(&[]), true)
}

/// 0x777610(kingdom, faction IDS).
fn check_faction_777610(h: &mut impl Host, kingdom: &[u16], faction: &[u16], effects: &mut impl Effects) -> Result<()> {
    effects.check_faction(kingdom, faction)?;
    if prefix(kingdom).is_empty() {
        return Ok(());
    }
    let Some(base) = h.base_row(kingdom)? else { return Ok(()) };
    let Some(index) = h.row_index(kingdom)? else { return Ok(()) };
    let nation = h.row(index)?.nation_18;
    if prefix(&nation).is_empty() {
        return h.store_faction(index, &[], false);
    }
    if !prefix(&base.faction_1c).is_empty() {
        return Ok(());
    }
    let Some(found) = h.find_definition("Faction", faction)? else {
        return h.store_faction(index, &[], true);
    };
    if h.nation_factions(&nation)?.iter().any(|f| same(f, &found)) {
        h.store_faction(index, &found, true)
    } else {
        h.store_faction(index, &base.faction_1c, false)
    }
}

/// 0x777CD0(kingdom, nation IDS).
fn check_nation_777cd0(h: &mut impl Host, kingdom: &[u16], nation: &[u16], effects: &mut impl Effects) -> Result<()> {
    effects.check_nation(kingdom, nation)?;
    if prefix(kingdom).is_empty() {
        return Ok(());
    }
    let Some(base) = h.base_row(kingdom)? else { return Ok(()) };
    let Some(index) = h.row_index(kingdom)? else { return Ok(()) };
    if !prefix(&base.nation_18).is_empty() {
        return Ok(());
    }
    let found = h.find_definition("Nation", nation)?.unwrap_or_default();
    h.store_nation(index, &found)?;
    let faction = h.row(index)?.faction_1c;
    check_faction_777610(h, kingdom, &faction, effects)?;
    h.mark_modified(index)
}

/// Session 0x777D90 SetKingdomNation(kingdom, nation).
pub fn set_kingdom_nation_777d90(h: &mut impl Host, kingdom: &[u16], nation: Option<&[u16]>, effects: &mut impl Effects) -> Result<()> {
    if h.countdown_118() > 0 {
        return message(h, COUNTING_DOWN_KEY, effects);
    }
    if !may_change_76c560(h, kingdom)? {
        return message(h, NATION_FAILED_KEY, effects);
    }
    check_nation_777cd0(h, kingdom, nation.unwrap_or(&[]), effects)?;
    if let Some(p) = h.kingdom_player_76b920(kingdom) {
        let current = match h.row_index(kingdom)? {
            Some(i) => h.row(i)?.nation_18,
            None => Vec::new(),
        };
        h.set_text_30(p, &current)?;
    }
    Ok(())
}

/// Session 0x777700 SetKingdomFaction(kingdom, faction).
pub fn set_kingdom_faction_777700(h: &mut impl Host, kingdom: &[u16], faction: Option<&[u16]>, effects: &mut impl Effects) -> Result<()> {
    if h.countdown_118() > 0 {
        return message(h, COUNTING_DOWN_KEY, effects);
    }
    if !may_change_76c560(h, kingdom)? {
        return message(h, FACTION_FAILED_KEY, effects);
    }
    check_faction_777610(h, kingdom, faction.unwrap_or(&[]), effects)?;
    if let Some(p) = h.kingdom_player_76b920(kingdom) {
        let current = match h.row_index(kingdom)? {
            Some(i) => h.row(i)?.faction_1c,
            None => Vec::new(),
        };
        h.set_text_34(p, &current)?;
    }
    Ok(())
}

// ---------------------------------------------------------------- live Host

struct Live<'a, 'c, 's> {
    owner: &'a mut LocalBootstrap,
    catalog: &'a aa_runtime::catalog::Catalog<'c>,
    bank: &'a aa_runtime::definitions::Bank,
    manager: &'a aa_runtime::game_spy_manager_gate::State,
    text: &'s Localization,
}

struct Names<'a> {
    owner: &'a mut LocalBootstrap,
    text: &'a Localization,
}
impl crate::lobby::player_name::World for Names<'_> {
    type Player = usize;
    fn first_named(&mut self, name: &[u16]) -> Result<Option<usize>> {
        Ok(self.owner.session.players.iter().position(|p| crate::session::wcs_eq(&p.core.name, name)))
    }
    fn duplicate_name(&mut self, name: &[u16], number: u32) -> Result<Vec<u16>> {
        let key: Vec<u16> = crate::lobby::DUPLICATE_NAME_KEY.encode_utf16().collect();
        let format = self.text.display(&key).map_err(|e| missing(e.to_string()))?;
        crate::lobby::player_name::format(&format, name, number)
    }
    fn store_name_7664f0(&mut self, player: &usize, name: Vec<u16>) -> Result<()> {
        let p = self.owner.session.players.get_mut(*player).ok_or_else(|| missing("766560 actual target player is absent"))?;
        p.core.name = name;
        p.core.flag_2c = true;
        Ok(())
    }
}

impl Live<'_, '_, '_> {
    fn creator(&self) -> Result<&aa_runtime::resource_creator::Creator> {
        self.owner.selected_load_creator()
    }
    fn creator_mut(&mut self) -> Result<&mut aa_runtime::resource_creator::Creator> {
        Ok(&mut self.owner.load_selection.as_mut().ok_or_else(|| missing("staging current Creator is unbound"))?.creator)
    }
    fn core(&self, slot: usize) -> &PlayerFields {
        &self.owner.session.players[slot].core
    }
    fn core_mut(&mut self, slot: usize) -> Result<&mut PlayerFields> {
        Ok(&mut self.owner.session.players.get_mut(slot).ok_or_else(|| missing("staging player slot is absent"))?.core)
    }
}

fn row_of(k: &aa_mapworld::header::KingdomParameter) -> Row {
    Row { playable_28: k.byte_28 != 0, nation_18: k.nation.clone(), faction_1c: k.faction.clone(), handicap_24: k.handicap.clone() }
}

impl Host for Live<'_, '_, '_> {
    fn countdown_118(&self) -> i32 {
        self.owner.session.launch_countdown
    }
    fn ordering_284(&self) -> Option<u32> {
        self.owner.ordering_player
    }
    fn local_8(&self) -> Option<u32> {
        self.owner.local_player
    }
    fn admin_is_host_2c4(&self) -> bool {
        self.owner.host_node.as_ref().is_some_and(|n| n.server_is_self())
    }
    fn player_slot(&self, id: u32) -> Option<usize> {
        self.owner.session.players.iter().position(|p| p.core.id == id)
    }
    fn kingdom_player_76b920(&self, kingdom: &[u16]) -> Option<usize> {
        self.owner.session.players.iter().position(|p| crate::session::wcs_eq(&p.core.kingdom, kingdom))
    }
    fn player_id(&self, slot: usize) -> u32 {
        self.core(slot).id
    }
    fn player_link(&self, slot: usize) -> PeerLink {
        self.core(slot).link
    }
    fn player_ai(&self, slot: usize) -> bool {
        self.core(slot).ai
    }
    fn player_kingdom(&self, slot: usize) -> Vec<u16> {
        self.core(slot).kingdom.clone()
    }
    fn next_player_id_e8(&mut self) -> Result<u32> {
        let id = self.owner.session.last_player_id.ok_or_else(|| missing("754F50 SessionE8 has no Reset producer"))?.wrapping_add(1);
        self.owner.session.last_player_id = Some(id);
        Ok(id)
    }
    fn create_player_773710(&mut self, id: u32, link: PeerLink, ai: bool, implicit: bool, effects: &mut dyn FnMut(u32, bool, bool) -> Result<()>) -> Result<usize> {
        // The SAME 773710 publication order as the implicit and join creators:
        // 7661D0 fields and +50 VWorld, roster append, +C4/+C8, then 770280.
        let core = PlayerFields::from_native_fields(id, link, ai, implicit, self.owner.node_host);
        let view = construct_view(None, self.text)?;
        let slot = self.owner.session.players.len();
        self.owner.session.players.push(OwnedPlayer { allocation_lifetime: Arc::new(()), core, vector_44: Vec::new(), view: Some(view) });
        self.owner.session.created = Some(id);
        self.owner.session.changed[0] = true;
        effects(id, ai, implicit)?;
        self.owner.update_rated_770280(self.catalog, self.bank, self.manager)?;
        Ok(slot)
    }
    fn set_text_14(&mut self, slot: usize, text: &[u16]) -> Result<()> {
        let p = self.core_mut(slot)?;
        p.text_14 = text.to_vec();
        p.flag_2c = true;
        Ok(())
    }
    fn store_name_7664f0(&mut self, slot: usize, name: &[u16]) -> Result<()> {
        let p = self.core_mut(slot)?;
        p.name = name.to_vec();
        p.flag_2c = true;
        Ok(())
    }
    fn unique_name_766560(&mut self, slot: usize, name: &[u16]) -> Result<()> {
        crate::lobby::player_name::set(&mut Names { owner: self.owner, text: self.text }, &slot, name)
    }
    fn set_kingdom_7664d0(&mut self, slot: usize, kingdom: &[u16]) -> Result<()> {
        let p = self.core_mut(slot)?;
        p.kingdom = kingdom.to_vec();
        p.flag_2c = true;
        Ok(())
    }
    fn set_position(&mut self, slot: usize, flag_38: bool, team: u32, position: u32) -> Result<()> {
        let p = self.core_mut(slot)?;
        p.flag_38 = flag_38;
        p.field_40 = team;
        p.field_3c = position;
        Ok(())
    }
    fn set_text_30(&mut self, slot: usize, nation: &[u16]) -> Result<()> {
        self.core_mut(slot)?.text_30 = nation.to_vec();
        Ok(())
    }
    fn set_text_34(&mut self, slot: usize, faction: &[u16]) -> Result<()> {
        self.core_mut(slot)?.text_34 = faction.to_vec();
        Ok(())
    }
    fn position_77e1f0(&self, kingdom: &[u16]) -> Result<Option<(u32, u32)>> {
        self.owner.assignment_data_14.position_77e1f0(kingdom)
    }
    fn changed_c8(&mut self) {
        self.owner.session.changed[0] = true;
    }
    fn row_index(&self, kingdom: &[u16]) -> Result<Option<usize>> {
        Ok(rows::kingdom_index(self.creator()?, kingdom))
    }
    fn row(&self, index: usize) -> Result<Row> {
        Ok(row_of(self.creator()?.parameters().kingdoms.get(index).ok_or_else(|| missing("staging current record is absent"))?))
    }
    fn base_row(&self, kingdom: &[u16]) -> Result<Option<Row>> {
        let current = self
            .owner
            .load_selection
            .as_ref()
            .and_then(|s| s.game_settings_68.as_ref())
            .ok_or_else(|| missing("777CD0 actual Session68 is unbound"))?;
        Ok(current.parameters().kingdoms.iter().find(|k| same(&k.id, kingdom)).map(row_of))
    }
    fn store_handicap(&mut self, index: usize, ids: &[u16], mark: bool) -> Result<()> {
        let (catalog, bank) = (self.catalog, self.bank);
        let creator = self.creator_mut()?;
        if mark {
            rows::store_handicap_and_mark(catalog, bank, creator, index, ids)
        } else {
            rows::store_handicap(catalog, bank, creator, index, ids)
        }
        .map_err(missing)
    }
    fn store_nation(&mut self, index: usize, ids: &[u16]) -> Result<()> {
        let (catalog, bank) = (self.catalog, self.bank);
        rows::store_nation(catalog, bank, self.creator_mut()?, index, ids).map_err(missing)
    }
    fn store_faction(&mut self, index: usize, ids: &[u16], mark: bool) -> Result<()> {
        let (catalog, bank) = (self.catalog, self.bank);
        rows::store_faction(catalog, bank, self.creator_mut()?, index, ids, mark).map_err(missing)
    }
    fn mark_modified(&mut self, index: usize) -> Result<()> {
        let (catalog, bank) = (self.catalog, self.bank);
        rows::mark_modified(catalog, bank, self.creator_mut()?, index).map_err(missing)
    }
    fn find_definition(&self, registry: &str, ids: &[u16]) -> Result<Option<Vec<u16>>> {
        let ids = prefix(ids);
        if ids.is_empty() {
            return Ok(None);
        }
        let text = String::from_utf16(ids).map_err(|_| missing("definition Find key is not UTF-16"))?;
        let manager = self.catalog.actors.native_data_registry(registry).ok_or_else(|| missing(format!("{registry} registry is absent")))?;
        let Some(found) = manager.find(&text) else { return Ok(None) };
        if !found.reloads.is_empty() {
            return Err(Error::Unsupported("48D500 Find of a reloaded definition"));
        }
        Ok(Some(found.ids.encode_utf16().collect()))
    }
    fn nation_factions(&self, nation: &[u16]) -> Result<Vec<Vec<u16>>> {
        let id = String::from_utf16(prefix(nation)).map_err(|_| missing("Nation IDS is not UTF-16"))?;
        let registry = self.catalog.actors.native_data_registry("Nation").ok_or_else(|| missing("Nation registry is absent"))?;
        let entry = registry.find(&id).ok_or_else(|| missing("current Nation definition is absent"))?;
        let precision = self.bank.combat_source().map_err(missing)?.source_precision();
        let node = self.catalog.actors.native_data_node(entry).map_err(|e| missing(e.to_string()))?;
        match node.field("factions", precision).map_err(|e| missing(e.to_string()))? {
            Some(aa_rules::native::TypedValue::String { value, .. }) => Ok(aa_rules::abilities::required_property_tokens(&value)
                .map_err(|e| missing(e.to_string()))?
                .into_iter()
                .map(|t| t.encode_utf16().collect())
                .collect()),
            _ => Err(missing("48C8D5 Nation factions requires its typed string")),
        }
    }
}

impl LocalBootstrap {
    /// One staging order on this Host, after 7791F0 and before 77A950.
    pub fn process_staging_request(
        &mut self,
        catalog: &aa_runtime::catalog::Catalog<'_>,
        bank: &aa_runtime::definitions::Bank,
        manager: &aa_runtime::game_spy_manager_gate::State,
        text: &Localization,
        request: &StagingRequest,
        effects: &mut impl Effects,
    ) -> Result<()> {
        if self.host_input_stage()? != launch::InputStage::PlayersRefreshed {
            return Err(missing("staging orders run between the actual 7791F0 refresh and 77A950"));
        }
        if self.session.launch_mode != 2 {
            return Err(Error::Unsupported("staging order on a non-LOAD_MAP current Creator (76C700)"));
        }
        process(&mut Live { owner: self, catalog, bank, manager, text }, request, effects)
    }
}

// ---------------------------------------------------------------- laboratory

/// A laboratory Session for the oracle replay (`native-explicit-ai.json`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Laboratory {
    pub countdown_118: i32,
    pub ordering_284: Option<u32>,
    pub local_8: Option<u32>,
    pub last_player_id_e8: u32,
    pub created_c4: Option<u32>,
    pub changed_c8: bool,
    pub players: Vec<LabPlayer>,
    /// (id, row, modified68) of the current Creator.
    pub rows: Vec<(Vec<u16>, Row, bool)>,
    /// (id, row) of Session+68's header.
    pub base: Vec<(Vec<u16>, Row)>,
    /// Registry names in index order.
    pub nations: Vec<Vec<u16>>,
    pub factions: Vec<Vec<u16>>,
    /// Nation IDS -> its factions.
    pub nation_factions: Vec<(Vec<u16>, Vec<Vec<u16>>)>,
    /// `networking_duplicate_name_formate|sd` text.
    pub duplicate_format: Vec<u16>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LabPlayer {
    pub link: Option<PeerLink>,
    pub id: u32,
    pub name: Vec<u16>,
    pub ai: bool,
    pub implicit: bool,
    pub text_14: Vec<u16>,
    pub kingdom: Vec<u16>,
    pub flag_2c: bool,
    pub flag_38: bool,
    pub field_3c: u32,
    pub field_40: u32,
    pub text_30: Vec<u16>,
    pub text_34: Vec<u16>,
}
impl Laboratory {
    fn link(&self, slot: usize) -> PeerLink {
        self.players[slot].link.unwrap_or(PeerLink::Null)
    }
}
impl Host for Laboratory {
    fn countdown_118(&self) -> i32 {
        self.countdown_118
    }
    fn ordering_284(&self) -> Option<u32> {
        self.ordering_284
    }
    fn local_8(&self) -> Option<u32> {
        self.local_8
    }
    fn admin_is_host_2c4(&self) -> bool {
        true
    }
    fn player_slot(&self, id: u32) -> Option<usize> {
        self.players.iter().position(|p| p.id == id)
    }
    fn kingdom_player_76b920(&self, kingdom: &[u16]) -> Option<usize> {
        self.players.iter().position(|p| crate::session::wcs_eq(&p.kingdom, kingdom))
    }
    fn player_id(&self, slot: usize) -> u32 {
        self.players[slot].id
    }
    fn player_link(&self, slot: usize) -> PeerLink {
        self.link(slot)
    }
    fn player_ai(&self, slot: usize) -> bool {
        self.players[slot].ai
    }
    fn player_kingdom(&self, slot: usize) -> Vec<u16> {
        self.players[slot].kingdom.clone()
    }
    fn next_player_id_e8(&mut self) -> Result<u32> {
        self.last_player_id_e8 = self.last_player_id_e8.wrapping_add(1);
        Ok(self.last_player_id_e8)
    }
    fn create_player_773710(&mut self, id: u32, link: PeerLink, ai: bool, implicit: bool, effects: &mut dyn FnMut(u32, bool, bool) -> Result<()>) -> Result<usize> {
        self.players.push(LabPlayer {
            link: Some(link),
            id,
            ai,
            implicit,
            flag_2c: true,
            flag_38: true,
            field_3c: u32::MAX,
            field_40: u32::MAX,
            ..LabPlayer::default()
        });
        self.created_c4 = Some(id);
        self.changed_c8 = true;
        effects(id, ai, implicit)?;
        Ok(self.players.len() - 1)
    }
    fn set_text_14(&mut self, slot: usize, text: &[u16]) -> Result<()> {
        self.players[slot].text_14 = text.to_vec();
        self.players[slot].flag_2c = true;
        Ok(())
    }
    fn store_name_7664f0(&mut self, slot: usize, name: &[u16]) -> Result<()> {
        self.players[slot].name = name.to_vec();
        self.players[slot].flag_2c = true;
        Ok(())
    }
    fn unique_name_766560(&mut self, slot: usize, name: &[u16]) -> Result<()> {
        struct W<'a>(&'a mut Laboratory);
        impl crate::lobby::player_name::World for W<'_> {
            type Player = usize;
            fn first_named(&mut self, name: &[u16]) -> Result<Option<usize>> {
                Ok(self.0.players.iter().position(|p| crate::session::wcs_eq(&p.name, name)))
            }
            fn duplicate_name(&mut self, name: &[u16], number: u32) -> Result<Vec<u16>> {
                crate::lobby::player_name::format(&self.0.duplicate_format, name, number)
            }
            fn store_name_7664f0(&mut self, player: &usize, name: Vec<u16>) -> Result<()> {
                self.0.players[*player].name = name;
                self.0.players[*player].flag_2c = true;
                Ok(())
            }
        }
        crate::lobby::player_name::set(&mut W(self), &slot, name)
    }
    fn set_kingdom_7664d0(&mut self, slot: usize, kingdom: &[u16]) -> Result<()> {
        self.players[slot].kingdom = kingdom.to_vec();
        self.players[slot].flag_2c = true;
        Ok(())
    }
    fn set_position(&mut self, slot: usize, flag_38: bool, team: u32, position: u32) -> Result<()> {
        let p = &mut self.players[slot];
        p.flag_38 = flag_38;
        p.field_40 = team;
        p.field_3c = position;
        Ok(())
    }
    fn set_text_30(&mut self, slot: usize, nation: &[u16]) -> Result<()> {
        self.players[slot].text_30 = nation.to_vec();
        Ok(())
    }
    fn set_text_34(&mut self, slot: usize, faction: &[u16]) -> Result<()> {
        self.players[slot].text_34 = faction.to_vec();
        Ok(())
    }
    fn position_77e1f0(&self, kingdom: &[u16]) -> Result<Option<(u32, u32)>> {
        // constructor-empty Session+14 lists: an empty name is (-1, -1)
        Ok(prefix(kingdom).is_empty().then_some((u32::MAX, u32::MAX)))
    }
    fn changed_c8(&mut self) {
        self.changed_c8 = true;
    }
    fn row_index(&self, kingdom: &[u16]) -> Result<Option<usize>> {
        Ok(self.rows.iter().position(|(id, _, _)| same(id, kingdom)))
    }
    fn row(&self, index: usize) -> Result<Row> {
        Ok(self.rows[index].1.clone())
    }
    fn base_row(&self, kingdom: &[u16]) -> Result<Option<Row>> {
        Ok(self.base.iter().find(|(id, _)| same(id, kingdom)).map(|(_, r)| r.clone()))
    }
    fn store_handicap(&mut self, index: usize, ids: &[u16], mark: bool) -> Result<()> {
        self.rows[index].1.handicap_24 = ids.to_vec();
        if mark {
            self.rows[index].2 = true;
        }
        Ok(())
    }
    fn store_nation(&mut self, index: usize, ids: &[u16]) -> Result<()> {
        self.rows[index].1.nation_18 = ids.to_vec();
        Ok(())
    }
    fn store_faction(&mut self, index: usize, ids: &[u16], mark: bool) -> Result<()> {
        self.rows[index].1.faction_1c = ids.to_vec();
        if mark {
            self.rows[index].2 = true;
        }
        Ok(())
    }
    fn mark_modified(&mut self, index: usize) -> Result<()> {
        self.rows[index].2 = true;
        Ok(())
    }
    fn find_definition(&self, registry: &str, ids: &[u16]) -> Result<Option<Vec<u16>>> {
        if prefix(ids).is_empty() {
            return Ok(None);
        }
        let list = match registry {
            "Nation" => &self.nations,
            "Faction" => &self.factions,
            _ => return Err(missing("laboratory registry")),
        };
        Ok(list.iter().find(|n| same(n, ids)).cloned())
    }
    fn nation_factions(&self, nation: &[u16]) -> Result<Vec<Vec<u16>>> {
        Ok(self.nation_factions.iter().find(|(n, _)| same(n, nation)).map(|(_, f)| f.clone()).unwrap_or_default())
    }
}

#[cfg(test)]
#[path = "local_staging_requests_tests.rs"]
mod tests;
