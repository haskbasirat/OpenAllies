//! Pre-staging membership (4EE4C0/5536B0) over actual held load shells.
//! Only tile/node/generation state and immutable source bindings persist.
//! Member fields exist within one synchronous lease, are committed back on
//! success/error, then discarded. No outward callback can see stale fields.
use crate::{catalog::Catalog, live_object_init::Record, spatial_world::SourceGeometry};
use aa_actors::{
    components::ComponentSlot,
    loading::{LoadKind, LoadShell},
    ActorId,
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
    /// Immutable definition-completion PC, independent of live index arithmetic.
    source_precision: Precision,
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
    /// The last `fields` result of each other held record, with the record's
    /// registry version at that time. `fields` reads only the binding and its
    /// geometry (both immutable once created) and that record, so while the
    /// version is unchanged the result is the same (see `member_fields`).
    /// Indexed by binding key (dense: next_key counts up from 1).
    member_cache: Vec<Option<((u64, u64), Member)>>,
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
        HeldView {
            owner: self.owner,
            shell: self.shell,
            definition: self.definition,
        }
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
            member_cache: Vec::new(),
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
            if !crate::catalog_lookups::same_sources(catalog, source.source_identities()) {
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
                || old.source_precision != source_pc
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
            source_precision: source_pc,
        });
        self.next_key = next;
        Ok(key)
    }
    /// `fields` of another binding's held registry record. A result is reused
    /// only while that record's registry version is unchanged, i.e. the
    /// record has not been installed, removed, swapped or mutably borrowed
    /// since: then every input of `fields` is unchanged. Errors are never
    /// cached; they are recomputed in binding order like before.
    fn member_fields(
        &self,
        cache: &mut Vec<Option<((u64, u64), Member)>>,
        binding: &Binding,
        registry: &aa_actors::registry::IdentityRegistry<RestoredRecord>,
        stale: &str,
    ) -> Result<Member> {
        self.ensure_member_fields(cache, binding, registry, stale)?;
        Ok(cache[binding.key as usize].as_ref().expect("ensured binding fields").1.clone())
    }
    /// Leaves `cache[binding.key]` holding exactly what `member_fields` returns.
    fn ensure_member_fields(
        &self,
        cache: &mut Vec<Option<((u64, u64), Member)>>,
        binding: &Binding,
        registry: &aa_actors::registry::IdentityRegistry<RestoredRecord>,
        stale: &str,
    ) -> Result<()> {
        let entry = registry.get(binding.id).ok_or_else(|| required(stale))?;
        let version = registry
            .version(binding.id)
            .expect("a current registry entry has a version");
        let slot = binding.key as usize;
        if cache
            .get(slot)
            .and_then(Option::as_ref)
            .is_some_and(|(cached, _)| *cached == version)
        {
            return Ok(());
        }
        let (shell, definition) = record_fields(&entry.value)?;
        let member = self.fields(binding, shell, definition)?;
        if cache.len() <= slot {
            cache.resize_with(slot + 1, || None);
        }
        cache[slot] = Some((version, member));
        Ok(())
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
        let mut cache = std::mem::take(&mut self.member_cache);
        //Same fields, computed (or reused, see member_fields) in binding order
        //with the same first error; the lease then borrows them in that order.
        let built = (|| -> Result<Option<Member>> {
            let mut own = None;
            for binding in &self.bindings {
                if binding.id == record.owner {
                    own = Some(self.fields(binding, &*record.shell, record.definition)?);
                } else {
                    self.ensure_member_fields(
                        &mut cache,
                        binding,
                        record.registry,
                        "indexed generation is not current",
                    )?;
                }
            }
            Ok(own)
        })();
        let own = match built {
            Ok(own) => own,
            Err(error) => {
                self.member_cache = cache;
                return Err(error);
            }
        };
        let leased = {
            let fields: Vec<&Member> = self
                .bindings
                .iter()
                .map(|binding| {
                    if binding.id == record.owner {
                        own.as_ref().expect("current binding fields were computed")
                    } else {
                        &cache[binding.key as usize]
                            .as_ref()
                            .expect("ensured binding fields")
                            .1
                    }
                })
                .collect();
            match self.index.lease_load_fields_by_ref(&fields) {
                Ok(mut lease) => {
                    let result = lease.index_mut().refresh_membership(key, true);
                    // The membership kernel retains the current member throughout. Commit
                    // flags even if an error followed list writes or the native bit26 store.
                    lease
                        .index()
                        .member(key)
                        .map(|member| member.actor.flags)
                        .map(|flags| (flags, result))
                }
                Err(error) => Err(error),
            }
        };
        self.member_cache = cache;
        let (flags, result) = leased?;
        record.shell.thing.flags = flags;
        result.map_err(Error::from)
    }

    /// Callback-free spatial half of4EE560(retain=false), before4EFEC0 retires
    /// the GLOBAL identity. Caller still owns the actual registered shell.
    /// Current fields are leased only for553830; successful/error prefixes are
    /// committed to that shell. Other-object rescans require actual bound fields
    /// and fail explicitly if reached by this bounded current-only lease.
    /// Auxiliary index identity and Binding are released only after removal and
    /// unregister succeed. This method never drops the record or retires GTID.
    pub(crate) fn detach_for_destruction(
        &mut self,
        catalog: &Catalog<'_>,
        id: ActorId,
        base: &mut LoadShell<DefinitionProof, RestoredComponent>,
        definition: &RuntimeDefinition,
        pc: Precision,
    ) -> Result<()> {
        if id.is_null() || self.index.precision() != pc {
            return Err(required(
                "destruction current identity/live spatial PC differs",
            ));
        }
        let position = self
            .bindings
            .iter()
            .position(|b| b.id == id)
            .ok_or_else(|| required("destruction object has no existing load spatial binding"))?;
        let binding = self.bindings[position].clone();
        let source =
            crate::live_object_init::Source::bind(catalog, definition).map_err(required)?;
        if source.source_precision() != binding.source_precision
            || self.index.resolve_reference(id.raw()) != Some(binding.key)
            || self.index.reference_for_key(binding.key)? != id.raw()
        {
            return Err(required(
                "destruction source PC/current index generation differs",
            ));
        }
        {
            let bank = catalog.definition_bank.try_borrow().map_err(required)?;
            let entry = bank.entry(binding.definition).map_err(required)?;
            if entry.key.domain != "actordefinition"
                || !entry.key.name.eq_ignore_ascii_case(&binding.proof.id)
            {
                return Err(required("destruction same Bank definition handle differs"));
            }
        }
        let fields = self.fields(&binding, base, definition)?;
        let mut lease = self.index.lease_load_fields(vec![fields])?;
        let removal = if lease.index().member(binding.key)?.actor.flags & (1 << 26) != 0 {
            //Known immutable getters form4CF210 blocking first,4EA0F0 soft
            //second;553830 consumes those same current pose/source shapes.
            lease.index_mut().remove(binding.key)
        } else {
            Ok(())
        };
        base.thing.flags = lease.index().member(binding.key)?.actor.flags;
        if let Err(error) = removal {
            return Err(error.into());
        }
        lease.index_mut().unregister(binding.key)?;
        drop(lease);
        self.bindings.remove(position);
        Ok(())
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
        operation: impl FnOnce(
            &mut SpatialIndex,
            QueryBindings<'_>,
            &Record<'_>,
            HeldView<'_>,
        ) -> aa_spatial::Result<R>,
    ) -> Result<R> {
        self.with_query_held(
            catalog,
            record,
            Some(extra),
            |index, bindings, record, extra| {
                operation(
                    index,
                    bindings,
                    record,
                    extra.expect("requested actual extra held shell"),
                )
            },
        )
    }

    fn with_query_held<R>(
        &mut self,
        catalog: &Catalog<'_>,
        record: &mut Record<'_>,
        mut extra: Option<&mut HeldShell<'_>>,
        operation: impl FnOnce(
            &mut SpatialIndex,
            QueryBindings<'_>,
            &Record<'_>,
            Option<HeldView<'_>>,
        ) -> aa_spatial::Result<R>,
    ) -> Result<R> {
        if let Some(extra) = extra.as_deref_mut() {
            if extra.owner == record.owner {
                return Err(required(
                    "query extra held owner duplicates current allocation",
                ));
            }
            let extra_record = Record {
                owner: extra.owner,
                shell: &mut *extra.shell,
                definition: extra.definition,
                registry: &mut *record.registry,
            };
            self.prepare(catalog, &extra_record)?;
        }
        self.prepare(catalog, record)?;
        let mut fields = Vec::with_capacity(self.bindings.len());
        let mut cache = std::mem::take(&mut self.member_cache);
        let built = (|| -> Result<()> {
            for binding in &self.bindings {
                if binding.id == record.owner {
                    fields.push(self.fields(binding, &*record.shell, record.definition)?);
                } else if let Some(extra) = extra.as_deref().filter(|e| e.owner == binding.id) {
                    fields.push(self.fields(binding, &*extra.shell, extra.definition)?);
                } else {
                    fields.push(self.member_fields(
                        &mut cache,
                        binding,
                        record.registry,
                        "query indexed generation is not current",
                    )?);
                }
            }
            Ok(())
        })();
        self.member_cache = cache;
        built?;
        let bindings = QueryBindings {
            bindings: &self.bindings,
            definitions: &self.definitions,
        };
        let mut lease = self.index.lease_load_fields(fields)?;
        let result = operation(
            lease.index_mut(),
            bindings,
            &*record,
            extra.as_deref().map(HeldShell::view),
        );
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
            } else if let Some(extra) = extra.as_deref_mut().filter(|e| e.owner == id) {
                extra.shell.thing.flags = flags;
            } else {
                // Storing an equal word changes nothing; skip the mutable
                // borrow then (same panics on the same impossible storage).
                let unchanged = match &record
                    .registry
                    .get(id)
                    .expect("prevalidated query generation")
                    .value
                {
                    RestoredRecord::Thing(v)
                    | RestoredRecord::Object(v)
                    | RestoredRecord::Actor(v) => v.shell.thing.flags == flags,
                    RestoredRecord::Kingdom(v) => {
                        v.runtime
                            .as_ref()
                            .expect("prevalidated Kingdom query shell")
                            .base
                            .thing
                            .flags
                            == flags
                    }
                    _ => unreachable!(
                        "prevalidated query storage cannot change through immutable policy"
                    ),
                };
                if unchanged {
                    continue;
                }
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

#[cfg(test)]
#[path = "load_spatial_teardown_tests.rs"]
mod teardown_tests;
