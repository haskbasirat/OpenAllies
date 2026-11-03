//! The running battle of a LAN session: `Battle` as the session's world
//! ([`aa_netcode::lan::LanGame`]). A person's commands leave as the original's
//! requests, the decrees that come back are the only way anything reaches the
//! world, and world ticks run when the original's frame driver says so
//! (`LanNode::frame`, 0x75CA70). `session::step_battle` hands over to [`step`].
use super::SharedNode;
use crate::session::Battle;
use aa_netcode::lan::{LanEvent, LanGame};
use aa_orders::payload::{Field, ThingCheck};
use aa_play::player_orders::native::{self, Outbox, Replay, Tables};
use aa_play::{Command, KingdomId};
use aa_simulation::NativeClock;
use bevy::log::{error, warn};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// What a LAN battle keeps beside its world.
pub struct Link {
    pub node: SharedNode,
    /// SessionPlayer id -> kingdom.
    pub kingdoms: BTreeMap<u32, KingdomId>,
    pub tables: Tables,
    pub outbox: Outbox,
    pub replay: Replay,
    /// Decree commands waiting for the next world update.
    pub pending: Vec<Command>,
    /// Decrees that came to nothing here, for the log.
    pub refused: usize,
    /// The session's events for the front end.
    pub events: Arc<Mutex<Vec<LanEvent>>>,
    /// The session stopped (a failed frame): nothing more runs.
    pub failed: bool,
    /// The film's position in the last frame (world ticks so far).
    pub digest_log: Option<std::fs::File>,
    /// The last pause and speed the HUD asked for.
    pub paused_asked: bool,
    pub speed_asked: f32,
}

struct Game<'a> {
    battle: &'a mut Battle,
    link: &'a mut Link,
}

impl LanGame for Game<'_> {
    fn decree(&mut self, player: Option<u32>, d: usize, fields: &[Field]) -> aa_netcode::Result<()> {
        let Some(player) = player else { return Ok(()) };
        let kingdom = self.link.kingdoms.get(&player).copied();
        match native::commands(&self.battle.world, &self.link.tables, &mut self.link.replay, kingdom, player, d, fields) {
            Ok(c) => self.link.pending.extend(c),
            Err(why) => {
                self.link.refused += 1;
                warn!("lan: decree {d} of player {player} came to nothing: {why}");
            }
        }
        Ok(())
    }
    fn thing(&self, _check: ThingCheck, slot: u16) -> Option<u16> {
        aa_play::ids::unit(self.battle.world.ids.thing_at_slot(slot)?).map(|_| slot)
    }
    fn world_clock(&self) -> Option<NativeClock> {
        Some(self.battle.world.clock)
    }
    /// 0x76EE10: the kingdom of a player who left goes on under a computer
    /// player, on every node at this decree (difficulty: the middle level).
    fn implicit_ai(&mut self, player: u32) -> aa_netcode::Result<()> {
        if let Some(k) = self.link.kingdoms.remove(&player)
            && let Some(kingdom) = self.battle.world.kingdoms.get_mut(k as usize)
        {
            kingdom.controller = aa_play::Controller::Computer(1);
        }
        Ok(())
    }
    fn world_update(&mut self) -> aa_netcode::Result<(NativeClock, Vec<u32>)> {
        let b = &mut *self.battle;
        // GetRNG 0x4F02B0 logs to A: the getter values of this update, as the
        // trace records them.
        let start = b.world.trace.entries().count();
        let commands = std::mem::take(&mut self.link.pending);
        match b.recorder.as_mut() {
            Some(recorder) => recorder.step(&mut b.world, commands),
            None => b.world.step(commands),
        }
        let draws: Vec<u32> = b.world.trace.entries().skip(start).filter(|e| matches!(e.1, aa_play::trace::SKind::RngGetter(_))).map(|e| e.2).collect();
        // keep the log short: only this update's entries were needed
        if let Some(log) = b.world.trace.log.as_mut() {
            log.clear();
        }
        let events = std::mem::take(&mut b.world.events);
        b.frame_events.extend(events.iter().cloned());
        b.world.events = events;
        if let Some(f) = self.link.digest_log.as_mut() {
            use std::io::Write;
            let d = aa_play::trace::digest(&b.world);
            let _ = writeln!(f, "{} {:016x} {:08x}", d.tick, d.world, d.rng_state);
        }
        b.world.trace.digests.clear();
        Ok((b.world.clock, draws))
    }
}

/// `step_battle` for a LAN battle: the HUD's commands go out as requests, then
/// one frame of the session.
pub fn step(battle: &mut Battle) {
    battle.frame_events.clear();
    if battle.local_kingdom().is_none() {
        battle.selection.clear();
        battle.pending.clear();
    }
    // the film's checkpoints: every 17th world tick stands for the admin ticks
    // (the recorder counts them itself here; 0x76AEE0 with the session's clock)
    if let Some(recorder) = battle.recorder.as_mut() {
        recorder.own_admin_clock = false;
    }
    let Some(mut link) = battle.lan.take() else { return };
    let node = link.node.clone();
    let mut guard = match node.lock() {
        Ok(g) => g,
        Err(_) => {
            battle.lan = Some(link);
            return;
        }
    };
    let Some(lan) = guard.as_mut() else {
        battle.lan = Some(link);
        return;
    };
    if link.failed {
        battle.lan = Some(link);
        return;
    }
    let commands = std::mem::take(&mut battle.pending);
    for command in commands {
        match native::requests(&battle.world, &link.tables, &mut link.outbox, &command) {
            Ok(requests) => {
                for (d, fields) in requests {
                    if let Err(e) = lan.request(d, &fields) {
                        warn!("lan: request {d} refused: {e}");
                    }
                }
            }
            Err(why) => warn!("lan: a command with no request: {why}"),
        }
    }
    // the HUD's pause and speed are orders in a multiplayer session (0x754880, 0x76F530)
    if battle.paused != link.paused_asked {
        link.paused_asked = battle.paused;
        let _ = lan.request(aa_netcode::lan::descriptor::SET_PAUSED_REQUEST, &[Field::Uint { width: 1, value: u32::from(battle.paused) }]);
    }
    if (battle.speed - link.speed_asked).abs() > 1e-6 {
        let exponent = battle.speed.max(1e-6).log2();
        link.speed_asked = battle.speed;
        let _ = lan.request(aa_netcode::lan::descriptor::SET_GAME_SPEED_REQUEST, &[Field::Uint { width: 32, value: exponent.to_bits() }]);
    }
    let result = {
        let mut game = Game { battle, link: &mut link };
        lan.frame(&mut game)
    };
    match result {
        Ok(events) => {
            if let Ok(mut q) = link.events.lock() {
                q.extend(events);
            }
        }
        Err(e) => {
            error!("lan: the session's frame failed: {e}");
            link.failed = true;
            if let Ok(mut q) = link.events.lock() {
                q.push(LanEvent::Disconnected { reason: None });
            }
        }
    }
    // the interpolation between ticks follows the driver's game time
    drop(guard);
    battle.lan = Some(link);
}
