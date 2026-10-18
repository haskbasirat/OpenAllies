//! The MGAI controller (vtable AC8E74, 0x20 bytes, ctor 944D60, dtor
//! 944DE0/944F30) and the execution of the orders it sends.
//!
//! Native layout: +0x04 the SAI (`KKC_SAI`, set by 945090; its fiber
//! scheduler is 79DFF0), +0x08/+0x0C/+0x10 command queue (944F50 appends,
//! 944E60 flushes), +0x14/+0x18/+0x1C one `KKC_MGAIPlayer` per MG kingdom
//! (944F90 creates them, 944D20 deletes them).
//!
//! Engine context (not ported, explicit seams in [`MgaiEngine`]): the SAI
//! think 7A0080 creates the controller (79E830, with the fiber 79E2C0 whose
//! body is 945090), switches to the fiber with a time budget and, every time
//! the fiber switches back, flushes the queue (944E60). The flush wraps each
//! queued command into an order (950470, vtable ACAC20) and submits it to the
//! current kingdom's TGC player (order virtual +0x08 = 950770 -> 9506D0:
//! network message 766CC0..766D00 / 951C20); the receiving net node executes
//! the order later with 9507D0 -> 9505F0: issuer = kingdom actor of the
//! order's source TGC player (74B3B0 / 765E00, with a cheat-dependent
//! fallback over world kingdom actors without a TGC player), then 9525A0
//! (951D70 == OK) and `KKC_MGCommand::Process` 954570. [`execute_order`] is
//! that execution after the issuer is known; when it runs relative to the
//! fiber is TGC timing (engine).
use super::{ArmyHandle, KingdomArmy, MgaiHost, MgaiPlayer, Stage};
use crate::battle::{KingdomActor, KingdomRelations};
use crate::commands::{Command, NativePaths, ProcessOutcome, Processor, Validator};
use crate::data::MetaGameData;
use crate::resolve::{Engine, MgContext};
use crate::rng::MetaGameRng;
use crate::state::{ArmyId, MgState};
use crate::{Error, Result};
use aa_numeric::Precision;
use serde::{Deserialize, Serialize};

/// How the AI fiber continues after the SAI scheduler 79DFF0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SaiResume {
    /// The fiber was resumed. `state_replaced`: `[B49FA0]` (the MGState)
    /// differs from the value read before the switch (new or loaded game).
    Resumed { state_replaced: bool },
    /// The fiber is never resumed (its owner stopped running it): nothing
    /// after this switch happens.
    Abandoned,
}

/// Engine side of the controller and of the players it drives.
pub trait MgaiEngine {
    /// Kingdom actor army list (`actor + 0x284`), native order.
    fn kingdom_armies(&self, state: &MgState, actor: KingdomActor) -> Result<Vec<KingdomArmy>>;
    /// Current handle serial of a live army's actor (changes when the actor
    /// is replaced). Default 0.
    fn army_serial(&self, _state: &MgState, _army: ArmyId) -> u32 {
        0
    }
    /// World tick `[B36534] + 0xDC`.
    fn tick(&self) -> i32;
    /// Army actor `+0x70` virtual `+0x64`.
    fn army_busy(&mut self, state: &MgState, army: ArmyId) -> Result<bool>;
    /// Nonzero bytes of the uninitialised 96A5D0 block (see [`MgaiHost`]).
    fn uninitialized_nonzero(&mut self, len: usize) -> Result<usize>;
    /// The app/SAI RNG `[[B36528] + 0x2C]`.
    fn app_rng(&mut self) -> &mut MetaGameRng;
    /// `FUN_0076B960` on the net node `[B36550]` (TGC player whose kingdom
    /// actor 765E00 is `actor`): `None` without such a player, else its
    /// byte `+0x0C` (nonzero: the computer plays it).
    fn kingdom_player(&self, actor: Option<KingdomActor>) -> Option<bool>;
    /// `FUN_0079DFF0(arg)` on the SAI: switch to the main fiber when `arg`
    /// is nonzero or the time budget (+0x48) is spent. On every switch the
    /// SAI think 7A0080 flushes the controller queue ([`flush`], 944E60)
    /// after the switch returns; whatever the engine does until it resumes
    /// the fiber (including executing orders) may change the state.
    fn sai_yield(&mut self, state: &mut MgState, arg: u8, queue: &mut Vec<Command>) -> Result<SaiResume>;
    /// Observation hook after each PlayTurn stage (no native effect).
    fn stage(&mut self, _stage: Stage, _player: &MgaiPlayer, _state: &MgState) {}
}

/// `FUN_00944E60`: the queued commands in queue order, which the caller
/// submits as orders (950470 + order virtual +0x08) to the current
/// kingdom's TGC player; the queue is emptied.
pub fn flush(queue: &mut Vec<Command>) -> Vec<Command> {
    std::mem::take(queue)
}

/// The controller.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MgaiController {
    /// `+0x14`: one player per MG kingdom, state order.
    pub players: Vec<MgaiPlayer>,
    /// `+0x08`: issued, not yet flushed commands.
    pub queue: Vec<Command>,
}

/// The player-side view of the engine while one PlayTurn runs on the fiber.
struct Fiber<'a, E: MgaiEngine> {
    engine: &'a mut E,
    queue: &'a mut Vec<Command>,
    abandoned: bool,
}

impl<E: MgaiEngine> MgaiHost for Fiber<'_, E> {
    fn kingdom_armies(&self, state: &MgState, actor: KingdomActor) -> Result<Vec<KingdomArmy>> {
        self.engine.kingdom_armies(state, actor)
    }
    fn army_serial(&self, state: &MgState, army: ArmyId) -> u32 {
        self.engine.army_serial(state, army)
    }
    /// `FUN_00944CE0(arg)`: 4F5290 on the world twice (result unused, no
    /// effect), 79DFF0(arg) on the SAI (`controller + 4`); true when
    /// `[B49FA0]` is the same after the switch.
    fn yield_turn(&mut self, state: &mut MgState, arg: u8) -> Result<bool> {
        if self.abandoned {
            return Ok(false);
        }
        match self.engine.sai_yield(state, arg, self.queue)? {
            SaiResume::Resumed { state_replaced } => Ok(!state_replaced),
            SaiResume::Abandoned => {
                // The fiber never comes back: every native yield site returns
                // at once on false, so nothing observable follows.
                self.abandoned = true;
                Ok(false)
            }
        }
    }
    /// `FUN_00944F50`: append to the queue (ownership moves to the controller).
    fn issue(&mut self, _state: &MgState, command: Command) -> Result<()> {
        self.queue.push(command);
        Ok(())
    }
    fn tick(&self) -> i32 {
        self.engine.tick()
    }
    fn army_busy(&mut self, state: &MgState, army: ArmyId) -> Result<bool> {
        self.engine.army_busy(state, army)
    }
    fn uninitialized_nonzero(&mut self, len: usize) -> Result<usize> {
        self.engine.uninitialized_nonzero(len)
    }
    fn app_rng(&mut self) -> &mut MetaGameRng {
        self.engine.app_rng()
    }
    fn stage(&mut self, stage: Stage, player: &MgaiPlayer, state: &MgState) {
        self.engine.stage(stage, player, state);
    }
}

impl MgaiController {
    /// `FUN_00944D60`: empty queue and player list.
    pub fn new() -> Self {
        Self::default()
    }

    /// `FUN_00944F90`: one player (963430) per MG kingdom, state order.
    pub fn create_players<R: KingdomRelations>(&mut self, state: &MgState, data: &MetaGameData, relations: &R, precision: Precision) -> Result<()> {
        for k in 0..state.kingdoms.len() {
            let p = MgaiPlayer::new(state, data, relations, precision, k)?;
            self.players.push(p);
        }
        Ok(())
    }

    /// `FUN_00944D20`: delete every player (virtual 0 = 9632C0), count 0.
    pub fn delete_players(&mut self) {
        self.players.clear();
    }

    /// `FUN_00944F50`.
    pub fn issue(&mut self, command: Command) {
        self.queue.push(command);
    }

    /// `FUN_00945090(sai)`, the fiber body (79E2C0 tail-calls it). Returns
    /// when the engine abandons the fiber; native never returns.
    ///
    /// Every pass: create the players when there are none (944F90); unless
    /// MGState+0x45 (RTS battle play requested), when the computer plays the
    /// current kingdom (no TGC player, or its byte +0x0C set) and its player
    /// has not ended its turn (+0x29): PlayTurn 964D30, then clear +0x29 of
    /// every other player (a false PlayTurn marks an abort). Then 79DFF0(1)
    /// with the MGState check of 944CE0; a replaced state or an abort deletes
    /// the players (944D20).
    pub fn run<R: KingdomRelations, E: MgaiEngine>(
        &mut self,
        state: &mut MgState,
        data: &MetaGameData,
        relations: &R,
        precision: Precision,
        engine: &mut E,
    ) -> Result<()> {
        loop {
            if self.players.is_empty() {
                self.create_players(state, data, relations, precision)?;
            }
            let mut abort = false;
            if !state.battle_play_requested {
                // 938A00: index of MGState+0x10 in the kingdom list
                let cur = state
                    .current
                    .filter(|&c| c < state.kingdoms.len())
                    .ok_or_else(|| Error::Unsupported("945090: no current MG kingdom (native null dereference)".into()))?;
                let plays = engine.kingdom_player(state.kingdoms[cur].actor).unwrap_or(true);
                if plays {
                    let player = self.players.get_mut(cur).ok_or_else(|| {
                        Error::Unsupported("945090: current kingdom index beyond the player list (native reads past it)".into())
                    })?;
                    if !player.turn_ended {
                        let mut fiber = Fiber { engine: &mut *engine, queue: &mut self.queue, abandoned: false };
                        let ok = player.play_turn(state, data, relations, precision, &mut fiber)?;
                        if fiber.abandoned {
                            return Ok(());
                        }
                        abort = !ok;
                        for (i, p) in self.players.iter_mut().enumerate() {
                            if i != cur {
                                p.turn_ended = false;
                            }
                        }
                    }
                }
            }
            match engine.sai_yield(state, 1, &mut self.queue)? {
                SaiResume::Abandoned => return Ok(()),
                SaiResume::Resumed { state_replaced } => {
                    if state_replaced || abort {
                        self.delete_players();
                    }
                }
            }
        }
    }
}

/// What executing an order did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderEffect {
    /// 954570 through [`Processor::process`] (commands 0..=4, 7..=10).
    Process(ProcessOutcome),
    /// 954570 -> 952790 (endturn), returned byte.
    EndTurn(bool),
    /// 954570 -> 954220 (quickbattle), returned byte.
    QuickBattle(bool),
}

/// Result of [`execute_order`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderResult {
    /// 951D70 result code (0 = valid).
    pub code: u32,
    /// `None` when the validation failed (9505F0 only logs).
    pub effect: Option<OrderEffect>,
}

/// `FUN_009505F0` after the issuer resolution: 9525A0 (951D70 == 0, with
/// the 93FDC0 path search and the army virtual +0x134 `controllable`), then
/// `KKC_MGCommand::Process` 954570.
#[allow(clippy::too_many_arguments)]
pub fn execute_order<R: KingdomRelations, E: Engine, C: Fn(ArmyId, KingdomActor) -> bool>(
    state: &mut MgState,
    data: &MetaGameData,
    relations: &R,
    precision: Precision,
    command: &Command,
    issuer: KingdomActor,
    controllable: C,
    engine: &mut E,
) -> Result<OrderResult> {
    let world = NativePaths { data, relations, precision, controllable };
    let code = Validator { state, data, relations, world: &world, precision }.validate(command, issuer)?;
    if code != 0 {
        return Ok(OrderResult { code, effect: None });
    }
    let index = command.data.map(|d| d.index);
    let effect = match index {
        Some(5) => {
            let mut ctx = MgContext { state, data, relations, engine, precision };
            OrderEffect::QuickBattle(ctx.process_quick_battle()?)
        }
        Some(6) => {
            let mut ctx = MgContext { state, data, relations, engine, precision };
            OrderEffect::EndTurn(ctx.process_end_turn(issuer)?)
        }
        _ => {
            let mut p = Processor { state, data, relations, precision };
            OrderEffect::Process(p.process(command, issuer, &world)?)
        }
    };
    Ok(OrderResult { code, effect: Some(effect) })
}

/// Handle of the army's current actor (for engines that store handles).
pub fn current_handle<E: MgaiEngine>(engine: &E, state: &MgState, army: ArmyId) -> ArmyHandle {
    ArmyHandle { army, serial: engine.army_serial(state, army) }
}
