//! One battle tick, in the original's system order as far as it is known
//! (commands, AI, economy, production, movement, combat, morale, supply,
//! victory). Shared file: change only by agreement.
use crate::world::{Command, World};

impl World {
    /// Advance one tick (1/16 s), applying `commands` first.
    pub fn step(&mut self, commands: Vec<Command>) {
        self.events.clear();
        for command in commands {
            self.apply(command);
        }
        let ai_commands = crate::ai::think(self);
        for command in ai_commands {
            self.apply(command);
        }
        crate::economy::tick(self);
        crate::production::tick(self);
        crate::deploy::tick(self);
        crate::movement::tick(self);
        crate::combat::tick(self);
        crate::morale::tick(self);
        crate::supply::tick(self);
        crate::victory::tick(self);
        // appended by the UI owner: rally points of this tick's recruits,
        // then the detection zones and fog after the actors moved
        crate::player_orders::tick(self);
        crate::fog::tick(self);
        self.tick += 1;
        self.time = self.tick as f32 * crate::DT;
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
                | Command::Disband { kingdom, .. } => kingdom,
            };
            self.events.push(crate::world::Event::Refused { kingdom, reason });
        }
    }
}
