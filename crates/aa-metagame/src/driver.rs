//! The engine side of a single-player World War II campaign, without the RTS
//! engine: what the original world, net node and MetaGame interface do around
//! the ported metagame rules, so a campaign can be played end to end.
//!
//! | native | role here |
//! |---|---|
//! | 944A20 / 9399A0 | new campaign world: `KKC_MGState` (9397E0) + InitCreate (939B90) |
//! | 9447E0 | one world kingdom per campaign kingdom slot, team of the slot = (last) campaign team listing it, the default kingdom's faction = the chosen general, the local player plays `default_kingdom` (76AC60) |
//! | 765410 order execution | orders sent in a tick are executed by the net node at the next tick: 9505F0 = 951D70 + 954570 ([`execute_order`]) |
//! | 940020 | the per-tick waypoint mover of every army component (unless the actor is still moving) |
//! | 945090 on the SAI fiber 79E2C0 | the MGAI controller fiber, here on its own thread; every switch of 79DFF0 hands the state to the engine side and back |
//! | 944E60 | queued MGAI commands flushed at a switch become orders of the current kingdom |
//! | interface update 93694D..936A07 | a started, unresolved battle: with `MetaGameUIComputerQuickBattle` (UVar 0xCE, `[B39940]+0x338`) and the local player not involved (94D620) the local player sends quickbattle (94D4F0), else the Start Battle dialog opens ("StartBattle" 932A60) |
//!
//! The fiber thread stands for the native fiber: PlayTurn runs on it and
//! blocks at a switch until the engine side resumes it, exactly where the
//! native fiber switches back to the main fiber. Everything else runs on the
//! caller's thread, one call to [`Campaign::tick`] per world tick
//! (`[B36534]+0xDC`).
//!
//! Engine models that are not native code (documented choices):
//! - world kingdom actors are `KingdomActor(campaign kingdom index)` and their
//!   engine names (4BEDF0) the campaign kingdom IDS;
//! - the relation virtuals (kingdom actor vtable+0x140 / +0x138) follow the
//!   slot teams of 9447E0: enemies across teams, allies (and self) within one;
//!   a kingdom without a team is everyone's enemy;
//! - a kingdom actor's army list (`+0x284`) is its live armies in creation
//!   order; the army actor update order is creation order too;
//! - the uninitialised 96A5D0 block reads as zero bytes (fresh heap pages);
//! - a 79DFF0 switch at a yield with argument 0 happens only when the
//!   optional time budget ([`Options::budget`]) is spent (the native budget
//!   is wall-clock time: `KKC_SAI+0x48` = now + the 765410 think argument,
//!   UVar `SAITimePerThink`, 7A0080);
//! - one switch per world tick stands for the wall-clock schedule of 765410
//!   (UVar `SAITimeBetweenThinks`).
//!
//! A computer player that reaches an unported (`Unsupported`) path stops the
//! campaign: [`Note::AiFailed`] and an error from every later
//! [`Campaign::tick`]. Nothing is played in its place.
use crate::battle::{KingdomActor, KingdomRelations, Side};
use crate::commands::{BUILTIN_HOT, Command, CommandData, NativePaths, Validator};
use crate::data::{MetaGameData, MG_COMMANDS};
use crate::mgai::controller::{execute_order, flush, MgaiController, MgaiEngine, OrderEffect, OrderResult, SaiResume};
use crate::mgai::KingdomArmy;
use crate::resolve::{Engine, EngineCall, MgContext};
use crate::rng::MetaGameRng;
use crate::state::{ArmyId, InitEngine, MgState};
use crate::{Error, Result, GAME_PRECISION};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The world kingdom actor of campaign kingdom `k` (9447E0 creates one
/// kingdom slot per campaign kingdom, in campaign order).
pub fn kingdom_actor(k: usize) -> KingdomActor {
    KingdomActor(k as u32)
}

/// Kingdom relations of a campaign world: the team of every kingdom slot
/// (9447E0: each campaign team listing the kingdom assigns its IDS to the
/// slot, so the last one wins). Enemies across teams, allies within a team.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TeamRelations {
    /// Team index per campaign kingdom (= kingdom actor id).
    pub team: Vec<Option<usize>>,
}

impl TeamRelations {
    pub fn of_campaign(data: &MetaGameData, campaign: usize) -> Self {
        let c = &data.campaigns[campaign];
        let mut team = vec![None; c.kingdoms.len()];
        for (ti, t) in c.teams.iter().enumerate() {
            for &k in &t.kingdoms {
                if let Some(slot) = team.get_mut(k) {
                    *slot = Some(ti);
                }
            }
        }
        Self { team }
    }
    fn team(&self, a: KingdomActor) -> Option<usize> {
        self.team.get(a.0 as usize).copied().flatten()
    }
}

impl KingdomRelations for TeamRelations {
    fn enemy(&self, s: KingdomActor, o: KingdomActor) -> bool {
        s != o && (self.team(s).is_none() || self.team(s) != self.team(o))
    }
    fn relation_138(&self, s: KingdomActor, o: KingdomActor) -> bool {
        s == o || (self.team(s).is_some() && self.team(s) == self.team(o))
    }
}

/// Settings the engine side takes from the user variables.
#[derive(Clone, Debug)]
pub struct Options {
    /// UVar `MetaGameUIComputerQuickBattle` (id 0xCE, default false in the
    /// shipped UVars): the local interface answers battles it is not part of
    /// with quickbattle.
    pub computer_quick_battle: bool,
    /// Test harness, not native: with no local interface every started battle
    /// is answered with quickbattle, as a player who always picks Quick
    /// Battle.
    pub auto_quick_battle: bool,
    /// The 79DFF0 time budget for yields with argument 0, natively UVar
    /// `SAITimePerThink` (`None`: such a yield never switches, which keeps
    /// runs deterministic).
    pub budget: Option<Duration>,
    /// UVar `MetaGameUIFastMGAI` (id 0xCD, `[B39940]+0x334`): the interface
    /// update (0x9368D0) orders a change with KKC_MGSetFastAITurnOrder (933AF0,
    /// `MGState+0x51`).
    pub fast_mgai: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self { computer_quick_battle: false, auto_quick_battle: false, budget: None, fast_mgai: false }
    }
}

/// What the engine side did, for the interface (message log, visuals).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    /// A seam call of the ported code (battle messages, kills, captures,
    /// move visuals, victory, game over, end-turn message, events).
    Engine(EngineCall),
    /// An order executed by 9505F0 (issuer MG kingdom, command, result, and
    /// for a move or attack the text key 9537B0 chooses before it runs).
    Order { kingdom: Option<usize>, command: Command, result: OrderResult, move_key: Option<&'static str> },
    /// The computer player of `kingdom` reached an unported path: the
    /// campaign stops here (see the module documentation).
    AiFailed { kingdom: usize, error: String },
}

/// Engine for the ported resolution / mover code: collects the seam calls.
struct NoteEngine<'a> {
    notes: &'a mut Vec<Note>,
    busy: &'a dyn Fn(ArmyId) -> bool,
    won: &'a mut Vec<KingdomActor>,
}

impl Engine for NoteEngine<'_> {
    fn kingdom_field_19c(&self, actor: KingdomActor) -> u32 {
        u32::from(self.won.contains(&actor))
    }
    fn army_busy(&self, army: ArmyId) -> bool {
        (self.busy)(army)
    }
    fn call(&mut self, call: EngineCall) {
        // 4E9720(1, 1) sets the winner's +0x19C
        if let EngineCall::KingdomVictory { actor } = call
            && !self.won.contains(&actor)
        {
            self.won.push(actor);
        }
        self.notes.push(Note::Engine(call));
    }
}

/// An empty state standing in the engine side while the fiber holds the real one.
fn placeholder(campaign: usize) -> MgState {
    MgState {
        campaign,
        current: None,
        rng: MetaGameRng::new(0),
        kingdoms: Vec::new(),
        territories: Vec::new(),
        armies: Vec::new(),
        round: 0,
        battle_started: false,
        battle_play_requested: false,
        battle_resolved: false,
        game_over: false,
        fast_ai: 0,
        last_purchased: None,
        results: Vec::new(),
        faction: None,
        rts_winner: String::new(),
        events: Vec::new(),
        kingdom_names: Vec::new(),
    }
}

// ------------------------------------------------------------------ the fiber

/// Engine side -> fiber.
enum ToFiber {
    Resume { state: MgState, tick: i32, busy: Vec<bool>, serials: Vec<u32>, local: Option<KingdomActor> },
    Stop,
}

/// Fiber -> engine side.
enum FromFiber {
    /// 79DFF0 switched back to the main fiber: the state and the commands
    /// 944E60 flushed at this switch.
    Yielded { state: MgState, commands: Vec<Command> },
    /// The fiber body returned with an error (the fiber held the state).
    Failed { state: MgState, error: String },
}

/// The fiber's view of the engine (between two switches).
struct FiberEngine {
    to_main: Sender<FromFiber>,
    from_main: Receiver<ToFiber>,
    tick: i32,
    busy: Vec<bool>,
    serials: Vec<u32>,
    local: Option<KingdomActor>,
    rng: MetaGameRng,
    budget: Option<Duration>,
    since: Instant,
}

impl MgaiEngine for FiberEngine {
    fn kingdom_armies(&self, state: &MgState, actor: KingdomActor) -> Result<Vec<KingdomArmy>> {
        Ok(state
            .armies
            .iter()
            .enumerate()
            .filter(|(_, a)| a.as_ref().is_some_and(|a| a.owner == actor))
            .map(|(i, _)| KingdomArmy { army: ArmyId(i as u32), deleted: false })
            .collect())
    }
    fn army_serial(&self, _state: &MgState, army: ArmyId) -> u32 {
        self.serials.get(army.0 as usize).copied().unwrap_or(0)
    }
    fn tick(&self) -> i32 {
        self.tick
    }
    fn army_busy(&mut self, _state: &MgState, army: ArmyId) -> Result<bool> {
        Ok(self.busy.get(army.0 as usize).copied().unwrap_or(false))
    }
    fn uninitialized_nonzero(&mut self, _len: usize) -> Result<usize> {
        Ok(0)
    }
    fn app_rng(&mut self) -> &mut MetaGameRng {
        &mut self.rng
    }
    /// 76B960: the local kingdom has a human TGC player (byte +0x0C = 0);
    /// the computer kingdoms have no TGC player in a single-player campaign.
    fn kingdom_player(&self, actor: Option<KingdomActor>) -> Option<bool> {
        match (actor, self.local) {
            (Some(a), Some(l)) if a == l => Some(false),
            _ => None,
        }
    }
    fn sai_yield(&mut self, state: &mut MgState, arg: u8, queue: &mut Vec<Command>) -> Result<SaiResume> {
        if arg == 0 && self.budget.is_none_or(|b| self.since.elapsed() < b) {
            return Ok(SaiResume::Resumed { state_replaced: false });
        }
        let campaign = state.campaign;
        let real = std::mem::replace(state, placeholder(campaign));
        if self.to_main.send(FromFiber::Yielded { state: real, commands: flush(queue) }).is_err() {
            return Ok(SaiResume::Abandoned);
        }
        match self.from_main.recv() {
            Ok(ToFiber::Resume { state: s, tick, busy, serials, local }) => {
                *state = s;
                self.tick = tick;
                self.busy = busy;
                self.serials = serials;
                self.local = local;
                self.since = Instant::now();
                Ok(SaiResume::Resumed { state_replaced: false })
            }
            Ok(ToFiber::Stop) | Err(_) => Ok(SaiResume::Abandoned),
        }
    }
}

/// The MGAI controller fiber (945090) on its own thread.
struct Fiber {
    to_fiber: Sender<ToFiber>,
    /// Behind a lock so a campaign can live in shared (`Sync`) engine
    /// state; only the main side ever receives.
    from_fiber: std::sync::Mutex<Receiver<FromFiber>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Fiber {
    fn start(data: Arc<MetaGameData>, relations: TeamRelations, sai_seed: u32, budget: Option<Duration>) -> Self {
        let (to_fiber, from_main) = channel::<ToFiber>();
        let (to_main, from_fiber) = channel::<FromFiber>();
        let thread = std::thread::Builder::new()
            .name("mgai-fiber".into())
            .spawn(move || {
                // the fiber starts at the first switch to it (79E2C0)
                let Ok(ToFiber::Resume { mut state, tick, busy, serials, local }) = from_main.recv() else { return };
                let mut engine = FiberEngine {
                    to_main,
                    from_main,
                    tick,
                    busy,
                    serials,
                    local,
                    // KKC_SAI ctor 7A0AA0: 85D8A0 with the 59ABA0 timer milliseconds
                    rng: MetaGameRng::from_seed(sai_seed),
                    budget,
                    since: Instant::now(),
                };
                let mut controller = MgaiController::new();
                if let Err(e) = controller.run(&mut state, &data, &relations, GAME_PRECISION, &mut engine) {
                    let _ = engine.to_main.send(FromFiber::Failed { state, error: e.to_string() });
                }
            })
            .expect("spawn the MGAI fiber thread");
        Self { to_fiber, from_fiber: std::sync::Mutex::new(from_fiber), thread: Some(thread) }
    }
}

impl Drop for Fiber {
    fn drop(&mut self) {
        let _ = self.to_fiber.send(ToFiber::Stop);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

// ------------------------------------------------------------------ the campaign

/// A running campaign: the metagame state and its engine side.
pub struct Campaign {
    pub data: Arc<MetaGameData>,
    pub state: MgState,
    pub relations: TeamRelations,
    /// MG kingdom the local player plays (9447E0: the campaign's
    /// `default_kingdom`); `None` when no interface is attached.
    pub local: Option<usize>,
    pub options: Options,
    /// World tick `[B36534] + 0xDC`.
    pub tick: i32,
    /// Actor handle serial per army: the respawn 955DD0 on a type change
    /// replaces the actor, which invalidates stored handles.
    pub serials: Vec<u32>,
    /// Orders sent in the current tick (issuer, command), executed by the
    /// net node at the next tick.
    pub sent: Vec<(KingdomActor, Command)>,
    /// Kingdom actors whose `+0x19C` the victory call 4E9720(1, 1) set.
    pub won: Vec<KingdomActor>,
    /// What happened since the interface last drained it.
    pub notes: Vec<Note>,
    /// Why the computer player stopped the campaign (see the module
    /// documentation); [`Self::tick`] refuses to go on after it.
    pub stopped: Option<String>,
    sai_seed: u32,
    fiber: Option<Fiber>,
}

impl Campaign {
    /// A new campaign (DynamicCampaignNew 941F90 -> 944A20): the MGState of
    /// campaign `campaign` with the chosen faction (`MGState+0x0C`, the
    /// general picked by SetGeneral 941A60), InitCreate on the campaign world.
    /// `local`: the MG kingdom the local player plays (usually the
    /// campaign's `default_kingdom`). `sai_seed`: the SAI RNG seed (natively
    /// the 59ABA0 timer milliseconds).
    pub fn new(
        data: Arc<MetaGameData>,
        campaign: usize,
        local: Option<usize>,
        faction: Option<String>,
        sai_seed: u32,
        options: Options,
    ) -> Result<Self> {
        let c = data.campaigns.get(campaign).ok_or_else(|| Error::Unsupported(format!("campaign {campaign}")))?;
        let relations = TeamRelations::of_campaign(&data, campaign);
        let names: Vec<String> = c.kingdoms.iter().map(|k| k.ids.clone()).collect();
        let kingdom_actor_fn = |i: usize| (i < names.len()).then(|| kingdom_actor(i));
        let kingdom_name_fn = |a: KingdomActor| names.get(a.0 as usize).cloned();
        // the GTID handle of the territory visual is engine identity (save only)
        let visual_fn = |t: usize| Some(0x0001_0000 + t as u32);
        let engine = InitEngine {
            kingdom_actor: &kingdom_actor_fn,
            kingdom_name: &kingdom_name_fn,
            territory_visual: &visual_fn,
            faction,
        };
        let state = MgState::init_create_with(&data, campaign, &engine, &relations)?;
        let serials = vec![0; state.armies.len()];
        Ok(Self {
            data,
            state,
            relations,
            local,
            options,
            tick: 0,
            serials,
            sent: Vec::new(),
            won: Vec::new(),
            notes: Vec::new(),
            stopped: None,
            sai_seed,
            fiber: None,
        })
    }

    /// A campaign around a state built elsewhere (a loaded save).
    pub fn from_state(data: Arc<MetaGameData>, state: MgState, local: Option<usize>, sai_seed: u32, options: Options) -> Self {
        let relations = TeamRelations::of_campaign(&data, state.campaign);
        let serials = vec![0; state.armies.len()];
        Self {
            data,
            state,
            relations,
            local,
            options,
            tick: 0,
            serials,
            sent: Vec::new(),
            won: Vec::new(),
            notes: Vec::new(),
            stopped: None,
            sai_seed,
            fiber: None,
        }
    }

    /// The kingdom actor of MG kingdom `k`.
    pub fn actor(&self, k: usize) -> Option<KingdomActor> {
        self.state.kingdoms.get(k).and_then(|k| k.actor)
    }
    /// The local player's kingdom actor (`5C0E50`).
    pub fn local_actor(&self) -> Option<KingdomActor> {
        self.local.and_then(|k| self.actor(k))
    }
    /// The current MG kingdom is the local player's (`94D5D0`).
    pub fn local_turn(&self) -> bool {
        self.local.is_some() && self.state.current == self.local
    }

    /// A command by its built-in name (936CF0 lookup: `MG_COMMANDS`).
    pub fn command(name: &str) -> Option<Command> {
        let index = MG_COMMANDS.iter().position(|n| n.eq_ignore_ascii_case(name))?;
        Some(Command { data: Some(CommandData::builtin(index as u32)), ..Command::default() })
    }

    /// `0x951D70` for the local player (the interface's check before
    /// sending, 935A70; a nonzero code is shown with 952EC0).
    pub fn validate_local(&self, command: &Command) -> Result<u32> {
        let Some(issuer) = self.local_actor() else {
            return Err(Error::Unsupported("validation without a local player".into()));
        };
        self.validate(command, issuer)
    }

    /// `0x951D70` for `issuer`, with the ported path search and the
    /// army virtual +0x134 (controllable by its owner).
    pub fn validate(&self, command: &Command, issuer: KingdomActor) -> Result<u32> {
        let owners: Vec<Option<KingdomActor>> = self.state.armies.iter().map(|a| a.as_ref().map(|a| a.owner)).collect();
        let controllable = move |a: ArmyId, i: KingdomActor| owners.get(a.0 as usize).copied().flatten() == Some(i);
        let world = NativePaths { data: &self.data, relations: &self.relations, precision: GAME_PRECISION, controllable };
        Validator { state: &self.state, data: &self.data, relations: &self.relations, world: &world, precision: GAME_PRECISION }
            .validate(command, issuer)
    }

    /// `0x9521A0` for `issuer`: the first hot built-in command (authored
    /// `hot_command`, +0x2C, else the built-in default) whose requirement
    /// flags (+0x24) the filled fields of `command` satisfy: 0x100 an army,
    /// 0x800 owned by the issuer, 0x200 its virtual +0x138, 0x400 its virtual
    /// +0x140; 0x1000 an actor definition; 0x10 a territory, 0x20 hostile
    /// (954B30), 0x40 friendly (954C00); 0x10000 a technology; the hot-key
    /// bit 0x1000000 is ignored. The right click on a territory (956C20) sends
    /// what this chooses.
    pub fn hot_command_for(&self, command: &Command, issuer: KingdomActor) -> Option<CommandData> {
        let mut have = 0u32;
        if let Some(owner) = command.army.and_then(|a| self.state.armies.get(a.0 as usize)).and_then(|a| a.as_ref()).map(|a| a.owner) {
            have |= 0x100;
            if owner == issuer {
                have |= 0x800;
            }
            if self.relations.relation_138(owner, issuer) {
                have |= 0x200;
            }
            if self.relations.enemy(owner, issuer) {
                have |= 0x400;
            }
        }
        if command.actor_definition.is_some() {
            have |= 0x1000;
        }
        if let Some(t) = command.territory.and_then(|d| self.state.territories.iter().position(|t| t.definition == d)) {
            have |= 0x10;
            let world = NativePaths { data: &self.data, relations: &self.relations, precision: GAME_PRECISION, controllable: |_: ArmyId, _: KingdomActor| false };
            let v = Validator { state: &self.state, data: &self.data, relations: &self.relations, world: &world, precision: GAME_PRECISION };
            if v.hostile(t, issuer) {
                have |= 0x20;
            }
            if v.friendly(t, issuer) {
                have |= 0x40;
            }
        }
        if command.technology.is_some() {
            have |= 0x10000;
        }
        (0..MG_COMMANDS.len()).find_map(|i| {
            let hot = self.data.commands.iter().find(|c| c.index == i).and_then(|c| c.hot_command).unwrap_or(BUILTIN_HOT[i]);
            let data = CommandData::builtin(i as u32);
            (hot && data.flags & !0x0100_0000 & !have == 0).then_some(data)
        })
    }

    /// Send an order of the local player (`9506D0`); it is executed at the
    /// next tick (the net node runs 9505F0, which validates again).
    pub fn send_local(&mut self, command: Command) -> Result<()> {
        let Some(issuer) = self.local_actor() else {
            return Err(Error::Unsupported("order without a local player".into()));
        };
        self.sent.push((issuer, command));
        Ok(())
    }

    /// The MG kingdom of a kingdom actor (938B70).
    pub fn kingdom_of_actor(&self, actor: KingdomActor) -> Option<usize> {
        self.state.kingdoms.iter().position(|k| k.actor == Some(actor))
    }

    /// `0x94D620`: the local player is part of the pending battle: it
    /// owns the defending kingdom (955EB0) or the current kingdom (94D5D0).
    pub fn local_involved(&self) -> Result<bool> {
        let Some(local) = self.local_actor() else { return Ok(false) };
        if let Some(t) = self.pending_battle()?
            && let Some(d) = self.defending_kingdom(t)?
            && self.state.kingdoms[d].actor == Some(local)
        {
            return Ok(true);
        }
        Ok(self.local_turn())
    }

    /// `0x93A320`: the pending battle territory.
    pub fn pending_battle(&self) -> Result<Option<usize>> {
        let world = NativePaths { data: &self.data, relations: &self.relations, precision: GAME_PRECISION, controllable: |_: ArmyId, _: KingdomActor| false };
        Validator { state: &self.state, data: &self.data, relations: &self.relations, world: &world, precision: GAME_PRECISION }.pending_battle()
    }

    /// A read-only resolution context on a copy of the state (for the
    /// interface's battle queries).
    fn with_context<T>(&self, f: impl FnOnce(&MgContext<'_, TeamRelations, NoteEngine<'_>>) -> Result<T>) -> Result<T> {
        let mut notes = Vec::new();
        let mut won = self.won.clone();
        let busy = |_: ArmyId| false;
        let mut engine = NoteEngine { notes: &mut notes, busy: &busy, won: &mut won };
        let mut state = self.state.clone();
        let ctx = MgContext { state: &mut state, data: &self.data, relations: &self.relations, engine: &mut engine, precision: GAME_PRECISION };
        f(&ctx)
    }

    /// `0x955EB0` on MG territory `t`: the defending MG kingdom.
    pub fn defending_kingdom(&self, t: usize) -> Result<Option<usize>> {
        self.with_context(|ctx| ctx.defending_kingdom(t))
    }

    /// `0x95F370` on MG territory `t`: the attack odds as binary32 bits
    /// (the Start Battle dialog's odds tooltip 94E610 computes them on the
    /// main thread, `precision`).
    pub fn attack_odds_bits(&self, t: usize, precision: aa_numeric::Precision) -> Result<u32> {
        self.with_context(|ctx| ctx.attack_odds_bits(t, precision))
    }

    /// The Start Battle dialog's conditions (932A60 / 94E560): a battle was
    /// started (`+0x44`), is not resolved (`+0x46`), no RTS play was
    /// requested (`+0x45`) and there is a pending battle.
    pub fn battle_waiting(&self) -> Result<bool> {
        Ok(self.state.battle_started
            && !self.state.battle_resolved
            && !self.state.battle_play_requested
            && self.pending_battle()?.is_some())
    }

    /// One world tick of the engine side: the orders sent last tick, the
    /// waypoint movers, the interface's battle answer and one switch to the
    /// MGAI fiber. `busy(army)`: the army actor is still moving
    /// (`+0x70` vtable `+0x64`); the view answers it.
    pub fn tick(&mut self, busy: &dyn Fn(ArmyId) -> bool) -> Result<()> {
        if let Some(error) = &self.stopped {
            return Err(Error::Unsupported(error.clone()));
        }
        self.tick = self.tick.wrapping_add(1);
        // 0x9368D0 -> 933AF0: the fast-AI byte follows the UVar
        self.state.fast_ai = u8::from(self.options.fast_mgai);
        // 1. the orders of the last tick (net node, 9505F0)
        for (issuer, command) in std::mem::take(&mut self.sent) {
            self.execute(issuer, &command, busy)?;
        }
        // 2. 940020 on every army component (actor update order = creation order)
        for i in 0..self.state.armies.len() {
            let id = ArmyId(i as u32);
            if self.state.armies[i].as_ref().is_some_and(|a| !a.waypoints.is_empty()) {
                let mut engine = NoteEngine { notes: &mut self.notes, busy, won: &mut self.won };
                let mut ctx = MgContext { state: &mut self.state, data: &self.data, relations: &self.relations, engine: &mut engine, precision: GAME_PRECISION };
                ctx.step_orders(id)?;
            }
        }
        // 3. the interface update's battle block (93694D..936A07)
        if self.battle_waiting()? {
            let answer = if self.local.is_some() {
                self.options.computer_quick_battle && !self.local_involved()?
            } else {
                self.options.auto_quick_battle
            };
            if answer && !self.sent.iter().any(|(_, c)| c.data.map(|d| d.index) == Some(5)) {
                // 94D4F0 (the local player's order; headless: the current kingdom's)
                let issuer = self.local_actor().or_else(|| self.state.current.and_then(|k| self.actor(k)));
                if let (Some(issuer), Some(cmd)) = (issuer, Self::command("quickbattle")) {
                    self.sent.push((issuer, cmd));
                }
            }
        }
        // 4. one switch to the MGAI fiber (SAI think inside the net node update)
        if !self.state.game_over {
            self.switch_fiber(busy)?;
        }
        Ok(())
    }

    /// 9505F0 for one order.
    fn execute(&mut self, issuer: KingdomActor, command: &Command, busy: &dyn Fn(ArmyId) -> bool) -> Result<()> {
        let owners: Vec<Option<KingdomActor>> = self.state.armies.iter().map(|a| a.as_ref().map(|a| a.owner)).collect();
        let controllable = move |a: ArmyId, i: KingdomActor| owners.get(a.0 as usize).copied().flatten() == Some(i);
        let move_key = self.move_key(command, issuer)?;
        let mut engine = NoteEngine { notes: &mut self.notes, busy, won: &mut self.won };
        let result = execute_order(&mut self.state, &self.data, &self.relations, GAME_PRECISION, command, issuer, controllable, &mut engine)?;
        if let Some(OrderEffect::Process(out)) = &result.effect {
            if out.created.is_some() {
                self.serials.resize(self.state.armies.len(), 0);
            }
            if let Some(id) = out.respawned {
                // 955DD0 replaces the actor: a new handle
                if let Some(s) = self.serials.get_mut(id.0 as usize) {
                    *s = s.wrapping_add(1);
                }
            }
        }
        self.serials.resize(self.state.armies.len(), 0);
        let kingdom = self.kingdom_of_actor(issuer);
        self.notes.push(Note::Order { kingdom, command: command.clone(), result, move_key });
        Ok(())
    }

    /// The text key of `0x9537B0` for a move or attack: into a hostile
    /// territory (954B30) by its defender strength (954FD0 zero or not), else a
    /// plain move.
    fn move_key(&self, command: &Command, issuer: KingdomActor) -> Result<Option<&'static str>> {
        if !matches!(command.data.map(|d| d.index), Some(0 | 1)) {
            return Ok(None);
        }
        let Some(t) = command.territory.and_then(|d| self.state.territories.iter().position(|t| t.definition == d)) else {
            return Ok(None);
        };
        let world = NativePaths { data: &self.data, relations: &self.relations, precision: GAME_PRECISION, controllable: |_: ArmyId, _: KingdomActor| false };
        let v = Validator { state: &self.state, data: &self.data, relations: &self.relations, world: &world, precision: GAME_PRECISION };
        Ok(Some(if !v.hostile(t, issuer) {
            "metagame_army_move|ss"
        } else if f32::from_bits(v.defender_strength(t)?) == 0.0 {
            "metagame_army_attack_unoccupied|ss"
        } else {
            "metagame_army_attack_occupied|ss"
        }))
    }

    /// Resume the fiber for one switch; the commands it flushed become
    /// orders of the current kingdom (944E60 -> its TGC player).
    fn switch_fiber(&mut self, busy: &dyn Fn(ArmyId) -> bool) -> Result<()> {
        if self.fiber.is_none() {
            self.fiber = Some(Fiber::start(self.data.clone(), self.relations.clone(), self.sai_seed, self.options.budget));
        }
        let busy: Vec<bool> = (0..self.state.armies.len()).map(|i| busy(ArmyId(i as u32))).collect();
        let local = self.local_actor();
        let campaign = self.state.campaign;
        let state = std::mem::replace(&mut self.state, placeholder(campaign));
        let fiber = self.fiber.as_ref().expect("fiber started");
        if let Err(std::sync::mpsc::SendError(ToFiber::Resume { state, .. })) =
            fiber.to_fiber.send(ToFiber::Resume { state, tick: self.tick, busy, serials: self.serials.clone(), local })
        {
            self.state = state;
            self.fiber = None;
            return Err(Error::Unsupported("the MGAI fiber thread is gone".into()));
        }
        let answer = fiber.from_fiber.lock().unwrap_or_else(std::sync::PoisonError::into_inner).recv();
        match answer {
            Ok(FromFiber::Yielded { state, commands }) => {
                self.state = state;
                let issuer = self.state.current.and_then(|k| self.actor(k));
                if let Some(issuer) = issuer {
                    self.sent.extend(commands.into_iter().map(|c| (issuer, c)));
                }
                Ok(())
            }
            Ok(FromFiber::Failed { state, error }) => {
                self.state = state;
                self.fiber = None;
                let kingdom = self.state.current.unwrap_or(0);
                self.notes.push(Note::AiFailed { kingdom, error: error.clone() });
                self.stopped = Some(error.clone());
                Err(Error::Unsupported(error))
            }
            Err(_) => Err(Error::Unsupported("the MGAI fiber thread ended without its state".into())),
        }
    }

    /// Stop the fiber (a new or loaded game replaces the MGState; the
    /// controller deletes its players, 944D20).
    pub fn stop_ai(&mut self) {
        self.fiber = None;
    }

    /// Ticks until the local player's turn (or the game is over), at most
    /// `max_ticks`; returns the ticks run.
    pub fn run_until_local_turn(&mut self, max_ticks: usize, busy: &dyn Fn(ArmyId) -> bool) -> Result<usize> {
        for n in 0..max_ticks {
            if self.state.game_over || (self.local_turn() && self.sent.is_empty() && !self.battle_waiting()?) {
                return Ok(n);
            }
            self.tick(busy)?;
        }
        Ok(max_ticks)
    }

    /// Armies of a side in territory `t` (955420 / 9554F0 filters).
    pub fn side_armies(&self, t: usize, side: Side) -> Vec<ArmyId> {
        let owner = self.state.owner_actor(t);
        self.state.territories[t]
            .armies
            .iter()
            .map(|r| r.army)
            .filter(|&a| {
                let o = self.state.army(a).owner;
                match (side, owner) {
                    (Side::Attacker, None) => true,
                    (Side::Attacker, Some(w)) => self.relations.enemy(w, o),
                    (Side::Defender, None) => false,
                    (Side::Defender, Some(w)) => !self.relations.enemy(w, o),
                }
            })
            .collect()
    }
}
