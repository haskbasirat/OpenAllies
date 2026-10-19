//! ConstructionComponent periodic Update on the live retained World.
//!
//! Construction+8 is its Updatable (vtable A3B924, queue1). Periodic 50B330
//! calls +14 = 566880 (both arguments unused). The body is
//! aa_production::construction::construction_tick (native order: owner
//! Kingdom virtual +108 = Actor+E8 and its +239 dissolving byte with action
//! bits 0x16/0x14 via virtual +34 = 4D1F90 on Actor+F0/F4; 4D6660; 4CF110 =
//! Properties(+78)+1C modifier token, 501570 interval token+30; ActorData+4C0
//! upgrade list; World+E0 - Body(+60)+20 against SVars+1CC
//! ActiveConstructionTimeout for state 0x20 via 4D64F0).
//! Leaves that change the world beyond the bound path (sell 565A20, disable
//! 564DC0, auto-upgrade 566360, a state 0x20 transition) return explicit
//! errors at their call.
use crate::{
    actor_world::{ActorStore, ObjectKey, RuntimeComponent, Source, SpecialSource},
    spatial_world::SpatialWorld,
};
use aa_actors::{ActorId, components::ComponentSlot};
use aa_economy::X87Precision;
use aa_numeric::Precision;
use aa_production::{
    Error, Result,
    construction::{Construction, ConstructionLifecycle, ConstructionPayments, FpuPrecision},
};
use aa_simulation::NativeClock;
use aa_world::queues::{Delivery, Error as QueueError};

fn fail(e: impl std::fmt::Display) -> Error {
    Error::World(format!("live Construction 566880: {e}"))
}
fn required<T>(leaf: &str) -> Result<T> {
    Err(fail(format!("required native {leaf} is not bound")))
}

pub(crate) struct Context<'a> {
    pub actors: &'a mut ActorStore,
    pub spatial: &'a SpatialWorld,
    pub variables: &'a crate::load_view_variables::State,
    pub clock: &'a NativeClock,
    pub precision: Precision,
}

struct Live<'a> {
    actors: &'a ActorStore,
    spatial: &'a SpatialWorld,
    variables: &'a crate::load_view_variables::State,
    clock: &'a NativeClock,
    precision: Precision,
    key: ObjectKey,
    id: ActorId,
}
impl Live<'_> {
    fn object(&self) -> Result<&crate::actor_world::RuntimeObject> {
        self.actors.object(self.key).map_err(fail)
    }
    fn word(&self, offset: u32) -> Result<u32> {
        self.object()?
            .actor
            .as_ref()
            .ok_or_else(|| fail("Actor shell absent"))?
            .words
            .get(&offset)
            .copied()
            .ok_or_else(|| fail(format!("Actor+{offset:X} is uninitialized")))
    }
    fn state(&self) -> Result<u64> {
        let s = self.object()?.components().map_err(fail)?.state;
        Ok(u64::from(s.low) | (u64::from(s.high) << 32))
    }
    fn svar_fixed(&self, name: &str) -> Result<u32> {
        let ordinal = self
            .variables
            .registrations()
            .find(|r| r.name() == name)
            .map(|r| r.ordinal())
            .ok_or_else(|| fail(format!("SVar {name} is not registered")))?;
        match self.variables.current(ordinal as usize).map_err(fail)? {
            crate::load_view_variables::CurrentValue::Fixed(v) => Ok(*v),
            _ => Err(fail(format!("SVar {name} is not the float kind"))),
        }
    }
    fn modifier(&self) -> Result<Option<&aa_body::properties::ModifierState>> {
        match self.object()?.components().map_err(fail)?.get(ComponentSlot::Properties) {
            None => Ok(None),
            Some(RuntimeComponent::Properties(v)) => Ok(Some(&v.modifier)),
            Some(_) => Err(fail("Properties slot payload mismatch")),
        }
    }
}
impl FpuPrecision for Live<'_> {
    fn precision(&self) -> X87Precision {
        match self.precision {
            Precision::Single24 => X87Precision::Single24,
            Precision::Double53 => X87Precision::Double53,
            Precision::Extended64 => X87Precision::Extended64,
        }
    }
}
impl Construction for Live<'_> {
    fn construction_kingdom(&mut self) -> Result<Option<u32>> {
        //Virtual +108 = 4D1E80: Actor+E8.
        let owner = self.word(0xe8)?;
        Ok((owner != 0).then_some(owner))
    }
    fn construction_cost(&mut self, _: u32) -> Result<Vec<u32>> {
        required("45F4D0 construction cost")
    }
    fn construction_can_afford(&mut self, _: u32, _: &[u32]) -> Result<bool> {
        required("Kingdom affordability")
    }
    fn construction_debit(&mut self, _: u32, _: &[u32]) -> Result<()> {
        required("Kingdom debit")
    }
    fn construction_credit(&mut self, _: u32, _: &[u32]) -> Result<()> {
        required("Kingdom credit")
    }
    fn notify_construct_unaffordable(&mut self, _: u32, _: &[u32], _: u32) -> Result<()> {
        required("construct_error_affording message")
    }
    fn notify_upgrade_unaffordable(&mut self, _: u32, _: &[u32], _: Option<u32>) -> Result<()> {
        required("construct_error_affording_upgrade message")
    }
    fn has_structure(&mut self) -> Result<bool> {
        Ok(self.object()?.components().map_err(fail)?.present(ComponentSlot::Structure))
    }
    fn state_active(&mut self, state: u32) -> Result<bool> {
        //Virtual +34 = 4D1F90: (1 << state) & Actor+F0/F4 action bits.
        let low = u64::from(self.word(0xf0)?);
        let high = u64::from(self.word(0xf4)?);
        let bit = 1u64.checked_shl(state).unwrap_or(0);
        Ok((low | (high << 32)) & bit != 0)
    }
    fn actor_flags(&mut self) -> Result<u64> {
        self.state()
    }
    fn definition_flags(&mut self, _: u32) -> Result<u32> {
        required("ActorData+174 upgrade definition flags")
    }
    fn kingdom_allows(&mut self, _: u32, _: u32) -> Result<bool> {
        required("4E1910 Kingdom restrictions")
    }
    fn upgrade_list(&mut self) -> Result<Vec<u32>> {
        required("ActorData+4C0 automatic upgrade list")
    }
    fn construction_begun(&mut self, _: Option<u32>, _: u32) -> Result<()> {
        required("54BFC0 construction-begun trigger")
    }
    fn apply_upgrade_modifiers(&mut self, _: u32) -> Result<()> {
        required("561D40 upgrade modifiers")
    }
    fn construction_actor_state(&mut self, state: u32, enabled: bool) -> Result<()> {
        //4D64F0 returns at once when the bit already has the requested value.
        let bit = 1u64.checked_shl(state).ok_or_else(|| fail("state index outside 64 bits"))?;
        if (self.state()? & bit != 0) == enabled {
            return Ok(());
        }
        Err(fail(format!(
            "required native 4D64F0 state {state:#x} transition ({enabled}) for {:#x} is not bound",
            self.id.raw()
        )))
    }
    fn destroyed(&mut self) -> Result<bool> {
        Ok(self.actors.placement(self.key, self.spatial).map_err(fail)?.flags & 1 != 0)
    }
    fn destroy_actor(&mut self) -> Result<()> {
        required("4F6760 removal request")
    }
}
impl ConstructionLifecycle for Live<'_> {
    fn salvage_fraction(&mut self) -> Result<u32> {
        self.svar_fixed("SellFactor")
    }
    fn notify_sold(&mut self, _: &[u32], _: Option<u32>) -> Result<()> {
        required("42D490 event 0x17 structure sold")
    }
    fn enable_structure(&mut self) -> Result<()> {
        required("565400 enable structure")
    }
    fn disable_structure(&mut self) -> Result<()> {
        required("564DC0 disable structure")
    }
    fn kingdom_dissolving(&mut self, kingdom: u32) -> Result<bool> {
        let key = self
            .actors
            .key(ActorId::from_raw(kingdom))
            .ok_or_else(|| fail("owner Kingdom has no live GTID"))?;
        let shell = match &self.actors.entity(key).map_err(fail)?.source {
            Source::Special(s) if matches!(s.source, SpecialSource::Kingdom(_)) => s
                .derived
                .as_ref()
                .ok_or_else(|| fail("Kingdom derived state absent"))?,
            _ => return Err(fail("Actor+E8 is not an actual Kingdom allocation")),
        };
        //Kingdom+239: byte 1 of the constructor DWORD at +238.
        if let Some(v) = shell.bytes.get(&0x239) {
            return Ok(*v != 0);
        }
        let word = shell
            .words
            .get(&0x238)
            .copied()
            .ok_or_else(|| fail("Kingdom+238 is uninitialized"))?;
        Ok((word >> 8) as u8 != 0)
    }
    fn refresh_actor(&mut self) -> Result<()> {
        crate::live_updates::siege::refresh(
            self.actors,
            self.spatial,
            self.variables,
            aa_body::F32(self.clock.begin.0),
            self.id,
            self.precision,
        )
        .map_err(fail)
    }
    fn auto_upgrade_token(&mut self) -> Result<Option<u32>> {
        //4CF110: Properties+1C (the ModifierState); a token value only names it.
        Ok(self.modifier()?.map(|_| 0x1c))
    }
    fn auto_upgrade_interval(&mut self, _: u32) -> Result<u32> {
        //ModifierState+30 is scalars[(0x30 - 4) / 4].
        Ok(self
            .modifier()?
            .ok_or_else(|| fail("4CF110 token without Properties"))?
            .scalars[11]
            .0)
    }
    fn actor_time(&mut self) -> Result<u32> {
        required("Actor+1C time stamp")
    }
    fn world_time(&mut self) -> Result<u32> {
        Ok(self.clock.begin.0)
    }
    fn base_cost(&mut self, _: u32) -> Result<Vec<u32>> {
        required("45F4D0 base cost")
    }
    fn stockpiled_resources(&mut self) -> Result<Vec<bool>> {
        required("Resource registry stockpiled rows")
    }
    fn log_auto_upgrade_cost(&mut self, _: u32) -> Result<()> {
        required("404ED0 auto-upgrade diagnostic")
    }
    fn construction_start_time(&mut self) -> Result<u32> {
        //566A45 dereferences Actor+60 unconditionally.
        match self.object()?.components().map_err(fail)?.get(ComponentSlot::Body) {
            Some(RuntimeComponent::Body(v)) => Ok(v.body.last_heal_time.0),
            Some(_) => Err(fail("Body slot payload mismatch")),
            None => Err(fail("566A45 null Actor+60 Body dereference")),
        }
    }
    fn construction_window(&mut self) -> Result<u32> {
        self.svar_fixed("ActiveConstructionTimeout")
    }
}

/// One queue1 periodic delivery to Construction+8 (+14 = 566880).
pub(crate) fn run(key: ObjectKey, event: Delivery, cx: &mut Context<'_>) -> aa_world::queues::Result<()> {
    let queue_fail = |e: Error| QueueError::Callback(e.to_string());
    if event.elapsed_bits.is_none() {
        return Err(QueueError::Callback(
            "live Construction: +18/+1C are 943800 RET4; deferred delivery is invalid".into(),
        ));
    }
    let id = cx
        .actors
        .id(key)
        .ok_or_else(|| QueueError::Callback("live Construction: owner has no live GTID".into()))?;
    let mut payments = match cx.actors.component(id, ComponentSlot::Construction).map_err(|e| queue_fail(fail(e)))? {
        Some(RuntimeComponent::Construction(v)) if v.header.owner == id => ConstructionPayments {
            paid: v.words_10.clone(),
            upgrade_paid: v.words_1c.clone(),
            upgrade: v.definition_28.unwrap_or(0),
        },
        _ => return Err(queue_fail(fail("scheduled Construction allocation/owner differs"))),
    };
    let before = payments.clone();
    let result = {
        let mut live = Live {
            actors: &*cx.actors,
            spatial: cx.spatial,
            variables: cx.variables,
            clock: cx.clock,
            precision: cx.precision,
            key,
            id,
        };
        aa_production::construction::construction_tick(&mut payments, &mut live)
    };
    //The bound path never writes the payment vectors; any write would have
    //come from an unbound leaf, which already returned its error.
    if payments != before {
        return Err(queue_fail(fail("Construction payments changed on an unbound path")));
    }
    result.map_err(queue_fail)
}
