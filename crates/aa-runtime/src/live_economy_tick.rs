//! EconomyComponent periodic Update on the live retained World.
//!
//! Economy+8 is its Updatable (vtable A3C668; queue1 only when the source
//! EconomyData supply_required (ActorData+348) holds, 56F250/leaf_init).
//! Periodic 50B330 calls +14 = 56F2B0 (arguments unused): a clear +348 logs
//! "stopping economy ticks" and removes queue1/data0 (50B1C0); otherwise
//! 4D7CE0 recomputes 4D0A80 (in supply) against Actor+100 bit 19 and, when
//! they agree, flips state 19 and refreshes (4D64F0(0x13, !supplied), 4D1350).
//! The body is aa_economy::supply::economy_component_tick. This module builds
//! its SupplyActor/KingdomSupply views from the SAME live owners:
//! * 4D0A80 reads Supply+8C (ActorData+424 relay), Element+10 (the regiment
//!   Actor, its ActorData+2F4 supply_used and +100 bit 19), Kingdom+190/+1F0,
//!   Properties+1C bit 5, Organization+7C (+D4, +D0 GTID via 44DEE0,
//!   ActorData+31D in_supply_detached, Actor+118/+128/+D8);
//! * a non-large actor (4EE700) asks 54F8B0(kingdom, actor) on its cached
//!   tile, i.e. union slot +38 (any_allied, mask 1, excluding the actor) on
//!   the retained ZoneMap tile set, whose leaf calls Actor/Kingdom virtual
//!   +138 (same owner, or relation code 1 through 4CF040/4E52D0).
//! A clear +348 retains the exact 404F60 arguments (Economy.cpp line 0xC6,
//! KKC_EconomyComponent::Tick, "actor %d stopping economy ticks", Actor+14)
//! and runs 50B1C0(1, 0) on this Updatable's own event list.
//! Unbound reached leaves (state 19 transition and its refresh, covered-tile
//! iteration of large actors, relay chains) return explicit errors.
use crate::{
    actor_world::{ActorStore, ObjectKey, RuntimeComponent, Source},
    catalog::Catalog,
    definitions::Bank,
    spatial_world::SpatialWorld,
};
use aa_actors::{ActorId, components::ComponentSlot as Slot};
use aa_economy::{
    EconomyError, F32Bits,
    supply::{KingdomSupply, SupplyActor, SupplyComponent, SupplyDefinition, SupplyWorld},
};
use aa_numeric::Precision;
use aa_world::queues::{Delivery, Error as QueueError, Owner, Queues};
use std::collections::BTreeMap;

fn error(e: impl std::fmt::Display) -> EconomyError {
    EconomyError::World(format!("live Economy 56F2B0: {e}"))
}
fn required<T>(leaf: &str) -> Result<T, EconomyError> {
    Err(error(format!("required native {leaf} is not bound")))
}
fn zerr(e: impl std::fmt::Display) -> aa_zones::Error {
    aa_zones::Error::World(format!("live Economy 54F8B0: {e}"))
}

pub(crate) struct Context<'a, 'source> {
    pub catalog: &'a Catalog<'source>,
    pub bank: &'a Bank,
    pub actors: &'a ActorStore,
    pub spatial: &'a SpatialWorld,
    pub zones: &'a aa_zones::tile::ZoneMap,
    pub source_precision: Precision,
}

/// Read-only access to the live owners shared by the supply view and the
/// 54F8B0 zone-set leaf.
struct Reads<'a, 'c, 'source> {
    cx: &'a Context<'c, 'source>,
}
impl Reads<'_, '_, '_> {
    fn key(&self, raw: u32) -> Result<ObjectKey, String> {
        self.cx.actors.key(ActorId::from_raw(raw)).ok_or_else(|| format!("{raw:#x} has no live GTID"))
    }
    fn object(&self, raw: u32) -> Result<&crate::actor_world::RuntimeObject, String> {
        self.cx.actors.object(self.key(raw)?).map_err(|e| e.to_string())
    }
    fn class(&self, raw: u32) -> Result<u32, String> {
        Ok(self.object(raw)?.definition.native_class)
    }
    fn word(&self, raw: u32, offset: u32) -> Result<u32, String> {
        self.object(raw)?
            .actor
            .as_ref()
            .ok_or("Actor shell absent")?
            .words
            .get(&offset)
            .copied()
            .ok_or_else(|| format!("Actor+{offset:X} is uninitialized"))
    }
    fn special(&self, raw: u32) -> Result<&aa_mapworld::special_state::SpecialShell, String> {
        let key = self.key(raw)?;
        let Source::Special(s) = &self.cx.actors.entity(key).map_err(|e| e.to_string())?.source else {
            return Err("special Kingdom/Team derived owner is not bound".into());
        };
        s.derived.as_ref().ok_or_else(|| "special derived fields absent".into())
    }
    fn kingdom_word(&self, raw: u32, offset: u32) -> Result<u32, String> {
        if self.class(raw)? != 4 {
            return Err("Actor+E8 is not an actual Kingdom allocation".into());
        }
        self.special(raw)?
            .words
            .get(&offset)
            .copied()
            .ok_or_else(|| format!("Kingdom+{offset:X} uninitialized"))
    }
    fn related_code(&self, actor: u32, other: u32) -> Result<u32, aa_zones::Error> {
        let other = (other != 0).then_some(ActorId::from_raw(other));
        match self.class(actor).map_err(zerr)? {
            3 => {
                let owner = self.word(actor, 0xe8).map_err(zerr)?;
                crate::relations::borrowed::actor(self, (owner != 0).then_some(ActorId::from_raw(owner)), other)
            }
            4 => crate::relations::borrowed::kingdom(self, ActorId::from_raw(actor), other),
            _ => Err(zerr("virtual +144 relation for a non-Actor/Kingdom class is not bound")),
        }
    }
    fn zone_kingdom(&self, raw: u32) -> Result<aa_zones::query::Kingdom, String> {
        Ok(aa_zones::query::Kingdom {
            key: raw,
            index: self.kingdom_word(raw, 0x18c)?,
            nonmajor: self.kingdom_word(raw, 0x190)?,
            shared_mask: self.kingdom_word(raw, 0x228)?,
            supply_mask: crate::load_detection_activation::kingdom_supply_mask(self.special(raw)?)
                .map_err(|e| e.to_string())?,
        })
    }
}
impl crate::relations::borrowed::Read for Reads<'_, '_, '_> {
    type Id = ActorId;
    type Error = aa_zones::Error;
    fn kingdom_team(&self, id: ActorId) -> aa_zones::Result<Option<ActorId>> {
        let s = self.special(id.raw()).map_err(zerr)?;
        match s.identity_references.get(&0x1ec) {
            Some(v) => Ok(*v),
            None if s.words.get(&0x1ec) == Some(&0) => Ok(None),
            _ => Err(zerr("Kingdom+1EC Team reference unbound")),
        }
    }
    fn kingdom_slot(&self, id: ActorId) -> aa_zones::Result<u32> {
        self.kingdom_word(id.raw(), 0x188).map_err(zerr)
    }
    fn team_slot(&self, id: ActorId) -> aa_zones::Result<u32> {
        self.special(id.raw())
            .map_err(zerr)?
            .words
            .get(&0x38)
            .copied()
            .ok_or_else(|| zerr("Team+38 absent"))
    }
    fn kingdom_word(&self, id: ActorId, team: bool, slot: u32) -> aa_zones::Result<u32> {
        self.special(id.raw())
            .map_err(zerr)?
            .vectors
            .get(&if team { 0x1dc } else { 0x1d0 })
            .and_then(|v| v.values.get(slot as usize))
            .copied()
            .ok_or_else(|| zerr("Kingdom relation vector slot absent"))
    }
    fn team_word(&self, id: ActorId, team: bool, slot: u32) -> aa_zones::Result<u32> {
        self.special(id.raw())
            .map_err(zerr)?
            .vectors
            .get(&if team { 0x58 } else { 0x4c })
            .and_then(|v| v.values.get(slot as usize))
            .copied()
            .ok_or_else(|| zerr("Team relation vector slot absent"))
    }
    fn orphan_relation(&self) -> aa_zones::Result<u32> {
        Err(zerr("global B28158 orphan relation is not bound"))
    }
    fn team_null_relation(&self) -> aa_zones::Result<u32> {
        Err(zerr("global B2815C team-null relation is not bound"))
    }
}
/// Union slot +38 leaf 514BB0 reads (mask/exclude tests precede the call).
impl aa_zones::query::ZoneActors for Reads<'_, '_, '_> {
    fn owner(&mut self, a: u32) -> aa_zones::Result<u32> {
        self.word(a, 0xe8).map_err(zerr)
    }
    fn allied(&mut self, a: u32, k: u32) -> aa_zones::Result<bool> {
        let same = match self.class(a).map_err(zerr)? {
            3 => self.word(a, 0xe8).map_err(zerr)? == k,
            4 => a == k,
            _ => return Err(zerr("virtual +138 for a non-Actor/Kingdom class is not bound")),
        };
        Ok(same || self.related_code(a, k)? == 1)
    }
    fn enemy(&mut self, _: u32, _: u32) -> aa_zones::Result<bool> {
        Err(zerr("slot +38 does not read virtual +140"))
    }
    fn zone_slot(&mut self, _: u32, _: u32) -> aa_zones::Result<u32> {
        Err(zerr("slot +38 does not read Actor+148"))
    }
    fn word4(&mut self, _: u32) -> aa_zones::Result<u32> {
        Err(zerr("slot +38 does not read Actor+4"))
    }
    fn component_b0(&mut self, _: u32) -> aa_zones::Result<u32> {
        Err(zerr("slot +38 does not read Actor+B0"))
    }
    fn reveal_mask(&mut self, _: u32) -> aa_zones::Result<Option<u32>> {
        Err(zerr("slot +38 does not read Detection+10"))
    }
}

struct Live<'a, 'c, 'source> {
    reads: Reads<'a, 'c, 'source>,
    views: BTreeMap<u32, SupplyActor>,
    queues: &'a mut Queues,
    owner: Owner,
    log: &'a mut Vec<crate::live_updates::NativeLog>,
}
impl<'a, 'c, 'source> Live<'a, 'c, 'source> {
    fn cx(&self) -> &'a Context<'c, 'source> {
        self.reads.cx
    }
    /// The SupplyActor view of one live actor: a read-only snapshot of the
    /// SAME fields (the kernel's only write path, 4D64F0, is unbound here).
    fn view(&self, raw: u32) -> Result<SupplyActor, EconomyError> {
        let cx = self.cx();
        let key = self.reads.key(raw).map_err(error)?;
        let object = cx.actors.object(key).map_err(error)?;
        let components = object.components().map_err(error)?;
        let id = &object.definition.proof.id;
        let placement = cx.actors.placement(key, cx.spatial).map_err(error)?;
        let supply = match components.get(Slot::Supply) {
            None => None,
            Some(RuntimeComponent::Supply(s)) => Some(SupplyComponent {
                radius: F32Bits(s.supply_range.0),
                dirty: s.byte_14 != 0,
                stockpile: F32Bits(s.stockpile.0),
                relay: s.word_1c,
            }),
            Some(_) => return Err(error("Supply slot payload mismatch")),
        };
        let supply_fields = match supply {
            Some(_) => Some(
                cx.catalog
                    .actors
                    .native_supply_fields(id, cx.source_precision)
                    .map_err(error)?
                    .ok_or_else(|| error("actual SupplyData absent"))?,
            ),
            None => None,
        };
        let economy_ticks = match components.get(Slot::Economy) {
            None => false,
            Some(RuntimeComponent::Economy(_)) => cx
                .catalog
                .actors
                .native_economy_inputs(id, cx.source_precision)
                .map_err(error)?
                .resources
                .as_ref()
                .ok_or_else(|| error("actual EconomyData+348 producer absent"))?
                .supply_required
                .boolean()
                .map_err(error)?,
            Some(_) => return Err(error("Economy slot payload mismatch")),
        };
        //Organization+D4/+D0 and ActorData+2F4/+31D (OrganizationComponent
        //source) are read only for an actor that owns an Organization.
        let (organization, gtid_d0, byte_31d, byte_2f4) = match components.get(Slot::Organization) {
            None => (None, 0, false, false),
            Some(RuntimeComponent::Organization(o)) => {
                let w = |off: u32| {
                    o.words.get(&off).copied().ok_or_else(|| error(format!("Organization+{off:X} unwritten")))
                };
                let data = cx
                    .catalog
                    .layout_data(cx.source_precision)
                    .map_err(error)?
                    .organization_component(cx.catalog.actors, id, cx.source_precision)
                    .map_err(error)?
                    .ok_or_else(|| error("actual OrganizationComponent source absent"))?;
                (Some(w(0xd4)? != 0), w(0xd0)?, data.in_supply_detached, data.supply_used)
            }
            Some(_) => return Err(error("Organization slot payload mismatch")),
        };
        let element = match components.get(Slot::Element) {
            None => None,
            Some(RuntimeComponent::Element(e)) => Some(e.organization.map(ActorId::raw)),
            Some(_) => return Err(error("Element slot payload mismatch")),
        };
        let properties_flag20 = match components.get(Slot::Properties) {
            None => None,
            Some(RuntimeComponent::Properties(p)) => Some(p.modifier.flags & 0x20 != 0),
            Some(_) => return Err(error("Properties slot payload mismatch")),
        };
        let city_flag20 = match components.get(Slot::City) {
            None => None,
            Some(RuntimeComponent::City(c)) => Some(c.byte_20 != 0),
            Some(_) => return Err(error("City slot payload mismatch")),
        };
        let (word_118, word_128) = if organization.is_some() {
            (self.reads.word(raw, 0x118).map_err(error)? != 0, self.reads.word(raw, 0x128).map_err(error)? != 0)
        } else {
            (false, false)
        };
        let kingdom = self.reads.word(raw, 0xe8).map_err(error)?;
        let tile = placement.cached_tile.ok_or_else(|| error("actual cached tile absent"))?;
        let zone_word8 = object
            .actor
            .as_ref()
            .and_then(|a| a.zones_148.zones().slots.first().copied().flatten())
            .map(|z| F32Bits(z.radius_bits));
        Ok(SupplyActor {
            gtid: raw,
            flags8: placement.flags,
            states: u64::from(components.state.low) | (u64::from(components.state.high) << 32),
            definition: SupplyDefinition {
                relay: supply_fields.as_ref().is_some_and(|s| s.is_relay),
                stockpile_max: F32Bits(supply_fields.as_ref().and_then(|s| s.stockpile_max_bits).unwrap_or(0)),
                economy_ticks,
                byte_31d,
                byte_2f4,
            },
            supply,
            element,
            organization_component: organization,
            properties_flag20,
            city_flag20,
            word_118,
            word_128,
            word_d8: components.present(Slot::Hq),
            gtid_d0,
            kingdom: (kingdom != 0).then_some(kingdom),
            position: [F32Bits(placement.position_bits[0]), F32Bits(placement.position_bits[1])],
            tile: [tile[0] as u32, tile[1] as u32],
            zone_word8,
        })
    }
}
impl SupplyWorld for Live<'_, '_, '_> {
    fn actor(&self, actor: u32) -> Result<&SupplyActor, EconomyError> {
        self.views.get(&actor).ok_or_else(|| error(format!("supply view of {actor:#x} was not bound")))
    }
    fn actor_mut(&mut self, _: u32) -> Result<&mut SupplyActor, EconomyError> {
        required("supply view mutation")
    }
    fn resolve(&self, gtid: u32) -> Option<u32> {
        self.cx().actors.key(ActorId::from_raw(gtid)).map(|_| gtid)
    }
    fn kingdom(&self, kingdom: u32) -> Result<KingdomSupply, EconomyError> {
        Ok(KingdomSupply {
            word_190: self.reads.kingdom_word(kingdom, 0x190).map_err(error)?,
            shortage_ratio: F32Bits(self.reads.kingdom_word(kingdom, 0x1f0).map_err(error)?),
        })
    }
    fn owner(&mut self, actor: u32) -> Result<Option<u32>, EconomyError> {
        let w = self.reads.word(actor, 0xe8).map_err(error)?;
        Ok((w != 0).then_some(w))
    }
    fn world_suspended(&mut self) -> Result<bool, EconomyError> {
        required("4F0460 in the Economy tick")
    }
    fn provide_under_siege(&self) -> bool {
        false
    }
    fn recharge_rate(&self) -> F32Bits {
        F32Bits(0)
    }
    fn map_width(&self) -> u32 {
        self.cx().zones.geometry.width as u32
    }
    fn large_actor(&mut self, actor: u32) -> Result<bool, EconomyError> {
        //4EE700: Thing+8 0x20000400 and the blocking shape against the tile.
        let cx = self.cx();
        let key = self.reads.key(actor).map_err(error)?;
        let flags = cx.actors.placement(key, cx.spatial).map_err(error)?.flags;
        if flags & 0x2000_0400 != 0x2000_0400 {
            return Ok(false);
        }
        let d = &cx.actors.object(key).map_err(error)?.definition;
        let s = crate::live_object_init::Source::bind_in_bank(cx.catalog, cx.bank, d).map_err(error)?;
        aa_spatial::geometry::marks_blocker(
            flags,
            s.blocking_shape().radius_bits,
            cx.spatial.index().grid().cell_size_bits,
            cx.spatial.index().precision(),
        )
        .map_err(error)
    }
    fn tile_supplied(&mut self, tile: u32, kingdom: Option<u32>, actor: u32) -> Result<bool, EconomyError> {
        let zones = self.cx().zones;
        let t = zones.tile(tile as usize).map_err(error)?;
        let k = match kingdom {
            Some(k) => Some(self.reads.zone_kingdom(k).map_err(error)?),
            None => None,
        };
        aa_zones::query::tile_friendly_supply(&zones.pool, t.set, t.supply, &mut self.reads, k.as_ref(), actor)
            .map_err(error)
    }
    fn covered_tiles(&mut self, _: u32) -> Result<Vec<u32>, EconomyError> {
        required("4CF210/554DD0/554330 large-actor covered-tile iteration")
    }
    fn relay_candidates(&mut self, _: u32, _: Option<u32>) -> Result<Vec<u32>, EconomyError> {
        required("relay candidate collection")
    }
    fn set_state(&mut self, actor: u32, state: u32, enabled: bool) -> Result<(), EconomyError> {
        Err(error(format!(
            "required native 4D64F0 state {state:#x} transition ({enabled}) for {actor:#x} is not bound"
        )))
    }
    fn refresh_actor(&mut self, _: u32) -> Result<(), EconomyError> {
        required("4D1350 actor refresh")
    }
    fn refresh_economy(&mut self, _: u32) -> Result<(), EconomyError> {
        required("4CF0F0 Economy refresh")
    }
    fn stop_economy_ticks(&mut self, actor: u32) -> Result<(), EconomyError> {
        //56F2C5: [Economy+4]+14 GTID, then 404F60(B38B70, file, 0xC6, ...).
        self.log.push(crate::live_updates::NativeLog {
            level: 10,
            file: r".\Source\Sim\Component\Economy.cpp",
            line: 0xc6,
            function: "KKC_EconomyComponent::Tick",
            format: "actor %d stopping economy ticks",
            arguments: vec![actor],
        });
        self.queues.remove_matching(self.owner, 1, 0).map(|_| ()).map_err(error)
    }
    fn clear_zone(&mut self, _: u32) -> Result<(), EconomyError> {
        required("514CE0 supply zone release")
    }
    fn set_zone(&mut self, _: u32, _: [F32Bits; 2], _: F32Bits) -> Result<(), EconomyError> {
        required("515F40 supply zone move")
    }
}

/// One queue1 periodic delivery to Economy+8 (+14 = 56F2B0).
pub(crate) fn run(
    key: ObjectKey,
    event: Delivery,
    cx: &Context<'_, '_>,
    queues: &mut Queues,
    log: &mut Vec<crate::live_updates::NativeLog>,
) -> aa_world::queues::Result<()> {
    let fail = |e: EconomyError| QueueError::Callback(e.to_string());
    if event.elapsed_bits.is_none() {
        return Err(QueueError::Callback("live Economy: +18/+1C are 943800 RET4".into()));
    }
    let id = cx.actors.id(key).ok_or_else(|| fail(error("owner has no live GTID")))?;
    match cx.actors.component(id, Slot::Economy).map_err(|e| fail(error(e)))? {
        Some(RuntimeComponent::Economy(e)) if e.header.owner == id => {}
        _ => return Err(fail(error("scheduled Economy allocation/owner differs"))),
    }
    let mut live = Live { reads: Reads { cx }, views: BTreeMap::new(), queues, owner: event.owner, log };
    let view = live.view(id.raw()).map_err(fail)?;
    if let Some(Some(org)) = view.element {
        let org_view = live.view(org).map_err(fail)?;
        live.views.insert(org, org_view);
    }
    live.views.insert(id.raw(), view);
    aa_economy::supply::economy_component_tick(&mut live, id.raw()).map_err(fail)
}
