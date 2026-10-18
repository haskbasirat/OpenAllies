//! GActor::Init(NULL) on the real held saved allocation, before publication.
//!
//! Reuses the proved native coordinator and the existing held build-target and
//! action-mask refreshes. Source fields come from the completed source Bank's
//! PC, independently of the live arithmetic PC. Unbound reached services fail;
//! an absent component is read from actual storage, never a supplied shortcut.
use crate::{catalog::Catalog, live_object_init::Record};
use aa_actors::{
    ActorId,
    actor_creation::{Error, Result},
    actor_init_null,
    components::ComponentSlot,
};
use aa_mapworld::restore::{ActorPropertiesContext, RestoredRecord};
use aa_numeric::Precision;
use aa_rules::native::{TypedValue, object_definition::TrackInfoInputs};
use std::cell::RefCell;

#[cfg(test)]
#[path = "load_actor_init_world_tests.rs"]
mod tests;

fn required(name: &'static str) -> Error {
    Error::Unsupported(name)
}
fn world(value: impl std::fmt::Display) -> Error {
    Error::World(value.to_string())
}

/// Required owned-world services. Defaults are explicit failures at the reached
/// native address, allowing the persistent owner to bind each real leaf without
/// pretending that unsupported branches ran successfully.
pub trait Services {
    fn object_init_null(&mut self, record: &mut Record<'_>, pc: Precision) -> Result<bool>;
    fn mind_init_existing(&mut self, _: &mut Record<'_>, _: Precision) -> Result<bool> {
        Err(required(
            "Mind InitExisting requires actual inventory and owned state effects",
        ))
    }
    fn scheduled_component_init_existing(
        &mut self,
        _: &mut Record<'_>,
        _: ComponentSlot,
        _: Precision,
    ) -> Result<bool> {
        Err(required(
            "564D80 actual shared World version/queue/head/RNG services",
        ))
    }
    fn detection_after_init(&mut self, _: &mut Record<'_>) -> Result<()> {
        Err(required(
            "56EC90 actual held Detection/zone activation services",
        ))
    }
    fn economy_init_existing(&mut self, _: &mut Record<'_>, _: Precision) -> Result<bool> {
        Err(required("56F280 actual Economy Init load queue/RNG owner"))
    }
    fn required_object_init(&mut self, _: &mut Record<'_>) -> Result<bool> {
        Err(required(
            "4D1FD0 nonnull ActorData+4E0 required-object service",
        ))
    }
    fn required_object_attach(&mut self, _: &mut Record<'_>, _: ActorId) -> Result<()> {
        Err(required("required-object virtual128 attachment"))
    }
    fn refresh_4d67f0(&mut self, _: &mut Record<'_>) -> Result<()> {
        Err(required("4D67F0 non-bit29 owned settlement/FX refresh"))
    }
    fn refresh_4d1350(&mut self, record:&mut Record<'_>, _:Precision)->Result<()> {
        rebuild_actions(record)
    }
    /// Actual shared GWorld+2C, read lazily only by a reached native branch.
    fn world_version(&self) -> Result<u32> {
        Err(required("Mover Init actual shared GWorld+2C version"))
    }
    fn global_property(&self) -> Result<Option<u32>> {
        Err(required(
            "actual GameData+464/+24 global Property reference",
        ))
    }
    fn property_applies(&mut self, _: &mut Record<'_>, _: u32) -> Result<bool> {
        Err(required("global Property eligibility"))
    }
    fn properties_add(&mut self, _: &mut Record<'_>, _: u32) -> Result<()> {
        Err(required("global PropertiesComponent addition"))
    }
    fn refresh_4d4130(&mut self, _: &mut Record<'_>) -> Result<()> {
        Err(required("4D4130 non-bit29 owned actor refresh"))
    }
    fn organization_refresh(&mut self, _: &mut Record<'_>) -> Result<()> {
        Err(required("Actor Init Organization refresh"))
    }
    fn settlement_refresh(&mut self, _: &mut Record<'_>) -> Result<()> {
        Err(required("Actor Init Settlement refresh"))
    }
    fn performer_refresh(&mut self, _: &mut Record<'_>) -> Result<()> {
        Err(required("Actor Init Performer refresh"))
    }
    fn denizen_refresh(&mut self, _: &mut Record<'_>) -> Result<()> {
        Err(required("Actor Init Denizen refresh"))
    }
    fn create_zone2(&mut self, _: &mut Record<'_>) -> Result<bool> {
        Err(required("4D45B0 actual owned actor zone2 creation"))
    }
    fn discovery_radius(&mut self, _: Precision) -> Result<u32> {
        Err(required("4BB000 actual GameData discovery radius"))
    }
    fn zone2(&self, _: &Record<'_>) -> Result<Option<u32>> {
        Err(required("actual Actor+148/+14C zone2 owner"))
    }
    fn move_zone(&mut self, _: u32, _: [u32; 2], _: u32) -> Result<()> {
        Err(required("515F40 actual owned zone movement"))
    }
    fn track_scale_bits(&self) -> Result<u32> {
        Err(required("actual GameData TrackInfo scale"))
    }
    fn track_divisor(&self) -> Result<i32> {
        Err(required("actual GameData TrackInfo divisor"))
    }
    fn allocate_track(&mut self) -> Result<Option<u32>> {
        Err(required(
            "6F7810 actual owned TrackInfo updatable allocation",
        ))
    }
    fn construct_track(&mut self, _: u32, _: ActorId, _: i32) -> Result<u32> {
        Err(required("actual TrackInfo updatable construction"))
    }
    fn existing_fx(&mut self, _: &mut Record<'_>) -> Result<()> {
        Err(required(
            "42A590 actual current ActorData+E4 existing FX list",
        ))
    }
}

struct Bound<'a, 'r, 'source, S> {
    catalog: &'a Catalog<'source>,
    record: &'a mut Record<'r>,
    services: &'a mut S,
    source_pc: Precision,
    live_pc: Precision,
    tracks: RefCell<Option<(u32, Vec<TrackInfoInputs>)>>,
}

fn actor_word(record: &Record<'_>, offset: u32) -> Result<u32> {
    record
        .shell
        .actor
        .as_ref()
        .ok_or(Error::NullDereference("Actor Init held actor shell"))?
        .words
        .get(&offset)
        .copied()
        .ok_or_else(|| world(format!("Actor Init actual field +{offset:x} absent")))
}

/// Actual native early return of both 4D67F0 and 4D4130, before 4D10A0.
pub fn refresh_bit29_returns(flags: u32) -> bool {
    flags & (1 << 29) != 0
}

/// 4D1350 uses the same actual component arena and F0/F4 words as Load. The
/// existing generic action kernel preserves clear/partial writes before a
/// reached unbound contributor/publication error. No Organization is invented.
fn rebuild_actions(record: &mut Record<'_>) -> Result<()> {
    let actor = record
        .shell
        .actor
        .as_mut()
        .ok_or(Error::NullDereference("Actor Init action shell"))?;
    // Existing publication reads actual +E8 through the native 4D1E80 leaf;
    // both admitted class3/class4 vtables use that same leaf. Refuse absent
    // constructor storage rather than allowing the older helper's optional get.
    if !actor.words.contains_key(&0xe8) {
        return Err(required("Actor Init actual initialized Actor+E8"));
    }
    let object = record
        .shell
        .object
        .as_ref()
        .ok_or(Error::NullDereference("Actor Init action Object shell"))?;
    crate::actor_world::rebuild_saved_construction_actions(
        record.owner,
        &actor.components,
        &mut record.shell.thing.flags,
        &mut actor.words,
        crate::actor_world::SavedActionContext {
            // These optional sources are unbound, not successful defaults.
            // They are queried only by a present Construction contributor.
            construction: None,
            allow_sell_under_siege: None,
            object_byte_4c: object.byte_4c,
            thing_word_c: record.shell.thing.word_c,
        },
    )
    .map_err(world)
}

pub(crate) fn validate(catalog: &Catalog<'_>, record: &Record<'_>) -> Result<Precision> {
    catalog.require_source_owner().map_err(world)?;
    if !record
        .registry
        .get(record.owner)
        .is_some_and(|v| matches!(v.value, RestoredRecord::Loading))
    {
        return Err(required(
            "Actor Init requires the actual held Loading allocation",
        ));
    }
    if !matches!(record.definition.native_class, 3 | 4)
        || record.definition.proof.category.runtime_class != record.definition.native_class
        || !crate::load_spatial::ordinary_layout(
            record.definition.native_class,
            record.shell.kind,
            record.shell.object.is_some(),
            record.shell.actor.is_some(),
        )
        || !crate::load_spatial::same_proof(
            &record.shell.thing.definition,
            &record.definition.proof,
        )
    {
        return Err(required("Actor Init held constructor layout/proof differs"));
    }
    let pc = {
        let bank = catalog.definition_bank.try_borrow().map_err(world)?;
        let source = bank.combat_source().map_err(world)?;
        if source.source_identities() != catalog.actors.sources.as_slice() {
            return Err(required("Actor Init source FP owner differs from Catalog"));
        }
        source.source_precision()
    };
    let id = &record.definition.proof.id;
    catalog
        .require_untranslated_actor_identity(id)
        .map_err(world)?;
    let source = catalog
        .actors
        .find(id)
        .ok_or(required("Actor Init canonical source absent"))?;
    if source.source != record.definition.proof.source
        || source.template_chain != record.definition.proof.template_chain
        || catalog.actors.native_category(id).map_err(world)? != record.definition.proof.category
        || catalog.actors.native_final_flags(id, pc).map_err(world)? != record.definition.flags
        || record.definition.component_mask
            != Some(catalog.canonical_component_mask(id, pc).map_err(world)?)
    {
        return Err(required(
            "Actor Init canonical source/flags/component mask differs",
        ));
    }
    Ok(pc)
}

pub fn initialize(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    pc: Precision,
    services: &mut impl Services,
) -> Result<bool> {
    let source_pc = validate(catalog, record)?;
    let owner = record.owner;
    actor_init_null::initialize(
        &mut Bound {
            catalog,
            record,
            services,
            source_pc,
            live_pc:pc,
            tracks: RefCell::new(None),
        },
        owner,
        pc,
    )
}

impl<S: Services> Bound<'_, '_, '_, S> {
    fn owner(&self, actor: ActorId) -> Result<()> {
        if actor == self.record.owner {
            Ok(())
        } else {
            Err(required("Actor Init attempted a different held allocation"))
        }
    }
    fn source_field(&self, name: &str) -> Result<Option<TypedValue>> {
        self.catalog
            .actors
            .native_field(&self.record.definition.proof.id, name, self.source_pc)
            .map_err(world)
    }
    fn tracks(&self, token: u32) -> Result<std::cell::Ref<'_, Vec<TrackInfoInputs>>> {
        let tracks = self.tracks.borrow();
        if tracks.as_ref().is_none_or(|v| v.0 != token) {
            return Err(required(
                "Actor Init captured TrackInfo definition identity differs",
            ));
        }
        Ok(std::cell::Ref::map(tracks, |v| {
            &v.as_ref().expect("checked captured tracks").1
        }))
    }
}

impl<S: Services> actor_init_null::World for Bound<'_, '_, '_, S> {
    type Definition = u32;
    type Track = usize;
    type Zone = u32;
    type Updatable = u32;
    fn object_init_null(&mut self, a: ActorId, pc: Precision) -> Result<bool> {
        self.owner(a)?;
        self.services.object_init_null(self.record, pc)
    }
    fn required_object_init(&mut self, a: ActorId) -> Result<bool> {
        self.owner(a)?;
        let ids = self.source_field("required_object_ids")?;
        // Same source +4E0 producer as native_actor_creation_inputs; the
        // constructor/deferred-ref path leaves null for absent/empty/null IDS.
        let present = match ids {
            None => false,
            Some(TypedValue::String { value, .. }) => {
                !value.is_empty() && !value.eq_ignore_ascii_case("null")
            }
            Some(_) => {
                return Err(required(
                    "actual ActorData required_object_ids string required",
                ));
            }
        };
        if !present {
            return Ok(true);
        }
        self.services.required_object_init(self.record)
    }
    fn has_component(&self, a: ActorId, s: ComponentSlot) -> Result<bool> {
        self.owner(a)?;
        Ok(self
            .record
            .shell
            .actor
            .as_ref()
            .ok_or(Error::NullDereference("Actor Init components"))?
            .components
            .present(s))
    }
    fn component_init_existing(
        &mut self,
        a: ActorId,
        s: ComponentSlot,
        pc: Precision,
    ) -> Result<bool> {
        self.owner(a)?;
        if s == ComponentSlot::Mind {
            return self.services.mind_init_existing(self.record, pc);
        }
        if crate::load_detection_init_world::supports(s) {
            return self
                .services
                .scheduled_component_init_existing(self.record, s, pc);
        }
        if s == ComponentSlot::Economy {
            return self.services.economy_init_existing(self.record, pc);
        }
        if s == ComponentSlot::Body {
            return crate::load_body_init_world::init_existing(self.catalog, self.record, pc);
        }
        if s == ComponentSlot::Mover {
            return crate::load_mover_init_world::init_existing(
                self.catalog,
                self.record,
                pc,
                || self.services.world_version(),
            );
        }
        if crate::load_constant_component_init::supports(s) {
            return crate::load_constant_component_init::init_existing(
                self.catalog,
                self.record,
                s,
                pc,
            );
        }
        crate::load_component_init_world::init_existing(self.catalog, self.record, s, pc)
    }
    fn component_after_init(&mut self, a: ActorId, s: ComponentSlot) -> Result<()> {
        self.owner(a)?;
        if s == ComponentSlot::Mind {
            return crate::load_mind_init_world::after_init(self.catalog, self.record);
        }
        if matches!(s, ComponentSlot::Construction | ComponentSlot::Lair) {
            return crate::load_detection_init_world::after_constant(self.catalog, self.record, s);
        }
        if s == ComponentSlot::Detection {
            #[cfg(test)]
            crate::load_detection_init_world::capture_after_entry(self.catalog, self.record);
            return self.services.detection_after_init(self.record);
        }
        if s == ComponentSlot::Economy {
            return crate::load_economy_init_world::after_init(self.catalog, self.record);
        }
        if s == ComponentSlot::Body {
            return crate::load_body_init_world::after_init(self.catalog, self.record);
        }
        if s == ComponentSlot::Mover {
            return crate::load_mover_init_world::after_init(self.catalog, self.record);
        }
        if crate::load_constant_component_init::supports(s) {
            return crate::load_constant_component_init::after_init(self.catalog, self.record, s);
        }
        crate::load_component_init_world::after_init(self.catalog, self.record, s)
    }
    fn required_object(&self, a: ActorId) -> Result<ActorId> {
        self.owner(a)?;
        Ok(ActorId::from_raw(actor_word(self.record, 0x158)?))
    }
    fn gtid_live(&self, a: ActorId) -> Result<bool> {
        Ok(self.record.registry.get(a).is_some())
    }
    fn required_object_attach(&mut self, r: ActorId, a: ActorId) -> Result<()> {
        self.owner(a)?;
        self.services.required_object_attach(self.record, r)
    }
    fn refresh_4d4cf0(&mut self, a: ActorId) -> Result<()> {
        self.owner(a)?;
        crate::saved_actor_properties::refresh(
            self.catalog,
            ActorPropertiesContext {
                owner: a,
                owner_actor: self
                    .record
                    .shell
                    .actor
                    .as_mut()
                    .ok_or(Error::NullDereference("Actor Init +108 list"))?,
                owner_definition: self.record.definition,
                organization: None,
                registry: self.record.registry,
                precision: self.source_pc,
            },
        )
        .map_err(world)
    }
    fn refresh_4d1350(&mut self, a: ActorId) -> Result<()> {
        self.owner(a)?;
        self.services.refresh_4d1350(self.record,self.live_pc)
    }
    fn refresh_4d67f0(&mut self, a: ActorId) -> Result<()> {
        self.owner(a)?;
        if refresh_bit29_returns(self.record.shell.thing.flags) {
            Ok(())
        } else {
            self.services.refresh_4d67f0(self.record)
        }
    }
    fn thing_flags(&self, a: ActorId) -> Result<u32> {
        self.owner(a)?;
        Ok(self.record.shell.thing.flags)
    }
    fn global_property(&self) -> Result<Option<u32>> {
        self.services.global_property()
    }
    fn property_applies(&mut self, a: ActorId, p: u32) -> Result<bool> {
        self.owner(a)?;
        self.services.property_applies(self.record, p)
    }
    fn properties_add(&mut self, a: ActorId, p: u32) -> Result<()> {
        self.owner(a)?;
        self.services.properties_add(self.record, p)
    }
    fn refresh_4d4130(&mut self, a: ActorId) -> Result<()> {
        self.owner(a)?;
        if refresh_bit29_returns(self.record.shell.thing.flags) {
            Ok(())
        } else {
            self.services.refresh_4d4130(self.record)
        }
    }
    fn organization_refresh(&mut self, a: ActorId) -> Result<()> {
        self.owner(a)?;
        self.services.organization_refresh(self.record)
    }
    fn settlement_refresh(&mut self, a: ActorId) -> Result<()> {
        self.owner(a)?;
        self.services.settlement_refresh(self.record)
    }
    fn performer_refresh(&mut self, a: ActorId) -> Result<()> {
        self.owner(a)?;
        self.services.performer_refresh(self.record)
    }
    fn denizen_refresh(&mut self, a: ActorId) -> Result<()> {
        self.owner(a)?;
        self.services.denizen_refresh(self.record)
    }
    fn needs_settlement_zone(&mut self, a: ActorId) -> Result<bool> {
        self.owner(a)?;
        self.source_field("needs_space_for_settlement_zone")?
            .ok_or(required("ActorData+34 settlement-space source missing"))?
            .boolean()
            .map_err(world)
    }
    fn associated_object(&self, a: ActorId) -> Result<ActorId> {
        self.owner(a)?;
        Ok(ActorId::from_raw(self.record.word(0x50)?))
    }
    fn create_zone2(&mut self, a: ActorId) -> Result<bool> {
        self.owner(a)?;
        self.services.create_zone2(self.record)
    }
    fn discovery_radius(&mut self, pc: Precision) -> Result<u32> {
        self.services.discovery_radius(pc)
    }
    fn position_xy(&self, a: ActorId) -> Result<[u32; 2]> {
        self.owner(a)?;
        Ok([self.record.word(0x20)?, self.record.word(0x24)?])
    }
    fn zone2(&self, a: ActorId) -> Result<Option<u32>> {
        self.owner(a)?;
        self.services.zone2(self.record)
    }
    fn move_zone(&mut self, z: u32, xy: [u32; 2], r: u32) -> Result<()> {
        self.services.move_zone(z, xy, r)
    }
    fn sai_present(&self) -> Result<bool> {
        self.catalog.saved_observer_present().map_err(world)
    }
    fn sai_notify(&mut self, a: ActorId) -> Result<()> {
        self.owner(a)?;
        self.catalog.notify_saved_observer(a.raw()).map_err(world)
    }
    fn track_definition(&self, a: ActorId) -> Result<u32> {
        self.owner(a)?;
        let id = &self.record.definition.proof.id;
        let token = self
            .catalog
            .definition_bank
            .try_borrow()
            .map_err(world)?
            .token("ActorDefinition", id)
            .ok_or(required(
                "Actor Init existing source definition token absent",
            ))?;
        let node = self.catalog.actors.resolved_actor(id).map_err(world)?;
        let mover = node.child("MoverComponent").map_err(world)?;
        let children = match mover {
            Some(m) => m.children("TrackInfo").map_err(world)?,
            None => Vec::new(),
        };
        let mut tracks = Vec::new();
        for child in children {
            let kind = match child.field("track_type", self.source_pc).map_err(world)? {
                Some(TypedValue::String { value, .. }) => match value.to_ascii_uppercase().as_str()
                {
                    "DISCRETE" => 0,
                    "CONTINUOUS" => 1,
                    _ => return Err(required("unknown actual TrackInfo.track_type")),
                },
                _ => return Err(required("actual TrackInfo.track_type string required")),
            };
            let fade_sec_bits = match child.field("fade_sec", self.source_pc).map_err(world)? {
                Some(TypedValue::Scalar(s)) => s.bits,
                _ => return Err(required("actual TrackInfo.fade_sec scalar required")),
            };
            tracks.push(TrackInfoInputs {
                kind,
                fade_sec_bits,
            });
        }
        *self.tracks.borrow_mut() = Some((token, tracks));
        Ok(token)
    }
    fn track_count(&self, d: u32) -> Result<i32> {
        i32::try_from(self.tracks(d)?.len()).map_err(world)
    }
    fn track_at(&self, d: u32, i: usize) -> Result<usize> {
        self.tracks(d)?
            .get(i)
            .ok_or(required("actual TrackInfo index out of range"))?;
        Ok(i)
    }
    fn track_kind(&self, t: usize) -> Result<u32> {
        self.tracks
            .borrow()
            .as_ref()
            .and_then(|v| v.1.get(t))
            .map(|v| v.kind)
            .ok_or(required("actual captured TrackInfo absent"))
    }
    fn track_length_bits(&self, t: usize) -> Result<u32> {
        self.tracks
            .borrow()
            .as_ref()
            .and_then(|v| v.1.get(t))
            .map(|v| v.fade_sec_bits)
            .ok_or(required("actual captured TrackInfo absent"))
    }
    fn track_scale_bits(&self) -> Result<u32> {
        self.services.track_scale_bits()
    }
    fn track_divisor(&self) -> Result<i32> {
        self.services.track_divisor()
    }
    fn allocate_track(&mut self) -> Result<Option<u32>> {
        self.services.allocate_track()
    }
    fn construct_track(&mut self, x: u32, a: ActorId, p: i32) -> Result<u32> {
        self.owner(a)?;
        self.services.construct_track(x, a, p)
    }
    fn set_track_updatable(&mut self, a: ActorId, v: Option<u32>) -> Result<()> {
        self.owner(a)?;
        let value = v.unwrap_or(0); // actual allocation-null stores a native null.
        let slot = self
            .record
            .shell
            .actor
            .as_mut()
            .ok_or(Error::NullDereference("Actor Init track storage"))?
            .words
            .get_mut(&0x144)
            .ok_or(required("Actor Init initialized +144 absent"))?;
        *slot = value;
        Ok(())
    }
    fn state_low(&self, a: ActorId) -> Result<u32> {
        self.owner(a)?;
        Ok(self
            .record
            .shell
            .actor
            .as_ref()
            .ok_or(Error::NullDereference("Actor Init state"))?
            .components
            .state
            .low)
    }
    fn existing_fx(&mut self, a: ActorId) -> Result<()> {
        self.owner(a)?;
        // Lazy at the actual state-bit21 branch. This certificate comes from
        // the canonical inherited View lists and native append/filter producer;
        // a synthetic probe's empty list cannot authorize a real source list.
        let source = crate::definition_fx::Source::bind(self.catalog, self.record.definition)
            .map_err(world)?;
        if source
            .list(crate::definition_fx::ListKind::Constructed)
            .is_certified_empty()
        {
            // 42A590 copies the actual actor XY/Z, then429080 returns before
            // any world/presentation read when this compiled list head is null.
            let _position = self.record.shell.thing.position_bits;
            Ok(())
        } else {
            self.services.existing_fx(self.record)
        }
    }
}
