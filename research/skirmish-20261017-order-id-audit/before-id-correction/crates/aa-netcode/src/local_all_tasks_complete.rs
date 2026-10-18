//!Actual775BF0 source/session effects after315. Clock/unfreeze and initial
//!World synchronization are required frontend siblings. No callback here can
//!construct a StartedMission or ActualActivatedSession.
use super::*;
use aa_orders::net::admin::RequestEmission;
use aa_orders::payload::Field;
use aa_runtime::load_tail_world::SessionRead;
use aa_runtime::{
    actor_world::{ActorStore, Source, SpecialSource},
    catalog::Catalog,
    game_spy_manager_gate, inactive_svar_dump,
    live_sim::Sim,
};
use std::cell::RefCell;

#[derive(Debug, PartialEq, Eq)]
pub enum Diagnostic {
    AllTasksSuccess,
    BeginPlaying,
    HumanOpposition(bool),
}
///Every method is a reached service. Implementations must borrow the actual
///Node clock/flags and same live World; there is no recording-only default.
pub trait Services {
    fn unfreeze_76dd90(
        &mut self,
        same_sim: &mut Sim,
        same_a: &RefCell<Synchronizer>,
        players: &PlayersBorrow<'_>,
    ) -> Result<()>;
    fn human_opposition(&mut self, players: &PlayersBorrow<'_>) -> Result<bool>;
    fn running_node_2c0(&mut self) -> Result<bool>;
    fn current_svar_sync_76d820(&mut self,inventory:&inactive_svar_dump::Inventory,same_a:&RefCell<Synchronizer>)->Result<()>;
    fn initial_world_sync_4f8ae0(
        &mut self,
        label: &str,
        same_a: &RefCell<Synchronizer>,
        same_s: &mut Synchronizer,
    ) -> Result<()>;
}
pub struct Context<'a, 'source, 'emit, S: Services> {
    pub catalog: &'a Catalog<'source>,
    pub sim: &'a mut Sim,
    ///Actual current frame World identity, paired with the SAME Sim
    ///publication. An ActorStore nonempty test cannot substitute for it.
    pub current_world: aa_runtime::world_revision::Witness,
    pub game_spy: &'a game_spy_manager_gate::State,
    pub inventory: &'a inactive_svar_dump::Inventory,
    pub sync_a: &'a RefCell<Synchronizer>,
    pub sync_s: &'a mut Synchronizer,
    pub emission: &'a mut RequestEmission<'emit>,
    pub notifications: &'a Notifications,
    pub services: &'a mut S,
    pub diagnostics: &'a mut Vec<Diagnostic>,
}
trait World {
    fn diagnostic(&mut self) -> Result<()>;
    fn notify(&mut self) -> Result<()>;
    fn state(&mut self) -> Result<u32>;
    fn unfreeze(&mut self) -> Result<()>;
    fn flag(&mut self, offset: u32) -> Result<bool>;
    fn store_flag(&mut self, offset: u32, value: bool) -> Result<()>;
    fn restart_recording(&mut self) -> Result<()>;
    fn begin_diagnostic(&mut self) -> Result<()>;
    fn human_opposition(&mut self) -> Result<bool>;
    fn opposition_diagnostic(&mut self, value: bool) -> Result<()>;
    fn update_game_spy(&mut self) -> Result<()>;
    fn flush_setup_ai(&mut self) -> Result<()>;
    fn synchronize_svars(&mut self) -> Result<()>;
    fn synchronize_world(&mut self) -> Result<()>;
    fn reset_s(&mut self) -> Result<()>;
}
fn run<W: World>(world: &mut W) -> Result<()> {
    world.diagnostic()?;
    world.notify()?;
    if matches!(world.state()?, 2 | 4 | 5) {
        world.unfreeze()?;
    }
    if world.flag(0x16b)? {
        world.restart_recording()?;
        world.store_flag(0x16b, false)?;
    }
    if world.flag(0x16a)? {
        world.store_flag(0x16a, false)?;
        world.begin_diagnostic()?;
        let opposition = world.human_opposition()?;
        world.store_flag(0x16d, opposition)?;
        world.store_flag(0x16e, false)?;
        world.opposition_diagnostic(opposition)?;
        world.update_game_spy()?;
        world.flush_setup_ai()?;
        world.synchronize_svars()?;
        world.synchronize_world()?;
        world.reset_s()?;
    }
    Ok(())
}
struct Current<'a, 'owner, 'ctx, 'source, 'emit, S: Services> {
    players: &'a mut PlayersBorrow<'owner>,
    context: &'a mut Context<'ctx, 'source, 'emit, S>,
}
impl<S: Services> World for Current<'_, '_, '_, '_, '_, S> {
    fn diagnostic(&mut self) -> Result<()> {
        self.context.diagnostics.push(Diagnostic::AllTasksSuccess);
        Ok(())
    }
    fn notify(&mut self) -> Result<()> {
        self.context
            .notifications
            .push(Notification::native_kind(9))
    }
    fn state(&mut self) -> Result<u32> {
        Ok(self.players.session.state)
    }
    fn unfreeze(&mut self) -> Result<()> {
        self.context
            .services
            .unfreeze_76dd90(self.context.sim, self.context.sync_a, self.players)
    }
    fn flag(&mut self, offset: u32) -> Result<bool> {
        self.players
            .session
            .flags_168_16e
            .get((offset - 0x168) as usize)
            .copied()
            .flatten()
            .ok_or_else(|| missing(format!("775BF0 current Session{offset:X} has no producer")))
    }
    fn store_flag(&mut self, offset: u32, value: bool) -> Result<()> {
        *self
            .players
            .session
            .flags_168_16e
            .get_mut((offset - 0x168) as usize)
            .ok_or_else(|| missing("775BF0 invalid owned Session flag"))? = Some(value);
        Ok(())
    }
    fn restart_recording(&mut self) -> Result<()> {
        Err(missing(
            "775BF0 positive16B requires actual76EE10 recording/restart owner",
        ))
    }
    fn begin_diagnostic(&mut self) -> Result<()> {
        self.context.diagnostics.push(Diagnostic::BeginPlaying);
        Ok(())
    }
    fn human_opposition(&mut self) -> Result<bool> {
        self.context.services.human_opposition(self.players)
    }
    fn opposition_diagnostic(&mut self, value: bool) -> Result<()> {
        self.context
            .diagnostics
            .push(Diagnostic::HumanOpposition(value));
        Ok(())
    }
    fn update_game_spy(&mut self) -> Result<()> {
        //770146 dereferences the actual nonnull manager before reading60.
        if self.context.game_spy.internet_enabled_bc()
            && matches!(self.players.session.launch_kind_60, 1 | 2)
        {
            return Err(missing(
                "770130 positive Internet manager/game type requires actual online statistics owner",
            ));
        }
        Ok(())
    }
    fn flush_setup_ai(&mut self) -> Result<()> {
        let SessionFields {
            setup_ai, players, ..
        } = &mut *self.players.session;
        flush_setup(
            setup_ai,
            &mut SetupEmission {
                players,
                local_player: self.players.local_player,
                services: self.context.services,
                emission: self.context.emission,
            },
        )
    }
    fn synchronize_svars(&mut self) -> Result<()> {
        self.context.services.current_svar_sync_76d820(self.context.inventory,self.context.sync_a)
    }
    fn synchronize_world(&mut self) -> Result<()> {
        self.context.services.initial_world_sync_4f8ae0(
            "g_gworld",
            self.context.sync_a,
            self.context.sync_s,
        )
    }
    fn reset_s(&mut self) -> Result<()> {
        self.context.sync_s.reset(0, 0);
        Ok(())
    }
}

trait SetupTransport {
    fn emit(&mut self, fields: &[Field]) -> Result<()>;
}
fn flush_setup(queue: &mut sai_setup::Queue, target: &mut impl SetupTransport) -> Result<()> {
    let mut index = 0;
    while index < queue.entries().len() {
        let entry = &queue.entries()[index];
        //Native773480 by-value CString captures before the request emitter.
        let fields = [
            Field::Text(entry.kingdom().to_vec()),
            Field::Text(entry.display().to_vec()),
            Field::Text(entry.profile().to_vec()),
            Field::Uint { width: 1, value: 0 },
        ];
        target.emit(&fields)?;
        index += 1;
    }
    queue.clear_count();
    Ok(())
}
struct SetupEmission<'a, 'emit, S: Services> {
    players: &'a [OwnedPlayer],
    local_player: &'a Option<u32>,
    services: &'a mut S,
    emission: &'a mut RequestEmission<'emit>,
}
impl<S: Services> SetupTransport for SetupEmission<'_, '_, S> {
    fn emit(&mut self, fields: &[Field]) -> Result<()> {
        let running = self.services.running_node_2c0()?;
        let player = if running {
            let id = self
                .local_player
                .ok_or_else(|| missing("76CA70 active Node8 is NULL"))?;
            self.players
                .iter()
                .find(|p| p.core.id == id)
                .ok_or_else(|| missing("76AC60 local player allocation is absent"))?
                .core
                .id
        } else {
            0
        };
        self.emission.emit_request(
            running,
            323,
            fields,
            player,
            &super::world_ready::success::TaskSchema,
        )?;
        Ok(())
    }
}
pub(super) trait OppositionFields {
    type Kingdom: Copy;
    type Team: Copy + PartialEq;
    fn count(&mut self) -> usize;
    fn ai(&mut self, index: usize) -> Result<bool>;
    fn kingdom(&mut self, index: usize) -> Result<Option<Self::Kingdom>>;
    fn result(&mut self, kingdom: Self::Kingdom) -> Result<u32>;
    fn team(&mut self, kingdom: Self::Kingdom) -> Result<Option<Self::Team>>;
}
pub(super) fn opposition<F: OppositionFields>(fields: &mut F) -> Result<bool> {
    let mut first = 0;
    while first < fields.count() {
        if !fields.ai(first)? && fields.kingdom(first)?.is_some() {
            let k = fields
                .kingdom(first)?
                .ok_or_else(|| missing("76BA90 Kingdom became NULL"))?;
            if fields.result(k)? != 2 {
                let mut second = 0;
                while second < fields.count() {
                    if first != second && !fields.ai(second)? && fields.kingdom(second)?.is_some() {
                        let k = fields
                            .kingdom(second)?
                            .ok_or_else(|| missing("76BA90 Kingdom became NULL"))?;
                        if fields.result(k)? != 2 {
                            let k = fields
                                .kingdom(first)?
                                .ok_or_else(|| missing("76BA90 Kingdom became NULL"))?;
                            let a = fields.team(k)?;
                            let k = fields
                                .kingdom(second)?
                                .ok_or_else(|| missing("76BA90 Kingdom became NULL"))?;
                            if a != fields.team(k)? {
                                return Ok(true);
                            }
                            let k = fields
                                .kingdom(first)?
                                .ok_or_else(|| missing("76BA90 Kingdom became NULL"))?;
                            if fields.team(k)?.is_none() {
                                let k = fields
                                    .kingdom(second)?
                                    .ok_or_else(|| missing("76BA90 Kingdom became NULL"))?;
                                if fields.team(k)?.is_none() {
                                    return Ok(true);
                                }
                            }
                        }
                    }
                    second += 1;
                }
            }
        }
        first += 1;
    }
    Ok(false)
}
struct Opposition<'a, 'owner> {
    players: &'a PlayersBorrow<'owner>,
    actors: &'a ActorStore,
}
impl Opposition<'_, '_> {
    fn shell(
        &self,
        id: aa_runtime::load_tail_world::KingdomId,
    ) -> Result<&aa_mapworld::special_state::SpecialShell> {
        let key = self
            .actors
            .key(id)
            .ok_or_else(|| missing("76BA90 current Kingdom generation is absent"))?;
        let entity = self
            .actors
            .entity(key)
            .map_err(|e| missing(e.to_string()))?;
        let Source::Special(s) = &entity.source else {
            return Err(missing("76BA90 player28 is not a special Kingdom"));
        };
        if !matches!(s.source, SpecialSource::Kingdom(_)) {
            return Err(missing("76BA90 player28 is not a Kingdom"));
        }
        s.derived
            .as_ref()
            .ok_or_else(|| missing("76BA90 current Kingdom fields are absent"))
    }
}
impl OppositionFields for Opposition<'_, '_> {
    type Kingdom = aa_runtime::load_tail_world::KingdomId;
    type Team = aa_runtime::load_tail_world::KingdomId;
    fn count(&mut self) -> usize {
        self.players.player_count()
    }
    fn ai(&mut self, index: usize) -> Result<bool> {
        self.players.player_ai(&self.players.player_at(index)?)
    }
    fn kingdom(&mut self, index: usize) -> Result<Option<Self::Kingdom>> {
        //765E00 checks GWorld[B36534], not Sim3C. This adapter is lent by
        //the genuine retained current frame; its constructor/publication
        //identity is checked before process_task_status makes any stores.
        self.players
            .player_kingdom_28(&self.players.player_at(index)?)
            .map_err(missing)
    }
    fn result(&mut self, id: Self::Kingdom) -> Result<u32> {
        self.shell(id)?
            .words
            .get(&0x19c)
            .copied()
            .ok_or_else(|| missing("76BA90 current Kingdom19C is unbound"))
    }
    fn team(&mut self, id: Self::Kingdom) -> Result<Option<Self::Team>> {
        let shell = self.shell(id)?;
        match shell.identity_references.get(&0x1ec) {
            Some(value) => Ok(*value),
            None if shell.words.get(&0x1ec) == Some(&0) => Ok(None),
            _ => Err(missing("76BA90 current Kingdom1EC team is unbound")),
        }
    }
}
pub(super) fn human_opposition(players: &PlayersBorrow<'_>, actors: &ActorStore) -> Result<bool> {
    opposition(&mut Opposition { players, actors })
}
///76BB90 uses the SAME765E00/current Kingdom19C reads as76BA90, but
///returns the first non-AI, non-NULL, not-defeated player allocation.
pub(super) fn first_active_index(fields:&mut impl OppositionFields)->Result<Option<usize>>{
    let mut i=0;
    while i<fields.count(){
        if !fields.ai(i)?&&fields.kingdom(i)?.is_some(){
            let id=fields.kingdom(i)?.ok_or_else(||missing("76BB90 Kingdom became NULL"))?;
            if fields.result(id)?!=2{return Ok(Some(i))}
        }
        i+=1;
    }
    Ok(None)
}
pub(super) fn first_active_human(players:&PlayersBorrow<'_>,actors:&ActorStore)->Result<Option<PlayerHandle>>{
    first_active_index(&mut Opposition{players,actors})?.map(|i|players.player_at(i)).transpose()
}
///Called only by the actual315 order-effects path. Failure leaves the native
///status/history/Session flag/request prefixes committed. Failed-task773AE0
///remains an explicit required branch.
pub fn process_task_status<'owner, S: Services>(
    players: &mut PlayersBorrow<'owner>,
    local: &mut LocalClientBorrow<'owner>,
    slot: &u32,
    status: u32,
    context: &mut Context<'_, '_, '_, S>,
) -> Result<()> {
    context
        .sim
        .require_created_world_witness(&context.current_world)
        .map_err(missing)?;
    super::world_ready::tasks::process_current_315(
        players,
        local,
        slot,
        status,
        &mut |players, success| {
            if !success {
                return Err(missing(
                    "776560 failed tasks require actual773AE0 World-failure handling",
                ));
            }
            run(&mut Current { players, context })
        },
    )
}
#[cfg(test)]
#[path = "local_all_tasks_complete_tests.rs"]
mod tests;
