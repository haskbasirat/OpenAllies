//! One battle tick, in the original's system order as far as it is known
//! (commands, AI, the queue-0 deliveries of production and the kingdom
//! economy, movement, combat, morale, supply). The native update 0x4F9970
//! delivers queue registrations (`crate::scheduler`); the systems not yet
//! migrated to queue leaves run as passes where their queues run.
//! Shared file: change only by agreement.
use crate::world::{Command, World};

/// One decree of a film record for [`World::step_decrees`]: its S entries
/// (descriptor id and class name, `None` for a decree without an S log such
/// as SetPlayer 298) and the aa-play commands it maps to (possibly none:
/// selections, names, sync values).
#[derive(Clone, Debug, Default)]
pub struct TracedDecree {
    pub sync: Option<(u32, Vec<u16>)>,
    pub commands: Vec<Command>,
}

impl World {
    /// Advance one tick (1/16 s), applying `commands` first.
    ///
    /// As the original frame runs a world tick (0x752AE0): the tick's decrees
    /// run first, then the driver logs GWorld+0xE0 (`GetGameTimeBeginTick`,
    /// the begin of the last completed tick: 0, 0, 1/16, 2/16...) to S, then
    /// 0x4F9970 advances the clock (`tick++`, begin = old end,
    /// `end = f32(i32 tick * step)`) before any system runs, so the systems
    /// see the tick's end time.
    pub fn step(&mut self, commands: Vec<Command>) {
        self.events.clear();
        self.trace.tick = self.clock.tick.wrapping_add(1);
        for command in commands {
            self.apply(command);
        }
        self.update();
    }

    /// [`World::step`] for decrees that are logged to S as the original
    /// executes them (a film replay): each decree's name and id entries, then
    /// its commands, in order; then the tick.
    pub fn step_decrees(&mut self, decrees: Vec<TracedDecree>) {
        self.events.clear();
        self.trace.tick = self.clock.tick.wrapping_add(1);
        for decree in decrees {
            if let Some((id, name)) = &decree.sync {
                self.trace.log_decree(*id, name);
            }
            for command in decree.commands {
                self.apply(command);
            }
        }
        self.update();
    }

    /// The update after the tick's decrees: clock, then the systems.
    fn update(&mut self) {
        self.trace.log(crate::trace::SKind::Clock, self.clock.begin.0);
        self.clock.advance_fragment();
        self.tick = u64::from(self.clock.tick);
        self.time = f32::from_bits(self.clock.end.0);
        crate::scheduler::sync_clock(self);
        let ai_commands = crate::ai::think(self);
        for command in ai_commands {
            self.apply(command);
        }
        // (core) the periodic queues (GWorld+1AC/+1C0/+1D4, step 6 of
        // 0x4F9970): the production queues and the kingdom economy ticks
        // (victory inside) on queue 0, newest registration first
        // (crate::scheduler).
        crate::scheduler::periodic(self);
        crate::scheduler::deferred(self);
        crate::deploy::tick(self);
        crate::movement::tick(self);
        crate::combat::tick(self);
        crate::morale::tick(self);
        crate::supply::tick(self);
        // (core) the victory checks 0x4E9960 / 0x4E9A40 run inside each
        // kingdom's economy tick (crate::economy).
        // appended by the UI owner: rally points of this tick's recruits,
        // then the detection zones and fog after the actors moved
        crate::player_orders::tick(self);
        crate::fog::tick(self);
        // appended by the combat owner: commander experience, landing
        // special operations, enchantments (expiry, periodic effects) and
        // the SAI general powers
        crate::special_ops::tick(self);
        // (core) step 8 of 0x4F9970: the deferred +208 (the minds' "schedule
        // in" re-thinks), after the passes that stand for phases 4 and 5.
        crate::scheduler::think(self);
        // (core) ids of the things removed this tick go back.
        crate::ids::sweep(self);
        if self.trace.recording() {
            let digest = crate::trace::digest(self);
            self.trace.digests.push(digest);
        }
    }

    /// Route a command to the system that owns it. Refusals become events.
    pub fn apply(&mut self, command: Command) {
        let result = match &command {
            Command::Move { .. } | Command::Attack { .. } | Command::Stop { .. } => crate::movement::command(self, &command),
            Command::Recruit { .. } | Command::CancelRecruit { .. } => crate::production::command(self, &command),
            Command::Deploy { .. } | Command::Pack { .. } => crate::deploy::command(self, &command),
            Command::Research { kingdom, building, property } => crate::production::research(self, *kingdom, *building, property),
            Command::Attach { kingdom, org, hq } => crate::production::attach(self, *kingdom, *org, *hq),
            Command::SetFormation { .. } | Command::SetRallyPoint { .. } | Command::Sell { .. } | Command::Detach { .. } | Command::Disband { .. } => {
                crate::player_orders::command(self, &command)
            }
            // appended by the combat owner
            Command::UseAbility { .. } => crate::special_ops::command(self, &command),
            // appended by the movement owner
            Command::Follow { .. } => crate::movement::command(self, &command),
            // appended by the core owner: the HUD orders on the ability and
            // builder ports (crate::combat::tasks)
            Command::Bombard { .. } | Command::Repair { .. } | Command::Construct { .. } | Command::AirAttack { .. } | Command::AirRecon { .. } => {
                crate::combat::tasks::command(self, &command)
            }
        };
        if let Err(reason) = result {
            let kingdom = match command {
                Command::Move { kingdom, .. }
                | Command::Attack { kingdom, .. }
                | Command::Stop { kingdom, .. }
                | Command::Recruit { kingdom, .. }
                | Command::CancelRecruit { kingdom, .. }
                | Command::Deploy { kingdom, .. }
                | Command::Pack { kingdom, .. }
                | Command::Research { kingdom, .. }
                | Command::Attach { kingdom, .. }
                | Command::SetFormation { kingdom, .. }
                | Command::SetRallyPoint { kingdom, .. }
                | Command::Sell { kingdom, .. }
                | Command::Detach { kingdom, .. }
                | Command::Disband { kingdom, .. }
                | Command::UseAbility { kingdom, .. }
                | Command::Follow { kingdom, .. }
                | Command::Bombard { kingdom, .. }
                | Command::Repair { kingdom, .. }
                | Command::Construct { kingdom, .. }
                | Command::AirAttack { kingdom, .. }
                | Command::AirRecon { kingdom, .. } => kingdom,
            };
            self.events.push(crate::world::Event::Refused { kingdom, reason });
        }
    }
}
