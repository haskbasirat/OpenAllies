//! Retained76F240 scalar fields of the ONE Sim object. The native allocator
//! does not initialize +10..1C: those words remain absent until76DCE0 writes
//! them. This owner excludes the separately retained actual SyncA/SyncS and
//! callback-registration owners; it is not a full Session/startup certificate.
use crate::{
    catalog::Catalog,
    simulation::{self, Timeline},
};
use aa_numeric::{Binary, Precision};
use aa_simulation::FloatBits;

/// One fresh process/application's B40904/B40908 sync counters plus the Rust
/// failure policy latch. Counter zeroes are PE loader zero-tail values, not a
/// new-mission reset. The application moves this owner across loading/startup.
#[derive(Debug)]
pub struct FrameControl {
    counters: simulation::SyncCounters,
    stopped: bool,
}
impl FrameControl {
    pub(crate) fn fresh_application() -> Self {
        Self {
            counters: simulation::SyncCounters { client: 0, host: 0 },
            stopped: false,
        }
    }
    pub fn stopped(&self) -> bool {
        self.stopped
    }
    pub fn counters(&self) -> &simulation::SyncCounters {
        &self.counters
    }
    pub fn control_loan(&mut self) -> simulation::borrowed::Control<'_> {
        simulation::borrowed::Control {
            counters: &mut self.counters,
            stopped: &mut self.stopped,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeWords {
    pub begin_game: Option<FloatBits>,
    pub begin_admin: Option<FloatBits>,
    pub end_game: Option<FloatBits>,
    pub end_admin: Option<FloatBits>,
}
struct WorldAlias {
    revision: u32,
    identity: std::sync::Weak<()>,
}
/// No Clone/Default: constructor and all later writes retain the same owner.
pub struct Sim {
    times: TimeWords,
    paused: u8,
    frozen: u8,
    world_revision: Option<WorldAlias>,
    field_34: u32,
    avi_frames: i32,
    speed: FloatBits,
    maximum: FloatBits,
    rate: Option<FloatBits>,
    constructor_precision: Option<Precision>,
}
impl Sim {
    /// Known scalar assignments in76F240. Rate read/arithmetic is deliberately
    /// incomplete until the actual registered SVar and constructor FP input
    /// are supplied. No allocation-zero assumption fills the time words.
    pub fn from_native_constructor_fields() -> Self {
        Self {
            times: TimeWords {
                begin_game: None,
                begin_admin: None,
                end_game: None,
                end_admin: None,
            },
            paused: 0,
            frozen: 1,
            world_revision: None,
            field_34: 0,
            avi_frames: 0,
            speed: FloatBits::ZERO,
            maximum: FloatBits(simulation::MAX_SPEED_BITS),
            rate: None,
            constructor_precision: None,
        }
    }
    /// Bind the actual fresh registered AdminTicksPerSecond source; the FP
    /// input describes this constructor, independently of source/restore PC.
    pub fn bind_constructor_rate(
        &mut self,
        catalog: &Catalog<'_>,
        p: Precision,
    ) -> Result<(), String> {
        let rate = crate::load_host_settings::admin_ticks_per_second(catalog)?;
        self.complete_rate(rate, p)
    }
    fn complete_rate(&mut self, bits: u32, p: Precision) -> Result<(), String> {
        if self.rate.is_some() {
            return Err("Sim constructor rate was already produced".into());
        }
        let value = Binary::from_f32_bits(bits).map_err(|e| e.to_string())?;
        if value.compare(&Binary::from_i32(0)).is_le() {
            return Err("Sim constructor requires the proved finite positive rate domain".into());
        }
        let result = Binary::from_i32(1)
            .div(&value, p)
            .and_then(|v| v.store_f32())
            .map_err(|e| e.to_string())?;
        self.rate = Some(FloatBits(result));
        self.constructor_precision = Some(p);
        Ok(())
    }
    pub fn frozen(&self) -> bool {
        self.frozen != 0
    }
    pub fn frozen_byte(&self) -> u8 {
        self.frozen
    }
    pub fn paused_byte(&self) -> u8 {
        self.paused
    }
    ///773936's direct store after the actual ResetAdminTime call succeeds.
    ///This does not unfreeze+29 or certify application activation.
    pub fn clear_paused_on_world_success_773936(&mut self) {
        self.paused = 0;
    }
    pub fn world_is_null(&self) -> bool {
        self.world_revision.is_none()
    }
    pub fn bound_world_revision(&self) -> Option<u32> {
        self.world_revision.as_ref().map(|world| world.revision)
    }
    pub fn require_created_world(
        &self,
        world: &crate::world_revision::ConstructedWorldRevision,
    ) -> Result<(), String> {
        self.require_created_world_witness(&world.witness())
    }
    /// Validate the native nonowning Sim+3C alias against the SAME currently
    /// retained World allocation. A stale witness cannot be kept alive by Sim.
    pub fn require_created_world_witness(
        &self,
        world: &crate::world_revision::Witness,
    ) -> Result<(), String> {
        let live = world
            .owner_identity()
            .upgrade()
            .ok_or("actual World allocation for Sim+3C witness no longer exists")?;
        if self.world_revision.as_ref().is_some_and(|actual| {
            actual.revision == world.value()
                && actual
                    .identity
                    .upgrade()
                    .is_some_and(|identity| std::sync::Arc::ptr_eq(&identity, &live))
        }) {
            Ok(())
        } else {
            Err(
                "Sim+3C is not bound by the actual76AA90 constructor-return store for this World"
                    .into(),
            )
        }
    }
    pub fn time_words(&self) -> TimeWords {
        self.times
    }
    pub fn game_per_admin_tick(&self) -> Result<FloatBits, String> {
        self.rate
            .ok_or_else(|| "actual Sim constructor rate is unbound".into())
    }
    pub fn constructor_precision(&self) -> Option<Precision> {
        self.constructor_precision
    }

    /// Native final77D9D0 -> store+29 fragment. The actual caller owns the
    /// preceding diagnostic/assert/reset path. In fresh76EC70 it passes1,
    /// never unfreezes, and logs through the SAME retained synchronizer.
    /// A failing callback leaves the prior byte, including noncanonical bytes.
    pub fn store_frozen_after_sync<E>(
        &mut self,
        raw: u32,
        sync: impl FnOnce() -> Result<(), E>,
    ) -> Result<(), E> {
        sync()?;
        self.frozen = raw as u8;
        Ok(())
    }
    ///76DCE0 valid-GWorld finite arithmetic. Each read is live and occurs at
    /// its actual position; an error retains any earlier stores. No time word
    /// is consumed merely to initialize a temporary Timeline with zeroes.
    pub fn reset_admin_time_76dce0<E: std::fmt::Display>(
        &mut self,
        admin: FloatBits,
        p: Precision,
        mut world_begin: impl FnMut() -> Result<FloatBits, E>,
    ) -> Result<(), String> {
        let previous = Binary::from_f32_bits(admin.0)
            .and_then(|v| v.sub(&Binary::from_i32(1), p))
            .and_then(|v| v.store_f32())
            .map_err(|e| e.to_string())?;
        self.times.begin_game = Some(world_begin().map_err(|e| e.to_string())?);
        self.times.begin_admin = Some(FloatBits(previous));
        self.times.end_game = Some(world_begin().map_err(|e| e.to_string())?);
        self.times.end_admin = Some(admin);
        self.maximum = FloatBits(simulation::MAX_SPEED_BITS);
        Ok(())
    }
    /// Read-only kernel view, only after all four real stores and actual rate.
    /// The persistent live driver must borrow/move this owner, never keep this
    /// diagnostic value as a second mutable timeline.
    pub fn completed_timeline_view(&self) -> Result<Timeline, String> {
        let absent = || "actual Sim time producer76DCE0 is incomplete".to_string();
        Ok(Timeline {
            begin_game: self.times.begin_game.ok_or_else(absent)?,
            begin_admin: self.times.begin_admin.ok_or_else(absent)?,
            end_game: self.times.end_game.ok_or_else(absent)?,
            end_admin: self.times.end_admin.ok_or_else(absent)?,
            speed: self.speed,
            max_speed: self.maximum,
            paused: self.paused != 0,
            frozen: self.frozen != 0,
            game_per_admin_tick: self.game_per_admin_tick()?,
            avi_frames: self.avi_frames,
        })
    }
    pub fn constructor_field_34(&self) -> u32 {
        self.field_34
    }
}

impl crate::load_sim_world::WorldConstructorSink for Sim {
    fn store_world_76aa90(
        &mut self,
        world: &crate::world_revision::ConstructedWorldRevision,
    ) -> Result<(), String> {
        if self.world_revision.is_some() {
            return Err("fresh76AA90 binding requires the same constructor-null Sim owner; old World replacement/destruction is unported".into());
        }
        //76AACF already stored zero BEFORE allocation. In this fresh domain
        //it equals the real constructor0 and has no intervening writer.
        if self.field_34 != 0 {
            return Err("fresh Sim+34 constructor provenance differs".into());
        }
        self.world_revision = Some(WorldAlias {
            revision: world.value(),
            identity: std::sync::Arc::downgrade(world.owner_identity()),
        });
        Ok(())
    }
}

impl simulation::borrowed::SimFields for Sim {
    fn frozen(&self) -> bool {
        self.frozen != 0
    }
    fn frozen_for_log(&self) -> simulation::Result<bool> {
        if self.frozen > 1 {
            return Err(simulation::Error::Unsupported(
                "SyncLog canonical Bool cannot encode noncanonical Sim+29 byte",
            ));
        }
        Ok(self.frozen != 0)
    }
    fn paused(&self) -> bool {
        self.paused != 0
    }
    fn time(&self, f: simulation::borrowed::TimeWord) -> simulation::Result<FloatBits> {
        use simulation::borrowed::TimeWord::*;
        match f {
            BeginGame => self.times.begin_game,
            BeginAdmin => self.times.begin_admin,
            EndGame => self.times.end_game,
            EndAdmin => self.times.end_admin,
        }
        .ok_or(simulation::Error::Unsupported(
            "actual Sim time word76DCE0 producer is unbound",
        ))
    }
    fn store_time(
        &mut self,
        f: simulation::borrowed::TimeWord,
        v: FloatBits,
    ) -> simulation::Result<()> {
        use simulation::borrowed::TimeWord::*;
        match f {
            BeginGame => self.times.begin_game = Some(v),
            BeginAdmin => self.times.begin_admin = Some(v),
            EndGame => self.times.end_game = Some(v),
            EndAdmin => self.times.end_admin = Some(v),
        };
        Ok(())
    }
    fn game_per_admin_tick(&self) -> simulation::Result<FloatBits> {
        self.rate.ok_or(simulation::Error::Unsupported(
            "actual Sim constructor rate is unbound",
        ))
    }
    fn avi_frames(&self) -> i32 {
        self.avi_frames
    }
    fn store_avi_frames(&mut self, v: i32) {
        self.avi_frames = v
    }
    fn store_frozen(&mut self, v: bool) {
        self.frozen = u8::from(v)
    }
    fn store_maximum(&mut self, v: FloatBits) {
        self.maximum = v
    }
}

#[cfg(test)]
#[path = "live_sim_tests.rs"]
mod tests;
