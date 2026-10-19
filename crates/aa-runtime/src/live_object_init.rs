//! Saved Object Init(NULL) over the *current* owned load shell.
//!
//! Fields, definition geometry, half-cell snapping, original height sampling
//! and world+8 insertion are concrete. Remaining callbacks are required native
//! services, never implicit success. This is not a full Actor/Kingdom Init or
//! PostCreate adapter. The current shell is borrowed before ActorStore staging;
//! there is no second mutable pose/flag copy.
use aa_actors::{
    ActorId, IdentityRegistry,
    actor_creation::{Error, Result, Shape},
    loading::{LoadKind, LoadShell},
    object_init_null,
};
use aa_mapworld::{
    bootstrap::DefinitionProof,
    restore::{RestoredComponent, RestoredRecord, RuntimeDefinition},
};
use aa_numeric::{Binary, Precision};
use aa_spatial::index_world::GroundSurface;

#[cfg(test)]
#[path = "live_object_init_tests.rs"]
mod tests;

/// The current record remains registered (normally as Loading while borrowed),
/// and all other current records stay in the same authoritative registry.
pub struct Record<'a> {
    pub owner: ActorId,
    pub shell: &'a mut LoadShell<DefinitionProof, RestoredComponent>,
    pub definition: &'a RuntimeDefinition,
    pub registry: &'a mut IdentityRegistry<RestoredRecord>,
}

/// Readonly disjoint current storage used by memory callbacks. This excludes
/// Actor+148 so its actual Zone can remain exclusively borrowed through reveal.
pub struct MemoryRecord<'a> {
    pub owner: ActorId,
    pub definition: &'a RuntimeDefinition,
    pub kind: LoadKind,
    pub proof: &'a DefinitionProof,
    pub flags: &'a u32,
    pub object_present: bool,
    pub actor_present: bool,
    pub registry: &'a IdentityRegistry<RestoredRecord>,
}

impl Record<'_> {
    pub fn memory_record(&self) -> MemoryRecord<'_> {
        MemoryRecord {
            owner: self.owner,
            definition: self.definition,
            kind: self.shell.kind,
            proof: &self.shell.thing.definition,
            flags: &self.shell.thing.flags,
            object_present: self.shell.object.is_some(),
            actor_present: self.shell.actor.is_some(),
            registry: self.registry,
        }
    }
    fn require_owner(&self, owner: ActorId) -> Result<()> {
        if owner != self.owner {
            return Err(Error::World(
                "Object Init attempted another current shell".into(),
            ));
        }
        Ok(())
    }

    pub fn word(&self, offset: u32) -> Result<u32> {
        let object = || {
            self.shell
                .object
                .as_ref()
                .ok_or(Error::NullDereference("Object Init shell"))
        };
        Ok(match offset {
            8 => self.shell.thing.flags,
            0x20 | 0x24 | 0x28 => self.shell.thing.position_bits[((offset - 0x20) / 4) as usize],
            0x2c | 0x30 | 0x34 => self.shell.thing.facing_bits[((offset - 0x2c) / 4) as usize],
            0x38 => object()?.associated_handle,
            0x3c => object()?.scale_bits,
            0x40 => object()?.variant as u32,
            0x44 => object()?.word_44,
            0x48 => object()?.word_48,
            0x50 | 0x54 => object()?.words_50_to_54[((offset - 0x50) / 4) as usize],
            0x58 => object()?.memory.count() as u32,
            //Only null/non-null is projected; this is never a native pointer.
            0x5c => u32::from(object()?.memory.allocation_present()),
            _ => {
                return Err(Error::World(format!(
                    "Object Init field +{offset:x} is not represented"
                )));
            }
        })
    }

    pub fn set_word(&mut self, offset: u32, value: u32) -> Result<()> {
        match offset {
            8 => self.shell.thing.flags = value,
            0x20 | 0x24 | 0x28 => {
                self.shell.thing.position_bits[((offset - 0x20) / 4) as usize] = value
            }
            0x2c | 0x30 | 0x34 => {
                self.shell.thing.facing_bits[((offset - 0x2c) / 4) as usize] = value
            }
            0x38 => {
                self.shell
                    .object
                    .as_mut()
                    .ok_or(Error::NullDereference("Object Init shell"))?
                    .associated_handle = value
            }
            0x3c => {
                self.shell
                    .object
                    .as_mut()
                    .ok_or(Error::NullDereference("Object Init shell"))?
                    .scale_bits = value
            }
            0x40 => {
                self.shell
                    .object
                    .as_mut()
                    .ok_or(Error::NullDereference("Object Init shell"))?
                    .variant = value as i32
            }
            0x44 => {
                self.shell
                    .object
                    .as_mut()
                    .ok_or(Error::NullDereference("Object Init shell"))?
                    .word_44 = value
            }
            0x48 => {
                self.shell
                    .object
                    .as_mut()
                    .ok_or(Error::NullDereference("Object Init shell"))?
                    .word_48 = value
            }
            0x50 | 0x54 => {
                self.shell
                    .object
                    .as_mut()
                    .ok_or(Error::NullDereference("Object Init shell"))?
                    .words_50_to_54[((offset - 0x50) / 4) as usize] = value
            }
            0x58 | 0x5c => {
                return Err(Error::Unsupported(
                    "Object detection-memory fields require their actual typed allocation owner",
                ));
            }
            _ => {
                return Err(Error::World(format!(
                    "Object Init field +{offset:x} is not represented"
                )));
            }
        }
        Ok(())
    }
}

/// A source-bound immutable definition, including the actual inherited shape.
/// Construction is private so callers cannot choose viewer bounds/defaults.
pub struct Source {
    id: String,
    provenance: aa_rules::resources::Provenance,
    template_chain: Vec<aa_rules::resources::Provenance>,
    category: aa_rules::native::CategoryDescriptor,
    native_class: u32,
    source_precision: Precision,
    height_bits: u32,
    shape: Shape,
    blocking: aa_spatial::collision::Shape,
}
impl Source {
    pub fn bind(
        catalog: &crate::catalog::Catalog<'_>,
        definition: &RuntimeDefinition,
    ) -> Result<Self> {
        if !matches!(definition.native_class, 2 | 3 | 4 | 6 | 7 | 9) {
            return Err(Error::Unsupported("current Object Init class virtuals"));
        }
        catalog.require_source_owner().map_err(Error::World)?;
        let bank = catalog
            .definition_bank
            .try_borrow()
            .map_err(|e| Error::World(e.to_string()))?;
        Self::bind_in_bank(catalog, &bank, definition)
    }
    ///The SAME source Bank moved into Staged/LoadedMap is borrowed directly.
    ///This never reconstructs or reinstalls a second Catalog Bank.
    pub fn bind_in_bank(
        catalog: &crate::catalog::Catalog<'_>,
        bank: &crate::definitions::Bank,
        definition: &RuntimeDefinition,
    ) -> Result<Self> {
        if !matches!(definition.native_class, 2 | 3 | 4 | 6 | 7 | 9) {
            return Err(Error::Unsupported("current Object Init class virtuals"));
        }
        Self::bind_definition_in_bank(catalog, bank, definition)
    }
    /// Definition-load precision only. The DataManager factory selects the
    /// ObjectData49DC40-derived LoadData by data kind (ObjectData1,
    /// ActorData2, Camera20, DynamicDoodad21), not by the runtime class the
    /// definition later creates (GROUP is ActorData/class8). This validates
    /// the same canonical identity, category, final flags and inherited
    /// geometry, and returns no Init Source, so no Init virtual is admitted.
    pub(crate) fn definition_load_precision(
        catalog: &crate::catalog::Catalog<'_>,
        bank: &crate::definitions::Bank,
        definition: &RuntimeDefinition,
    ) -> Result<Precision> {
        if !matches!(definition.proof.category.native_kind, 1 | 2 | 20 | 21) {
            return Err(Error::Unsupported(
                "definition load requires ObjectData-derived data kind",
            ));
        }
        Self::bind_definition_in_bank(catalog, bank, definition).map(|s| s.source_precision)
    }
    fn bind_definition_in_bank(
        catalog: &crate::catalog::Catalog<'_>,
        bank: &crate::definitions::Bank,
        definition: &RuntimeDefinition,
    ) -> Result<Self> {
        catalog.require_source_owner().map_err(Error::World)?;
        let owner = bank.combat_source().map_err(Error::World)?;
        if owner.source_identities() != catalog.actors.sources.as_slice() {
            return Err(Error::World(
                "Object Init definition source FP owner differs from Catalog".into(),
            ));
        }
        let pc = owner.source_precision();
        let id = &definition.proof.id;
        //Re-identify the SAME resolved allocation by its registered IDS. For
        //every unshadowed IDS this equals Find; a translated-away registered
        //Thing (chaparral_wall) keeps its own source.
        let thing = aa_rules::actors::ThingRef::registered(id);
        let canonical = catalog
            .actors
            .select(thing)
            .ok_or_else(|| Error::World(format!("Object Init definition {id} is absent")))?;
        if !canonical.id.eq_ignore_ascii_case(id)
            || canonical.source != definition.proof.source
            || canonical.template_chain != definition.proof.template_chain
            || definition.proof.category.runtime_class != definition.native_class
            || catalog
                .actors
                .native_category(thing)
                .map_err(|e| Error::World(e.to_string()))?
                != definition.proof.category
            || catalog
                .actors
                .native_final_flags(thing, pc)
                .map_err(|e| Error::World(e.to_string()))?
                != definition.flags
        {
            return Err(Error::World(
                "Object Init canonical definition/source differs".into(),
            ));
        }
        let geometry = catalog
            .actors
            .native_object_definition(thing, pc)
            .map_err(|e| Error::World(e.to_string()))?;
        Ok(Self {
            id: id.clone(),
            provenance: geometry.source,
            template_chain: canonical.template_chain.clone(),
            category: definition.proof.category,
            native_class: definition.native_class,
            source_precision: pc,
            height_bits: geometry.height.bits,
            shape: Shape {
                kind: geometry.blocking.words[0],
                radius_bits: geometry.blocking.words[1],
                extent_10_bits: geometry.blocking.words[4],
                extent_14_bits: geometry.blocking.words[5],
            },
            blocking: aa_spatial::collision::Shape {
                kind: geometry.blocking.words[0],
                radius_bits: geometry.blocking.words[1],
                bounds: geometry.blocking.words[2..6].try_into().unwrap(),
            },
        })
    }

    /// Definition-load precision, independent of the live Init arithmetic PC.
    pub fn source_precision(&self) -> Precision {
        self.source_precision
    }

    /// Immutable canonical blocking shape; this does not expose mutable source
    /// ownership or substitute viewer bounds for native placement geometry.
    pub fn blocking_shape(&self) -> &aa_spatial::collision::Shape {
        &self.blocking
    }

    pub(crate) fn validate_record(&self, record: &Record<'_>) -> Result<()> {
        self.validate_memory_record(&record.memory_record())
    }
    pub(crate) fn validate_memory_record(&self, record: &MemoryRecord<'_>) -> Result<()> {
        if record.owner.is_null()
            || !record
                .registry
                .get(record.owner)
                .is_some_and(|entry| matches!(&entry.value, RestoredRecord::Loading))
            || !record.object_present
            || record.actor_present != matches!(self.native_class, 3 | 4)
            || record.definition.native_class != self.native_class
            || record.definition.proof.id != self.id
            || record.definition.proof.source != self.provenance
            || record.definition.proof.template_chain != self.template_chain
            || record.definition.proof.category != self.category
            || record.proof.id != self.id
            || record.proof.source != self.provenance
            || record.proof.template_chain != self.template_chain
            || record.proof.category != self.category
            || record.kind
                != if matches!(self.native_class, 3 | 4) {
                    LoadKind::Actor
                } else {
                    LoadKind::Object
                }
        {
            return Err(Error::World(
                "Object Init current shell/source ownership differs".into(),
            ));
        }
        Ok(())
    }
}

/// Actual4EA150 occupancy indices, using the native mode-one iterator over the
/// captured current pose and immutable source shape. The caller must supply
/// the owned B3A8C4 point shape explicitly; this function never constructs it
/// from PE zeros. Zone contents remain live reads by the footprint service.
pub fn occupancy_indices(
    record: &Record<'_>,
    source: &Source,
    map: &aa_zones::tile::MapGeometry,
    global_point_shape: &aa_spatial::collision::Shape,
    pc: Precision,
) -> Result<Vec<usize>> {
    source.validate_record(record)?;
    let grid = aa_spatial::collision::Grid {
        dimensions: [map.width, map.height],
        coordinate_shift: map.shift,
        cell_size_bits: map.tile_size_bits,
    };
    let placement = aa_spatial::geometry::Placement {
        shape: source.blocking.clone(),
        facing: [record.word(0x2c)?, record.word(0x30)?],
        position: [record.word(0x20)?, record.word(0x24)?],
    };
    aa_spatial::creation_placement::occupancy_tiles(
        &grid,
        map.extent_bits,
        global_point_shape,
        &placement,
        pc,
    )
    .map_err(|e| Error::World(e.to_string()))?
    .into_iter()
    .map(|[x, y]| {
        if x < 0 || y < 0 || x >= map.width || y >= map.height {
            return Err(Error::World(
                "footprint iterator yielded an outside tile".into(),
            ));
        }
        (y as usize)
            .checked_mul(map.width as usize)
            .and_then(|n| n.checked_add(x as usize))
            .ok_or(Error::Domain("footprint row-major tile index overflow"))
    })
    .collect()
}

/// Raw owned inputs not yet present in the common saved shell. These reads are
/// required only when the original predicate reaches them; they must never
/// manufacture a phase or an absent authored byte.
pub trait MemoryServices {
    /// Untouched76AC90 reads [Sim+F8]. This is independent of MapResource+A8.
    fn simulation_phase(&mut self, record: &MemoryRecord<'_>) -> Result<u32>;
    /// ObjectData/ActorData+16D; called only on the native late read path.
    fn definition_announce_byte(&mut self, record: &MemoryRecord<'_>) -> Result<u8>;
}

pub(crate) trait MemoryFields {
    fn phase(&mut self) -> Result<u32>;
    fn mask(&mut self) -> Result<u32>;
    fn flags(&mut self) -> Result<u32>;
    fn announce(&mut self) -> Result<u8>;
}

pub(crate) fn potentially_visible(class: u32, fields: &mut impl MemoryFields) -> Result<bool> {
    // DoodadData+40 returns FULL EAX true before any flags/mask reads.
    // Other admitted Data+40 leaves return false; Data+38 is true only for
    // ActorData (runtime Actor/Kingdom), not Area/Camera/Doodad.
    if class == 9 {
        return Ok(false);
    }
    if matches!(class, 3 | 4) && fields.mask()? & (1 << 26) != 0 {
        return Ok(false);
    }
    let flags = fields.flags()?;
    if flags & (1 << 29) == 0
        || flags & (1 << 11) == 0
        || flags & (1 << 23) != 0
        || flags & (1 << 22) != 0
    {
        return Ok(false);
    }
    if fields.announce()? != 0 {
        return Ok(false);
    }
    // Actual virtual+A8: false for Object/Actor, true for Kingdom.
    Ok(class != 4)
}

pub(crate) fn memory_eligibility(class: u32, fields: &mut impl MemoryFields) -> Result<bool> {
    if fields.phase()? == 5 {
        return Ok(false);
    }
    if matches!(class, 3 | 4) && fields.mask()? & (1 << 26) != 0 {
        return Ok(false);
    }
    if potentially_visible(class, fields)? || class == 4 {
        return Ok(false);
    }
    let flags = fields.flags()?;
    Ok(flags & (1 << 22) == 0 && flags & (1 << 29) != 0)
}

/// Concrete4EB490+4EA350 over the current shell, including lazy source reads.
/// This neither creates/destroys detection memory nor implements4EBC70. Missing
/// live phase/source ownership remains an error from the required service.
pub fn memory_eligible(
    record: &Record<'_>,
    source: &Source,
    services: &mut impl MemoryServices,
) -> Result<bool> {
    memory_eligible_fields(&record.memory_record(), source, services)
}
pub(crate) fn memory_eligible_fields(
    record: &MemoryRecord<'_>,
    source: &Source,
    services: &mut impl MemoryServices,
) -> Result<bool> {
    source.validate_memory_record(record)?;
    struct Fields<'a, 'r, S> {
        record: &'a MemoryRecord<'r>,
        services: &'a mut S,
    }
    impl<S: MemoryServices> MemoryFields for Fields<'_, '_, S> {
        fn phase(&mut self) -> Result<u32> {
            self.services.simulation_phase(self.record)
        }
        fn mask(&mut self) -> Result<u32> {
            self.record
                .definition
                .component_mask
                .ok_or(Error::Unsupported(
                    "memory predicate ActorData+174 is not completed",
                ))
        }
        fn flags(&mut self) -> Result<u32> {
            Ok(*self.record.flags)
        }
        fn announce(&mut self) -> Result<u8> {
            self.services.definition_announce_byte(self.record)
        }
    }
    memory_eligibility(source.native_class, &mut Fields { record, services })
}

/// Concrete ownership is supplied by the live map. No MapResource mode,
/// surface, world-list or flags are fabricated by this adapter.
pub struct Context<'record, 'map, 'surface> {
    pub record: Record<'record>,
    pub source: &'record Source,
    pub surface: &'map GroundSurface<'surface>,
    pub map_mode: &'map mut Option<u32>,
    pub objects: &'map mut crate::world_object_list::WorldObjectList<ActorId>,
}

/// Required unreplaced native services. Each callback sees the actual current
/// shell and registry and may mutate them; later reads observe those writes.
/// The two concrete native entry gates below call membership only when bit26
/// is clear (+B0 is actually true for the three admitted vtables), and call
/// blocking notification only when the current bit10 is set.
pub trait Services {
    fn thickness_warning(&mut self, record: &mut Record<'_>) -> Result<()>;
    fn placement_null(
        &mut self,
        record: &mut Record<'_>,
        mode_owner: &Option<u32>,
        pc: Precision,
    ) -> Result<u32>;
    fn placement_failure(&mut self, record: &mut Record<'_>, code: u32) -> Result<()>;
    fn placement_map_warning(&mut self, record: &mut Record<'_>) -> Result<()>;
    fn spatial_membership(&mut self, record: &mut Record<'_>) -> Result<()>;
    fn blocking_notify(&mut self, record: &mut Record<'_>) -> Result<()>;
    fn tile_footprint(&mut self, record: &mut Record<'_>) -> Result<()>;
    fn retain_instances(&mut self, record: &mut Record<'_>) -> Result<bool>;
    fn clear_instances(&mut self, record: &mut Record<'_>) -> Result<()>;
    fn visibility_refresh(&mut self, record: &mut Record<'_>) -> Result<()>;
}

struct Bound<'a, 'record, 'map, 'surface, S> {
    context: Context<'record, 'map, 'surface>,
    services: &'a mut S,
    precision: Precision,
}

fn membership_service_required(flags: u32) -> bool {
    flags & (1 << 26) == 0
}
fn blocking_service_required(flags: u32) -> bool {
    flags & (1 << 10) != 0
}

impl<S: Services> object_init_null::World for Bound<'_, '_, '_, '_, S> {
    fn word(&mut self, actor: ActorId, offset: u32) -> Result<u32> {
        self.context.record.require_owner(actor)?;
        self.context.record.word(offset)
    }
    fn set_word(&mut self, actor: ActorId, offset: u32, value: u32) -> Result<()> {
        self.context.record.require_owner(actor)?;
        self.context.record.set_word(offset, value)
    }
    fn clear_byte_4c(&mut self, actor: ActorId) -> Result<()> {
        self.context.record.require_owner(actor)?;
        self.context
            .record
            .shell
            .object
            .as_mut()
            .ok_or(Error::NullDereference("Object Init shell"))?
            .byte_4c = 0;
        Ok(())
    }
    fn definition_height_bits(&mut self, actor: ActorId) -> Result<u32> {
        self.context.record.require_owner(actor)?;
        Ok(self.context.source.height_bits)
    }
    fn tile_size_bits(&mut self) -> Result<u32> {
        Ok(self.context.surface.terrain.cell_size)
    }
    fn tile_shift(&mut self) -> Result<u32> {
        Ok(self.context.surface.terrain.coordinate_shift)
    }
    fn radius(&mut self, actor: ActorId) -> Result<Binary> {
        self.context.record.require_owner(actor)?;
        Ok(Binary::from_f32_bits(
            self.context.source.shape.radius_bits,
        )?)
    }
    fn shape(&mut self, actor: ActorId) -> Result<Shape> {
        self.context.record.require_owner(actor)?;
        Ok(self.context.source.shape)
    }
    fn thickness_warning(&mut self, _: ActorId) -> Result<()> {
        self.services.thickness_warning(&mut self.context.record)
    }
    fn snap(&mut self, actor: ActorId, _: Shape, tile: u32, pc: Precision) -> Result<()> {
        self.context.record.require_owner(actor)?;
        let xy: &mut [u32; 2] = (&mut self.context.record.shell.thing.position_bits[..2])
            .try_into()
            .unwrap();
        aa_spatial::snap_half_cell(xy, tile, pc).map_err(|e| Error::World(e.to_string()))
    }
    fn placement_null(&mut self, _: ActorId) -> Result<u32> {
        self.services.placement_null(
            &mut self.context.record,
            &*self.context.map_mode,
            self.precision,
        )
    }
    fn placement_failure(&mut self, _: ActorId, code: u32) -> Result<()> {
        self.services
            .placement_failure(&mut self.context.record, code)
    }
    fn map_mode(&mut self) -> Result<Option<u32>> {
        Ok(*self.context.map_mode)
    }
    fn placement_map_warning(&mut self, _: ActorId) -> Result<()> {
        self.services
            .placement_map_warning(&mut self.context.record)
    }
    fn spatial_membership(&mut self, _: ActorId) -> Result<()> {
        if membership_service_required(self.context.record.word(8)?) {
            self.services.spatial_membership(&mut self.context.record)?;
        }
        Ok(())
    }
    fn terrain_height(&mut self, actor: ActorId, flags: u32) -> Result<Binary> {
        self.context.record.require_owner(actor)?;
        self.context
            .surface
            .sample(
                [
                    self.context.record.shell.thing.position_bits[0],
                    self.context.record.shell.thing.position_bits[1],
                ],
                flags,
                self.precision,
            )
            .map_err(|e| Error::World(e.to_string()))
    }
    fn blocking_notify(&mut self, _: ActorId) -> Result<()> {
        if blocking_service_required(self.context.record.word(8)?) {
            self.services.blocking_notify(&mut self.context.record)?;
        }
        Ok(())
    }
    fn tile_footprint(&mut self, _: ActorId) -> Result<()> {
        self.services.tile_footprint(&mut self.context.record)
    }
    fn world_list_insert(&mut self, actor: ActorId) -> Result<()> {
        self.context.record.require_owner(actor)?;
        self.context
            .objects
            .insert(actor, self.context.record.shell.thing.flags)
            .map_err(|e| Error::World(e.to_string()))?;
        Ok(())
    }
    fn has_instances(&mut self, actor: ActorId) -> Result<bool> {
        self.context.record.require_owner(actor)?;
        Ok((self.context.record.word(0x58)? as i32) > 0)
    }
    fn retain_instances(&mut self, _: ActorId) -> Result<bool> {
        self.services.retain_instances(&mut self.context.record)
    }
    fn clear_instances(&mut self, _: ActorId) -> Result<()> {
        self.services.clear_instances(&mut self.context.record)
    }
    fn visibility_refresh(&mut self, _: ActorId) -> Result<()> {
        self.services.visibility_refresh(&mut self.context.record)
    }
}

pub fn initialize(
    context: Context<'_, '_, '_>,
    pc: Precision,
    services: &mut impl Services,
) -> Result<bool> {
    context.source.validate_record(&context.record)?;
    let owner = context.record.owner;
    object_init_null::initialize(
        owner,
        pc,
        &mut Bound {
            context,
            services,
            precision: pc,
        },
    )
}
