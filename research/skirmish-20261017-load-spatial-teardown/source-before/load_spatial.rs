//! Pre-staging membership (4EE4C0/5536B0) over actual held load shells.
//! Only tile/node/generation state and immutable source bindings persist.
//! Member fields exist within one synchronous lease, are committed back on
//! success/error, then discarded. No outward callback can see stale fields.
use crate::{catalog::Catalog, live_object_init::Record, spatial_world::SourceGeometry};
use aa_actors::{
    ActorId,
    components::ComponentSlot,
    loading::{LoadKind, LoadShell},
};
use aa_mapworld::{
    bootstrap::DefinitionProof,
    restore::{RestoredComponent, RestoredRecord, RuntimeDefinition},
    terrain::BoundTerrain,
};
use aa_numeric::Precision;
use aa_spatial::{
    collision::{Actor, Grid, Shape},
    index::{ActorKey, Member, SpatialIndex},
};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Spatial(#[from] aa_spatial::Error),
    #[error("load spatial source/ownership: {0}")]
    Required(String),
}
pub type Result<T> = std::result::Result<T, Error>;
fn required(value: impl std::fmt::Display) -> Error {
    Error::Required(value.to_string())
}
pub(crate) fn same_proof(a: &DefinitionProof, b: &DefinitionProof) -> bool {
    a.id.eq_ignore_ascii_case(&b.id)
        && a.source == b.source
        && a.template_chain == b.template_chain
        && a.category.native_kind == b.category.native_kind
        && a.category.runtime_class == b.category.runtime_class
        && a.category.registry_index == b.category.registry_index
        && a.category.view_type == b.category.view_type
}
pub(crate) fn ordinary_layout(class: u32, kind: LoadKind, object: bool, actor: bool) -> bool {
    match class {
        2 | 6 | 7 | 9 => kind == LoadKind::Object && object && !actor,
        3 | 4 => kind == LoadKind::Actor && object && actor,
        _ => false,
    }
}

#[derive(Clone, Debug)]
pub struct Binding {
    pub id: ActorId,
    pub key: ActorKey,
    pub definition: u32,
    pub native_class: u32,
    /// Immutable validated definition certificate; agreeing mutable shell and
    /// RuntimeDefinition copies are not source authority at later handoffs.
    pub(crate) proof: DefinitionProof,
}
/// Moves the SAME native lists and generation map to staging. It carries no
/// mutable pose/flag copies. Bindings remain in original membership-call order.
pub struct Parts {
    pub index: SpatialIndex,
    pub bindings: Vec<Binding>,
    pub definitions: BTreeMap<u32, Arc<SourceGeometry>>,
    pub next_key: ActorKey,
}
pub struct LoadSpatial {
    index: SpatialIndex,
    bindings: Vec<Binding>,
    definitions: BTreeMap<u32, Arc<SourceGeometry>>,
    next_key: ActorKey,
}

/// A second real allocation held outside its matching Loading registry entry
/// during a synchronous child query (for example4EB6E0->4D67F0). No shell or
/// mutable field is copied; only the same owner's borrow crosses this call.
pub(crate) struct HeldShell<'a> {
    pub owner: ActorId,
    pub shell: &'a mut LoadShell<DefinitionProof, RestoredComponent>,
    pub definition: &'a RuntimeDefinition,
}
#[derive(Clone, Copy)]
pub(crate) struct HeldView<'a> {
    pub owner: ActorId,
    pub shell: &'a LoadShell<DefinitionProof, RestoredComponent>,
    pub definition: &'a RuntimeDefinition,
}
impl HeldShell<'_> {
    pub(crate) fn view(&self) -> HeldView<'_> {
        HeldView { owner:self.owner, shell:self.shell, definition:self.definition }
    }
}

/// Read-only logical identities and immutable source certificates belonging to
/// the SAME load index. Copying this view copies references, never object pose.
#[derive(Clone, Copy)]
pub(crate) struct QueryBindings<'a> {
    bindings: &'a [Binding],
    definitions: &'a BTreeMap<u32, Arc<SourceGeometry>>,
}
impl<'a> QueryBindings<'a> {
    pub(crate) fn key(self, id: ActorId) -> Result<ActorKey> {
        self.bindings
            .iter()
            .find(|b| b.id == id)
            .map(|b| b.key)
            .ok_or_else(|| required("query GTID has no load spatial binding"))
    }
    pub(crate) fn id(self, key: ActorKey) -> Result<ActorId> {
        self.bindings
            .iter()
            .find(|b| b.key == key)
            .map(|b| b.id)
            .ok_or_else(|| required("query key has no load GTID binding"))
    }
    pub(crate) fn native_class(self, key: ActorKey) -> Result<u32> {
        self.bindings
            .iter()
            .find(|b| b.key == key)
            .map(|b| b.native_class)
            .ok_or_else(|| required("query key has no load class binding"))
    }
    pub(crate) fn proof(self, key: ActorKey) -> Result<&'a DefinitionProof> {
        self.bindings
            .iter()
            .find(|b| b.key == key)
            .map(|b| &b.proof)
            .ok_or_else(|| required("query key has no load source certificate"))
    }
    pub(crate) fn source_geometry(self, token: u32) -> Result<&'a SourceGeometry> {
        self.definitions
            .get(&token)
            .map(Arc::as_ref)
            .ok_or_else(|| required("query immutable source geometry is unavailable"))
    }
    pub(crate) fn bindings(self) -> &'a [Binding] {
        self.bindings
    }
}
fn shape(value: &aa_rules::objects::Shape) -> Shape {
    let w = value.words;
    Shape {
        kind: w[0],
        radius_bits: w[1],
        bounds: [w[2], w[3], w[4], w[5]],
    }
}
fn record_fields(
    record: &RestoredRecord,
) -> Result<(
    &LoadShell<DefinitionProof, RestoredComponent>,
    &RuntimeDefinition,
)> {
    match record {
        RestoredRecord::Actor(v) | RestoredRecord::Object(v) | RestoredRecord::Thing(v) => {
            Ok((&v.shell, &v.definition))
        }
        RestoredRecord::Kingdom(v) => v
            .runtime
            .as_ref()
            .map(|v| (&v.base, &v.definition))
            .ok_or_else(|| required("prior Kingdom has no runtime shell")),
        RestoredRecord::Team(_) => Err(required("Team cannot have Object membership")),
        RestoredRecord::Loading => Err(required("another indexed record is held by Load")),
    }
}
impl LoadSpatial {
    /// Actual bound native tile flags, never viewer terrain or zero placeholders.
    pub fn new(terrain: &BoundTerrain, precision: Precision) -> Result<Self> {
        let l = terrain.layout;
        let count = l.tile_count().map_err(required)?;
        if terrain.cells.len() != count {
            return Err(required("bound terrain cell count differs"));
        }
        Ok(Self {
            index: SpatialIndex::new(
                Grid {
                    dimensions: [l.width, l.height],
                    coordinate_shift: l.coordinate_shift,
                    cell_size_bits: l.cell_size_bits,
                },
                terrain.cells.iter().map(|c| c.flags20).collect(),
                precision,
            )?,
            bindings: vec![],
            definitions: BTreeMap::new(),
            next_key: 1,
        })
    }
    pub fn index(&self) -> &SpatialIndex {
        &self.index
    }
    pub fn key(&self, id: ActorId) -> Result<ActorKey> {
        self.bindings
            .iter()
            .find(|b| b.id == id)
            .map(|b| b.key)
            .ok_or_else(|| required("object has no load spatial binding"))
    }
    pub fn bindings(&self) -> &[Binding] {
        &self.bindings
    }
    pub fn into_parts(self) -> Parts {
        Parts {
            index: self.index,
            bindings: self.bindings,
            definitions: self.definitions,
            next_key: self.next_key,
        }
    }
    fn prepare(&mut self, catalog: &Catalog<'_>, record: &Record<'_>) -> Result<ActorKey> {
        if !matches!(record.definition.native_class, 2 | 3 | 4 | 6 | 7 | 9) {
            return Err(required("unbound Object membership virtual class"));
        }
        if !ordinary_layout(
            record.definition.native_class,
            record.shell.kind,
            record.shell.object.is_some(),
            record.shell.actor.is_some(),
        ) {
            return Err(required(
                "membership actual constructor storage differs from vtable class",
            ));
        }
        if !record
            .registry
            .get(record.owner)
            .is_some_and(|e| matches!(e.value, RestoredRecord::Loading))
        {
            return Err(required(
                "current membership shell must be held by its registered Load",
            ));
        }
        catalog.require_source_owner().map_err(required)?;
        let source_pc = {
            let bank = catalog.definition_bank.try_borrow().map_err(required)?;
            let source = bank.combat_source().map_err(required)?;
            if source.source_identities() != catalog.actors.sources.as_slice() {
                return Err(required("membership source FP owner differs from Catalog"));
            }
            source.source_precision()
        };
        let definition = record.definition;
        let canonical = catalog
            .actors
            .actors
            .iter()
            .find(|d| d.id.eq_ignore_ascii_case(&definition.proof.id))
            .ok_or_else(|| required("canonical membership definition is absent"))?;
        if canonical.source != definition.proof.source
            || canonical.template_chain != definition.proof.template_chain
            || !same_proof(&record.shell.thing.definition, &definition.proof)
            || definition.proof.category.runtime_class != definition.native_class
            || catalog
                .actors
                .native_category(&canonical.id)
                .map_err(required)?
                != definition.proof.category
            || catalog
                .actors
                .native_final_flags(&canonical.id, source_pc)
                .map_err(required)?
                != definition.flags
        {
            return Err(required("membership shell/source/class provenance differs"));
        }
        if matches!(definition.native_class, 3 | 4)
            && definition.component_mask
                != Some(
                    catalog
                        .canonical_component_mask(&canonical.id, source_pc)
                        .map_err(required)?,
                )
        {
            return Err(required(
                "membership ActorData component mask differs from source",
            ));
        }
        let source = catalog
            .actors
            .native_object_definition(&canonical.id, source_pc)
            .map_err(required)?;
        if source.source != definition.proof.source {
            return Err(required("membership geometry source differs"));
        }
        let token = catalog
            .definition_bank
            .try_borrow_mut()
            .map_err(required)?
            .intern("ActorDefinition", &canonical.id)
            .map_err(required)?;
        if let Some(old) = self.definitions.get(&token) {
            if old.source.id != source.id
                || old.source.source != source.source
                || old.source.blocking != source.blocking
                || old.source.soft != source.soft
                || old.source.target != source.target
                || old.source.has_explicit_target != source.has_explicit_target
                || old.definition_flags != definition.flags
            {
                return Err(required("membership immutable definition binding changed"));
            }
        } else {
            self.definitions.insert(
                token,
                Arc::new(SourceGeometry {
                    definition: token,
                    source,
                    definition_flags: definition.flags,
                }),
            );
        }
        if let Some(old) = self.bindings.iter().find(|b| b.id == record.owner) {
            if old.definition != token
                || old.native_class != definition.native_class
                || !same_proof(&old.proof, &definition.proof)
            {
                return Err(required("membership identity was rebound"));
            }
            return Ok(old.key);
        }
        let key = self.next_key;
        let next = key
            .checked_add(1)
            .ok_or_else(|| required("load spatial key representation exhausted"))?;
        self.index
            .register_load_reference(key, record.owner.raw())?;
        self.bindings.push(Binding {
            id: record.owner,
            key,
            definition: token,
            native_class: definition.native_class,
            proof: definition.proof.clone(),
        });
        self.next_key = next;
        Ok(key)
    }
    fn fields(
        &self,
        binding: &Binding,
        shell: &LoadShell<DefinitionProof, RestoredComponent>,
        definition: &RuntimeDefinition,
    ) -> Result<Member> {
        let geometry = self
            .definitions
            .get(&binding.definition)
            .ok_or_else(|| required("membership immutable geometry unavailable"))?;
        if definition.native_class != binding.native_class
            || definition.flags != geometry.definition_flags
            || definition.proof.source != geometry.source.source
            || !definition
                .proof
                .id
                .eq_ignore_ascii_case(&geometry.source.id)
            || !same_proof(&shell.thing.definition, &definition.proof)
            || !same_proof(&binding.proof, &definition.proof)
            || !ordinary_layout(
                binding.native_class,
                shell.kind,
                shell.object.is_some(),
                shell.actor.is_some(),
            )
        {
            return Err(required("prior membership record/source identity changed"));
        }
        let object = shell
            .object
            .as_ref()
            .ok_or_else(|| required("Object membership shell has no Object fields"))?;
        Ok(Member {
            key: binding.key,
            reference: binding.id.raw(),
            actor: Actor {
                definition: binding.definition,
                flags: shell.thing.flags,
                position: [shell.thing.position_bits[0], shell.thing.position_bits[1]],
                facing: [shell.thing.facing_bits[0], shell.thing.facing_bits[1]],
            },
            height_bits: shell.thing.position_bits[2],
            facing_z_bits: shell.thing.facing_bits[2],
            cached_tile: [object.word_44 as i32, object.word_48 as i32],
            blocking: shape(&geometry.source.blocking),
            soft: shape(&geometry.source.soft),
            target: shape(if geometry.source.has_explicit_target {
                &geometry.source.target
            } else {
                &geometry.source.soft
            }),
            actor_category: matches!(binding.native_class, 3 | 4),
            wall_present: shell
                .actor
                .as_ref()
                .is_some_and(|a| a.components.present(ComponentSlot::Wall)),
        })
    }
    /// Exact admitted classes2/3/4/6/7/9 share constant-true+B0 and immutable source
    /// blocking/soft getters.5536B0/553830 have no outward world callbacks.
    pub fn refresh_membership(
        &mut self,
        catalog: &Catalog<'_>,
        record: &mut Record<'_>,
    ) -> Result<()> {
        let key = self.prepare(catalog, record)?;
        let mut fields = Vec::with_capacity(self.bindings.len());
        for binding in &self.bindings {
            let (shell, definition) = if binding.id == record.owner {
                (&*record.shell, record.definition)
            } else {
                record_fields(
                    &record
                        .registry
                        .get(binding.id)
                        .ok_or_else(|| required("indexed generation is not current"))?
                        .value,
                )?
            };
            fields.push(self.fields(binding, shell, definition)?);
        }
        let mut lease = self.index.lease_load_fields(fields)?;
        let result = lease.index_mut().refresh_membership(key, true);
        // The membership kernel retains the current member throughout. Commit
        // flags even if an error followed list writes or the native bit26 store.
        record.shell.thing.flags = lease.index().member(key)?.actor.flags;
        drop(lease);
        result.map_err(Error::from)
    }

    /// Synchronous query over real current/prior fields. The operation may use
    /// only immutable Record/registry/source access and the index's own native
    /// query/mark operations; no outward mutable world callback is permitted.
    /// Every destination/source/generation is checked BEFORE leasing. All
    /// candidate flag writes are then committed on both success and error.
    /// Outside this scope only native index nodes/marks/GTIDs remain resident.
    pub(crate) fn with_readonly_query<R>(
        &mut self,
        catalog: &Catalog<'_>,
        record: &mut Record<'_>,
        operation: impl FnOnce(
            &mut SpatialIndex,
            QueryBindings<'_>,
            &Record<'_>,
        ) -> aa_spatial::Result<R>,
    ) -> Result<R> {
        self.with_query_held(catalog, record, None, |index, bindings, record, _| {
            operation(index, bindings, record)
        })
    }

    pub(crate) fn with_readonly_query_extra<R>(
        &mut self,
        catalog: &Catalog<'_>,
        record: &mut Record<'_>,
        extra: &mut HeldShell<'_>,
        operation: impl FnOnce(&mut SpatialIndex, QueryBindings<'_>, &Record<'_>, HeldView<'_>) -> aa_spatial::Result<R>,
    ) -> Result<R> {
        self.with_query_held(catalog, record, Some(extra), |index,bindings,record,extra| {
            operation(index,bindings,record,extra.expect("requested actual extra held shell"))
        })
    }

    fn with_query_held<R>(
        &mut self,
        catalog: &Catalog<'_>,
        record: &mut Record<'_>,
        mut extra: Option<&mut HeldShell<'_>>,
        operation: impl FnOnce(&mut SpatialIndex, QueryBindings<'_>, &Record<'_>, Option<HeldView<'_>>) -> aa_spatial::Result<R>,
    ) -> Result<R> {
        if let Some(extra)=extra.as_deref_mut() {
            if extra.owner==record.owner {
                return Err(required("query extra held owner duplicates current allocation"));
            }
            let extra_record=Record { owner:extra.owner,shell:&mut *extra.shell,
                definition:extra.definition,registry:&mut *record.registry };
            self.prepare(catalog,&extra_record)?;
        }
        self.prepare(catalog, record)?;
        let mut fields = Vec::with_capacity(self.bindings.len());
        for binding in &self.bindings {
            let (shell, definition) = if binding.id == record.owner {
                (&*record.shell, record.definition)
            } else if let Some(extra)=extra.as_deref().filter(|e|e.owner==binding.id) {
                (&*extra.shell,extra.definition)
            } else {
                record_fields(
                    &record
                        .registry
                        .get(binding.id)
                        .ok_or_else(|| required("query indexed generation is not current"))?
                        .value,
                )?
            };
            fields.push(self.fields(binding, shell, definition)?);
        }
        let bindings = QueryBindings {
            bindings: &self.bindings,
            definitions: &self.definitions,
        };
        let mut lease = self.index.lease_load_fields(fields)?;
        let result = operation(lease.index_mut(), bindings, &*record, extra.as_deref().map(HeldShell::view));
        let mut flags = Vec::with_capacity(self.bindings.len());
        let mut missing = Vec::new();
        for binding in &self.bindings {
            match lease.index().member(binding.key) {
                Ok(member) => flags.push((binding.id, member.actor.flags)),
                Err(error) => missing.push(error.to_string()),
            }
        }
        // Record was immutably borrowed throughout the closed operation:
        // generations/variants/runtime allocations cannot have changed. Thus
        // these prevalidated writebacks cannot fail after earlier flag stores.
        for (id, flags) in flags {
            if id == record.owner {
                record.shell.thing.flags = flags;
            } else if let Some(extra)=extra.as_deref_mut().filter(|e|e.owner==id) {
                extra.shell.thing.flags=flags;
            } else {
                match record
                    .registry
                    .value_mut(id)
                    .expect("prevalidated query generation")
                {
                    RestoredRecord::Thing(v)
                    | RestoredRecord::Object(v)
                    | RestoredRecord::Actor(v) => v.shell.thing.flags = flags,
                    RestoredRecord::Kingdom(v) => {
                        v.runtime
                            .as_mut()
                            .expect("prevalidated Kingdom query shell")
                            .base
                            .thing
                            .flags = flags
                    }
                    _ => unreachable!(
                        "prevalidated query storage cannot change through immutable policy"
                    ),
                }
            }
        }
        drop(lease);
        if !missing.is_empty() {
            let prefix = result.err().map(|e| format!("{e}; ")).unwrap_or_default();
            return Err(required(format!(
                "{prefix}query removed required transient fields: {}",
                missing.join("; ")
            )));
        }
        result.map_err(Error::from)
    }
}

#[cfg(test)]
#[path = "load_spatial_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "load_spatial_query_tests.rs"]
mod query_tests;
