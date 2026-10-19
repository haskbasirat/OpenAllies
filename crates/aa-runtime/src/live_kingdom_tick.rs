//! GKingdom economy Updatable on the live retained World.
//!
//! The Kingdom's Updatable subobject is Kingdom+0x180 (vtable A29B50, Init
//! 4E5DF0 registers it on queue0 through 50AC00). Periodic 50B330 calls its
//! virtual +14, 4E9C40, with the elapsed binary32 seconds. The complete
//! arithmetic body is aa_economy::Economy::tick (oracle-tested, research
//! phase8-economy); this module binds it to the SAME owners:
//! * Kingdom+1A0/+1AC/+1B8/+1C4 resource vectors (SpecialShell vectors),
//! * Kingdom+190/+1F0/+1F4/+334 tick words and the +214/+220 singles,
//! * Kingdom+230 Commander (moved out of the shell for the call only: no
//!   other reader runs between 4E9C40's entry and its return),
//! * Kingdom+32C statistics and the +338 snapshot list (KingdomEconomyLoad),
//! * SVars+180 ExperienceFactionGainPeriodic, +1B8 EconomyStockpileResourceMax.
//!
//! Its world leaves are ported here in native order: 4E9960 (team defeat
//! check, with 4E64C0/4F0460) and 4E9A40 (city victory check, with 4F1B30 and
//! 4E3AB0), and 58F530/45F240 snapshots. Reached-only leaves that remain
//! unported return explicit errors at their native call: 42D4B0 game events
//! (upkeep shortage/repaired, city victory) and 4E9720 SetMissionResult.
use crate::{
    actor_world::{ActorStore, ObjectKey, Source, SpecialSource},
    catalog::Catalog,
    restoration::{KingdomEconomyLoad, StatisticsSnapshotEntry},
    stored_actor_state_effects::CurrentSettings,
};
use aa_actors::ActorId;
use aa_economy::{
    EconomyError, F32Bits, State, X87Precision,
    statistics::Statistics,
    tick::{CommanderParts, OwnerStatistics, TickInput, TickParts, TickWorld},
};
use aa_mapworld::special_state::SpecialShell;
use aa_numeric::{Binary, Precision};
use aa_simulation::NativeClock;
use aa_world::queues::{Delivery, Error as QueueError};
use std::{cmp::Ordering, collections::BTreeMap};

type Result<T> = std::result::Result<T, String>;

/// Same live World owners, lent for one queue0 delivery.
pub(crate) struct Context<'a, 'source> {
    pub catalog: &'a Catalog<'source>,
    pub actors: &'a mut ActorStore,
    pub economy: &'a mut BTreeMap<u32, KingdomEconomyLoad>,
    pub variables: &'a crate::load_view_variables::State,
    pub registration: &'a crate::live_registration::State,
    /// World+154/+158 Kingdom vector (null entries kept).
    pub majors: &'a [Option<ActorId>],
    pub mission: &'a crate::mission_fields::State,
    /// [GWorld+28]+A8 loan (4F0460).
    pub current_settings: CurrentSettings<'a>,
    /// Bank combat-source PC: the resource definitions are source projections.
    pub source_precision: Precision,
    pub clock: &'a NativeClock,
    /// World+D8 (4F65B0 phase).
    pub phase: u32,
    /// Simulation-thread x87 PC of this World update.
    pub precision: Precision,
}

fn fail(error: impl std::fmt::Display) -> QueueError {
    QueueError::Callback(format!("live Kingdom 4E9C40: {error}"))
}
fn economy_error(error: impl std::fmt::Display) -> EconomyError {
    EconomyError::World(error.to_string())
}
fn x87(p: Precision) -> X87Precision {
    match p {
        Precision::Single24 => X87Precision::Single24,
        Precision::Double53 => X87Precision::Double53,
        Precision::Extended64 => X87Precision::Extended64,
    }
}

fn kingdom_shell(actors: &ActorStore, id: ActorId) -> Result<&SpecialShell> {
    let key = actors
        .key(id)
        .ok_or_else(|| format!("Kingdom {:#x} has no live GTID", id.raw()))?;
    match &actors.entity(key).map_err(|e| e.to_string())?.source {
        Source::Special(s) if matches!(s.source, SpecialSource::Kingdom(_)) => s
            .derived
            .as_ref()
            .ok_or_else(|| "Kingdom derived state is absent".into()),
        _ => Err(format!("{:#x} is not an actual Kingdom allocation", id.raw())),
    }
}
fn team_shell(actors: &ActorStore, id: ActorId) -> Result<&SpecialShell> {
    let key = actors
        .key(id)
        .ok_or_else(|| format!("Team {:#x} has no live GTID", id.raw()))?;
    match &actors.entity(key).map_err(|e| e.to_string())?.source {
        Source::Special(s) if matches!(s.source, SpecialSource::Team(_)) => s
            .derived
            .as_ref()
            .ok_or_else(|| "Team derived state is absent".into()),
        _ => Err(format!("{:#x} is not an actual Team allocation", id.raw())),
    }
}
fn kingdom_shell_mut(actors: &mut ActorStore, key: ObjectKey) -> Result<&mut SpecialShell> {
    match &mut actors.entity_mut(key).map_err(|e| e.to_string())?.source {
        Source::Special(s) if matches!(s.source, SpecialSource::Kingdom(_)) => s
            .derived
            .as_mut()
            .ok_or_else(|| "Kingdom derived state is absent".into()),
        _ => Err("scheduled Kingdom target is not an actual Kingdom allocation".into()),
    }
}
fn word(shell: &SpecialShell, offset: u32) -> Result<u32> {
    shell
        .words
        .get(&offset)
        .copied()
        .ok_or_else(|| format!("Kingdom+{offset:X} is uninitialized"))
}
fn byte(shell: &SpecialShell, offset: u32) -> Result<u8> {
    if let Some(value) = shell.bytes.get(&offset) {
        return Ok(*value);
    }
    //Kingdom+238 is the low byte of the constructor DWORD 0x01010001.
    let aligned = offset & !3;
    shell
        .words
        .get(&aligned)
        .map(|w| (w >> ((offset - aligned) * 8)) as u8)
        .ok_or_else(|| format!("Kingdom+{offset:X} byte is uninitialized"))
}
/// Kingdom+1EC: the actual Team allocation or the constructor null.
fn team(shell: &SpecialShell) -> Result<Option<ActorId>> {
    match shell.identity_references.get(&0x1ec) {
        Some(value) => Ok(*value),
        None if shell.words.get(&0x1ec) == Some(&0) => Ok(None),
        _ => Err("Kingdom+1EC actual Team reference is unbound".into()),
    }
}
fn vector_bits(shell: &SpecialShell, offset: u32, count: usize) -> Result<Vec<F32Bits>> {
    let vector = shell
        .vectors
        .get(&offset)
        .ok_or_else(|| format!("Kingdom vector {offset:X} has moved or is missing"))?;
    if vector.values.len() != count {
        return Err(format!(
            "Kingdom vector {offset:X} length {} differs from the {count}-resource registry",
            vector.values.len()
        ));
    }
    Ok(vector.values.iter().copied().map(F32Bits).collect())
}
fn store_vector(shell: &mut SpecialShell, offset: u32, values: &[F32Bits]) -> Result<()> {
    let vector = shell
        .vectors
        .get_mut(&offset)
        .ok_or_else(|| format!("Kingdom vector {offset:X} disappeared during 4E9C40"))?;
    if vector.values.len() != values.len() {
        return Err(format!("Kingdom vector {offset:X} changed length during 4E9C40"));
    }
    for (cell, value) in vector.values.iter_mut().zip(values) {
        *cell = value.0;
    }
    Ok(())
}
/// Current SVar by registered name (405370 lookup on B39ABC).
fn svar<'a>(
    variables: &'a crate::load_view_variables::State,
    name: &str,
) -> Result<crate::load_view_variables::CurrentValue<'a>> {
    let ordinal = variables
        .registrations()
        .find(|r| r.name() == name)
        .map(|r| r.ordinal())
        .ok_or_else(|| format!("SVar {name} is not registered"))?;
    variables.current(ordinal as usize)
}
fn svar_fixed(variables: &crate::load_view_variables::State, name: &str) -> Result<u32> {
    match svar(variables, name)? {
        crate::load_view_variables::CurrentValue::Fixed(v) => Ok(*v),
        _ => Err(format!("SVar {name} is not the native float kind")),
    }
}
fn svar_bool(variables: &crate::load_view_variables::State, name: &str) -> Result<u8> {
    match svar(variables, name)? {
        crate::load_view_variables::CurrentValue::Bool(v) => Ok(*v),
        _ => Err(format!("SVar {name} is not the native Bool kind")),
    }
}

/// 4E9C40 world leaves over the same owners. The kingdom's Commander is held
/// by the call, so these reads never alias the mutable experience field.
struct Leaves<'a, 'b, 'source> {
    id: ActorId,
    actors: &'a ActorStore,
    catalog: &'a Catalog<'source>,
    variables: &'a crate::load_view_variables::State,
    registration: &'a crate::live_registration::State,
    majors: &'a [Option<ActorId>],
    mission: &'a crate::mission_fields::State,
    current_settings: CurrentSettings<'b>,
    phase: u32,
    precision: Precision,
    source_precision: Precision,
    /// Kingdom+338 list and +340 relation object of THIS kingdom.
    snapshots: &'a mut Vec<StatisticsSnapshotEntry>,
    relations_present: bool,
    provided: &'a [F32Bits],
    consumed: &'a [F32Bits],
}
impl Leaves<'_, '_, '_> {
    fn shell(&self) -> Result<&SpecialShell> {
        kingdom_shell(self.actors, self.id)
    }
    /// 4F0460: [World+28] non-null and its +A8 nonzero.
    fn world_28_active(&self) -> Result<bool> {
        let (binding, owner) = self
            .current_settings
            .ok_or("4F0460 actual current World+28 MapResource loan is absent")?;
        crate::live_placement::read_settings_mode(true, None, binding, Some(owner))
            .map(|mode| mode.is_some_and(|mode| mode != 0))
            .map_err(|e| e.to_string())
    }
    /// 4E64C0: true when this kingdom and (TeamVictory) its team mates hold no
    /// counted actors (+2F4). Iterates World+154 skipping null slots
    /// (4F0680/4F06B0); a missing TeamVictory makes the native read 'T'.
    fn team_has_no_counted_actors(&self) -> Result<bool> {
        let shell = self.shell()?;
        if (word(shell, 0x2f4)? as i32) > 0 {
            return Ok(false);
        }
        let team_victory = svar_bool(self.variables, "TeamVictory")?;
        let team = team(shell)?;
        let mut visited = false;
        let mut sum = 0i32;
        for other in self.majors.iter().flatten().copied() {
            visited = true;
            let other_shell = kingdom_shell(self.actors, other)?;
            if other == self.id
                || (team_victory != 0 && team.is_some() && self::team(other_shell)? == team)
            {
                sum = sum.wrapping_add(word(other_shell, 0x2f4)? as i32);
            }
        }
        Ok(!(visited && sum > 0))
    }
    /// 4E3AB0: signed count of the Kingdom+30C owned-city list.
    fn owned_cities(&self, id: ActorId) -> Result<i32> {
        let lists = kingdom_shell(self.actors, id)?
            .actor_lists
            .as_ref()
            .ok_or("4E3AB0 actual Kingdom+30C list owner is unbound")?;
        Ok(crate::kingdom_ownership_track::count(&lists.city_ownership))
    }
}
impl TickWorld for Leaves<'_, '_, '_> {
    fn gtid_live(&mut self, gtid: u32) -> std::result::Result<bool, EconomyError> {
        //Only populated recruit singles reach this; recruit_singles() refuses them.
        Err(economy_error(format!(
            "4E12D0 B364C0 generation lookup of {gtid:#x} requires bound recruit singles"
        )))
    }
    fn upkeep_shortage(&mut self, percent: i32) -> std::result::Result<(), EconomyError> {
        Err(economy_error(format!(
            "42D4B0 GameEventManager kind3 kingdom_upkeep_shortage({percent}%) for {:#x} is not bound",
            self.id.raw()
        )))
    }
    fn upkeep_repaired(&mut self) -> std::result::Result<(), EconomyError> {
        Err(economy_error(format!(
            "42D4B0 GameEventManager kind4 kingdom_upkeep_repaired for {:#x} is not bound",
            self.id.raw()
        )))
    }
    /// 4E9960.
    fn team_victory_check(&mut self) -> std::result::Result<(), EconomyError> {
        let run = || -> Result<Option<Vec<ActorId>>> {
            let shell = self.shell()?;
            if byte(shell, 0x238)? == 0 || !self.team_has_no_counted_actors()? {
                return Ok(None);
            }
            if word(shell, 0x19c)? != 0
                || self.world_28_active()?
                || self.mission.started48() != 1
                || self.phase == 6
            {
                return Ok(None);
            }
            let team_victory = svar_bool(self.variables, "TeamVictory")?;
            let team = team(shell)?;
            let mut defeated = Vec::new();
            for other in self.majors.iter().flatten().copied() {
                if other == self.id
                    || (team_victory != 0
                        && team.is_some()
                        && self::team(kingdom_shell(self.actors, other)?)? == team)
                {
                    defeated.push(other);
                }
            }
            Ok(Some(defeated))
        };
        match run().map_err(economy_error)? {
            None => Ok(()),
            Some(defeated) => match defeated.first() {
                None => Ok(()),
                Some(first) => Err(economy_error(format!(
                    "4E9960 reached 4E9720 SetMissionResult(2,1) for Kingdom {:#x}: 42D4B0/50D370/4F4DA0 mission-result services are not bound",
                    first.raw()
                ))),
            },
        }
    }
    /// 4E9A40.
    fn percentage_victory_check(&mut self) -> std::result::Result<(), EconomyError> {
        let run = || -> Result<bool> {
            let shell = self.shell()?;
            if word(shell, 0x19c)? != 0 || word(shell, 0x190)? == 1 {
                return Ok(false);
            }
            let ratio = self.variables.city_victory_ratio();
            if ratio <= 0 {
                return Ok(false);
            }
            //4F1B30 counts World+260; the typed list replaces node addresses.
            let available = self.registration.available_cities().len() as u32 as i32;
            if available <= 0 {
                return Ok(false);
            }
            let owned = match team(shell)? {
                None => self.owned_cities(self.id)?,
                Some(team) => {
                    let members = team_shell(self.actors, team)?
                        .team_members
                        .as_ref()
                        .ok_or("4E9A40 actual Team+44 member list is unbound")?;
                    let mut sum = 0i32;
                    for member in members.iter().copied().filter(|m| !m.is_null()) {
                        sum = sum.wrapping_add(self.owned_cities(member)?);
                    }
                    sum
                }
            };
            //fild owned; fidiv available; fmul 100.0f; fild ratio; fcompp.
            let percent = Binary::from_i32(owned)
                .div(&Binary::from_i32(available), self.precision)
                .and_then(|v| v.mul(&Binary::from_f32_bits(0x42c8_0000)?, self.precision))
                .map_err(|e| e.to_string())?;
            Ok(matches!(
                Binary::from_i32(ratio).compare(&percent),
                Ordering::Less | Ordering::Equal
            ))
        };
        if run().map_err(economy_error)? {
            return Err(economy_error(format!(
                "4E9A40 city victory for Kingdom {:#x} reached 42D4B0 event 0x39 and 4E9720(1,1): services are not bound",
                self.id.raw()
            )));
        }
        Ok(())
    }
    fn statistics_snapshot(&mut self) -> std::result::Result<(), EconomyError> {
        Err(economy_error(
            "58F530 snapshot requires the current Kingdom+32C statistics",
        ))
    }
    /// 4E37D0 -> 58F530 -> 45F240 (append after the current tail).
    fn statistics_snapshot_of(
        &mut self,
        statistics: &Statistics,
    ) -> std::result::Result<(), EconomyError> {
        let definitions = self
            .catalog
            .native_resource_economy(self.source_precision)
            .map_err(economy_error)?;
        let formulas = crate::world_start_world::bind_formulas(
            self.catalog.actors,
            definitions.catalog(),
            self.source_precision,
        )?;
        let count = i32::try_from(self.shell().map_err(economy_error)?.recruit_items.singles.len())
            .map_err(|_| economy_error("recruit count exceeds the native signed statistic"))?;
        let snapshot = aa_economy::statistics_live::snapshot(
            statistics,
            definitions.catalog(),
            self.provided,
            self.consumed,
            count,
            self.relations_present.then_some(self.id.raw()),
            &formulas,
            x87(self.precision),
        )?;
        self.snapshots
            .try_reserve(1)
            .map_err(|_| economy_error("45F240 snapshot node allocation failed"))?;
        self.snapshots.push(StatisticsSnapshotEntry::Live(snapshot));
        Ok(())
    }
}

/// One queue0 periodic delivery to Kingdom+180 (virtual +14 = 4E9C40).
pub(crate) fn run(key: ObjectKey, event: Delivery, cx: &mut Context<'_, '_>) -> aa_world::queues::Result<()> {
    let elapsed = event
        .elapsed_bits
        .ok_or_else(|| fail("Kingdom Updatable has no deferred +18/+1C body (943800 RET4 is not its table)"))?;
    let id = cx
        .actors
        .id(key)
        .ok_or_else(|| fail("scheduled Kingdom has no live GTID"))?;
    let definitions = cx
        .catalog
        .native_resource_economy(cx.source_precision)
        .map_err(fail)?;
    let count = definitions.resource_count();
    let shell = kingdom_shell_mut(cx.actors, key).map_err(fail)?;
    let state = State {
        raw_provided: vector_bits(shell, 0x1ac, count).map_err(fail)?,
        effective_provided: vector_bits(shell, 0x1a0, count).map_err(fail)?,
        consumed: vector_bits(shell, 0x1b8, count).map_err(fail)?,
        stockpiled: vector_bits(shell, 0x1c4, count).map_err(fail)?,
    };
    let mut tick_state = crate::kingdom_economy::tick_state(shell).map_err(fail)?;
    //Kingdom+214 singles: the typed recruit table owns them after Load. Only
    //the empty list (every shipped map) is bound; populated singles need the
    //4E6C20 item records with ActorData+2CC rates and GTIDs.
    if !shell.recruit_items.singles.is_empty() {
        return Err(fail("populated Kingdom+214 recruit singles require native item binding"));
    }
    let mut singles = crate::kingdom_economy::recruit_singles(shell).map_err(fail)?;
    //Kingdom+230: an actual Commander allocation, or the constructor null.
    if shell.commander.is_none() && shell.words.get(&0x230) != Some(&0) {
        return Err(fail("Kingdom+230 Commander owner is unbound"));
    }
    let mut commander = shell.commander.take();
    let provided = state.effective_provided.clone();
    let consumed = state.consumed.clone();
    let stockpiled_maximum = F32Bits(cx.variables.stockpile_maximum_bits());
    let rate = match svar_fixed(cx.variables, "ExperienceFactionGainPeriodic") {
        Ok(rate) => rate,
        Err(error) => {
            kingdom_shell_mut(cx.actors, key).map_err(fail)?.commander = commander;
            return Err(fail(error));
        }
    };
    let mut economy = match definitions.call_view(state, stockpiled_maximum, cx.precision) {
        Ok(economy) => economy,
        Err(error) => {
            kingdom_shell_mut(cx.actors, key).map_err(fail)?.commander = commander;
            return Err(fail(error));
        }
    };
    let Some(entry) = cx.economy.get_mut(&id.raw()) else {
        kingdom_shell_mut(cx.actors, key).map_err(fail)?.commander = commander;
        return Err(fail("Kingdom statistics owner (KingdomEconomyLoad) is absent"));
    };
    let KingdomEconomyLoad {
        statistics,
        snapshots,
        ..
    } = entry;
    let relations_present = statistics.relations.is_some();
    let result = {
        let commander_parts = match commander.as_mut() {
            None => Ok(None),
            Some(c) if c.kingdom == id.raw() => Ok(Some(CommanderParts {
                experience: &mut c.experience,
                owner_statistics: OwnerStatistics::Kingdom,
            })),
            Some(c) => Err(format!(
                "Commander owner {:#x} differs from its Kingdom {:#x}",
                c.kingdom,
                id.raw()
            )),
        };
        match commander_parts {
            Err(error) => Err(error),
            Ok(commander_parts) => {
                let mut leaves = Leaves {
                    id,
                    actors: &*cx.actors,
                    catalog: cx.catalog,
                    variables: cx.variables,
                    registration: cx.registration,
                    majors: cx.majors,
                    mission: cx.mission,
                    current_settings: cx.current_settings,
                    phase: cx.phase,
                    precision: cx.precision,
                    source_precision: cx.source_precision,
                    snapshots,
                    relations_present,
                    provided: &provided,
                    consumed: &consumed,
                };
                economy
                    .tick(
                        TickInput {
                            seconds: F32Bits(elapsed),
                            world_time: F32Bits(cx.clock.begin.0),
                            commander_experience_rate: F32Bits(rate),
                        },
                        TickParts {
                            state: &mut tick_state,
                            singles: &mut singles,
                            commander: commander_parts,
                            statistics: statistics.active.as_mut(),
                        },
                        &mut leaves,
                    )
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            }
        }
    };
    //Native stores are immediate: commit every value the call produced, even
    //when a later leaf errors, then return that error.
    let shell = kingdom_shell_mut(cx.actors, key).map_err(fail)?;
    shell.commander = commander;
    let committed = economy.state();
    store_vector(shell, 0x1ac, &committed.raw_provided).map_err(fail)?;
    store_vector(shell, 0x1a0, &committed.effective_provided).map_err(fail)?;
    store_vector(shell, 0x1b8, &committed.consumed).map_err(fail)?;
    store_vector(shell, 0x1c4, &committed.stockpiled).map_err(fail)?;
    crate::kingdom_economy::store_tick_state(shell, &tick_state);
    if !singles.items.is_empty() {
        return Err(fail("recruit singles changed shape during 4E9C40"));
    }
    if let Some(charging) = shell.words.get_mut(&0x220) {
        *charging = singles.charging as u32;
    }
    result.map_err(fail)
}
