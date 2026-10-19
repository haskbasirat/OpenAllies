//! Direct field leases for7A0D40 over the application's sole constructed SAI.
//! The owner is moved from construction through loading; activation never
//! constructs a replacement manager or copies a startup State back on success.
use aa_mind::sai::{command::SaiCommandData, manager::{Sai, SaiRng}};
use aa_navigation::startup;
use std::collections::VecDeque;

#[cfg(test)]
#[path = "load_sai_world_tests.rs"]
mod tests;

/// Exactly the manager fields read/written by the startup coordinator. These
/// references remain the authoritative fields even if a later child fails.
pub struct Fields<'a> {
    ready: &'a mut bool,
    revision: &'a mut Option<u32>,
    word6c: &'a mut u32,
    word70: &'a mut u32,
}
impl startup::Fields for Fields<'_> {
    fn ready(&self) -> bool { *self.ready }
    fn set_ready(&mut self, value: bool) { *self.ready = value; }
    fn set_world_revision(&mut self, value: u32) { *self.revision = Some(value); }
    fn set_word6c(&mut self, value: u32) { *self.word6c = value; }
    fn set_word70(&mut self, value: u32) { *self.word70 = value; }
}

/// Disjoint actual collections available to the concrete live-world services.
/// Entries are neither cloned nor inferred from the current map/player UI.
pub struct Collections<'a> {
    pub players: &'a mut Vec<Option<u32>>,
    pub players_4c: &'a mut VecDeque<u32>,
    pub players_54: &'a mut VecDeque<u32>,
    pub commands: &'a mut VecDeque<(u32, SaiCommandData)>,
    pub general_powers: &'a mut Vec<u32>,
    pub rng: &'a mut SaiRng,
    pub vector90:&'a mut Vec<u32>,
}

/// Lease the same manager to the native kernel and its concrete world. The
/// factory only separates borrows; it supplies no missing startup behavior.
pub fn initialize_manager<'a, W: startup::World<Fields<'a>>>(
    manager: &'a mut Sai,
    make_world: impl FnOnce(Collections<'a>) -> W,
) -> startup::Result<()> {
    let mut fields = Fields {
        ready: &mut manager.ready,
        revision: &mut manager.revision,
        word6c: &mut manager.word_6c,
        word70: &mut manager.word_70,
    };
    let mut world = make_world(Collections {
        players: &mut manager.players,
        players_4c: &mut manager.players_4c,
        players_54: &mut manager.players_54,
        commands: &mut manager.commands,
        general_powers: &mut manager.general_powers,
        rng: &mut manager.rng,
        vector90:&mut manager.vector_90,
    });
    startup::initialize_fields(&mut fields, &mut world)
}

/// The SAME started manager split into the startup field lease and its
/// disjoint collections, for 7A0C20 classifications reached on the live World
/// (a creation loan). No manager field is copied.
pub(crate) fn split_live(manager: &mut Sai) -> (Fields<'_>, Collections<'_>) {
    (
        Fields {
            ready: &mut manager.ready,
            revision: &mut manager.revision,
            word6c: &mut manager.word_6c,
            word70: &mut manager.word_70,
        },
        Collections {
            players: &mut manager.players,
            players_4c: &mut manager.players_4c,
            players_54: &mut manager.players_54,
            commands: &mut manager.commands,
            general_powers: &mut manager.general_powers,
            rng: &mut manager.rng,
            vector90: &mut manager.vector_90,
        },
    )
}

/// Sole loaded SAI-map owner. The topology's tile_rectangles are native tile
/// +10 identities; ZoneMap owns the disjoint +4/+14/+18 fields of those tiles.
/// No Clone/Default/raw topology constructor is provided by this adapter.
pub struct Map {
    topology: aa_navigation::topology::Topology,
    post_load: Option<aa_navigation::topology::PostLoad>,
    auxiliary_consumed: usize,
    lifecycle: aa_mind::sai::manager::SaiMapLifecycle,
    graph: Option<aa_navigation::region_search::RegionGraph>,
    regions: aa_navigation::sai_state::State,
    source_files: Vec<aa_rules::resources::SourceIdentity>,
    source_precision: aa_numeric::Precision,
    map_source: aa_mapworld::SourceIdentity,
    seen_store:Option<crate::actor_world::ActorStoreWitness>,
    seen:std::collections::BTreeMap<u64,(aa_actors::ActorId,aa_actors::detection_memory::RecordId)>,
    contexts:std::collections::BTreeMap<u64,String>,
    seen_contexts:std::collections::BTreeMap<u64,u64>,
    next_seen:u64,
    startup_attempted:bool,
    region_actors:Vec<crate::load_sai_lists::RegionLists>,
}
fn source_precision(catalog: &crate::catalog::Catalog<'_>) -> Result<aa_numeric::Precision, String> {
    catalog.require_source_owner()?;
    let bank = catalog.definition_bank.try_borrow().map_err(|e| e.to_string())?;
    source_precision_in_bank(catalog, &bank)
}
fn source_precision_in_bank(catalog: &crate::catalog::Catalog<'_>, bank:&crate::definitions::Bank) -> Result<aa_numeric::Precision, String> {
    catalog.require_source_owner()?;
    let owner = bank.combat_source()?;
    owner.completed_means()?;
    if owner.source_identities() != catalog.actors.sources {
        return Err("SAI map source Bank/catalog identities differ".into());
    }
    Ok(owner.source_precision())
}
impl Map {
    ///7AA9B0 uses the untouched TERRAIN suffix, after the actual GMap tiles
    /// load and before TEAMS/KINGDOMS. Older worlds require7ACB00; no substitute
    /// topology is inferred from heights, saved explored masks or render data.
    pub fn load(
        catalog: &crate::catalog::Catalog<'_>,
        archive: &aa_mapworld::MapArchive,
        terrain: &crate::battle_terrain::BattleTerrain,
        index: &aa_spatial::index::SpatialIndex,
        zones: &aa_zones::tile::ZoneMap,
        precision: aa_numeric::Precision,
    ) -> Result<Self, String> {
        use aa_mapworld::{header::MapHeader, terrain::TerrainChunk};
        use aa_navigation::topology::{GridLayout, StaticTile, Topology};
        let pc = source_precision(catalog)?;
        if terrain.source_precision() != pc {
            return Err("SAI map terrain/source numeric owner differs".into());
        }
        let header = MapHeader::decode(&archive.decode_chunk("MAP_HEADER", u64::MAX)
            .map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
        let expected_zones = crate::load_zone_services::before_first_record(&terrain.bound, index, &header)
            .map_err(|e|e.to_string())?;
        if zones.geometry != expected_zones.geometry || zones.tiles != expected_zones.tiles {
            return Err("SAI map load requires same pre-record GMap zone fields".into());
        }
        let decoded = TerrainChunk::decode(&archive.decode_chunk("TERRAIN", u64::MAX)
            .map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
        if decoded.map_source != terrain.terrain.map_source
            || decoded.source != terrain.terrain.source
            || decoded.pending_auxiliary_bytes != terrain.terrain.pending_auxiliary_bytes
            || decoded.serialized_tile_bytes != terrain.terrain.serialized_tile_bytes
            || decoded.suffix_offset != terrain.terrain.suffix_offset {
            return Err("SAI map terrain suffix belongs to a different source".into());
        }
        let world = aa_mapworld::world::WorldState::decode(&archive.decode_chunk("GWORLD", u64::MAX)
            .map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
        let layout = terrain.bound.layout;
        if terrain.base.state.layout.extent_i32 != layout.extent_i32
            || terrain.overlay.state.layout.extent_i32 != layout.extent_i32 {
            return Err("SAI map height/terrain constructed extents differ".into());
        }
        for (name,field) in [("HEIGHT_MAP",&terrain.base),("MOUNTAIN_HEIGHT_MAP",&terrain.overlay)] {
            if archive.decode_chunk(name,u64::MAX).map_err(|e|e.to_string())?.payload_source.sha256
                != field.chunk_sha256 {
                return Err("SAI map height source belongs to another map".into());
            }
        }
        let (topology, auxiliary_consumed) = Topology::decode_prefix(
            &decoded.pending_auxiliary_bytes,
            GridLayout {width:layout.width,height:layout.height,
                coordinate_shift:layout.coordinate_shift,
                extent_bits:layout.extent_i32.map(|n|(n as f32).to_bits())},
            world.version,
        ).map_err(|e|e.to_string())?;
        let mut tiles = Vec::new();
        tiles.try_reserve_exact(terrain.bound.cells.len()).map_err(|e|e.to_string())?;
        for y in 0..layout.height {
            for x in 0..layout.width {
                tiles.push(StaticTile {flags20:index.tile([x,y]).map_err(|e|e.to_string())?.flags20,
                    definition_type:terrain.terrain_definition_at(x,y)?.native_type});
            }
        }
        let post_load = topology.post_load(&tiles, precision).map_err(|e|e.to_string())?;
        let regions = aa_navigation::sai_state::State::new(topology.regions.len()).map_err(|e|e.to_string())?;
        let mut region_actors=Vec::new();
        region_actors.try_reserve_exact(topology.regions.len()).map_err(|e|e.to_string())?;
        for _ in &topology.regions {region_actors.push(crate::load_sai_lists::RegionLists::constructed()?);}
        let lifecycle = aa_mind::sai::manager::SaiMapLifecycle {
            analysed:topology.enabled, dynamic:false, areas:Vec::new(),
            bottlenecks:Vec::new(), route_cache:None,
        };
        Ok(Self {topology,post_load,auxiliary_consumed,lifecycle,graph:None,regions,
            source_files:catalog.actors.sources.clone(),source_precision:pc,
            map_source:archive.source().clone(),seen_store:None,seen:Default::default(),
            contexts:Default::default(),seen_contexts:Default::default(),next_seen:1,startup_attempted:false,region_actors})
    }
    pub fn topology(&self) -> &aa_navigation::topology::Topology { &self.topology }
    pub fn post_load(&self) -> Option<aa_navigation::topology::PostLoad> { self.post_load }
    pub fn auxiliary_consumed(&self) -> usize { self.auxiliary_consumed }
    pub fn lifecycle(&self) -> &aa_mind::sai::manager::SaiMapLifecycle { &self.lifecycle }
    pub fn graph(&self) -> Option<&aa_navigation::region_search::RegionGraph> { self.graph.as_ref() }
    pub fn regions(&self) -> &aa_navigation::sai_state::State { &self.regions }
    pub fn map_source(&self) -> &aa_mapworld::SourceIdentity { &self.map_source }
    pub(crate) fn bind_startup_actors(&mut self,catalog:&crate::catalog::Catalog<'_>,bank:&crate::definitions::Bank,
        inventory:&aa_mapworld::WorldInventory,zones:&aa_zones::tile::ZoneMap,actors:&crate::actor_world::ActorStore)->Result<(),String> {
        self.validate(catalog,bank,zones)?;
        if inventory.map_source!=self.map_source {return Err("SAI startup inventory belongs to another map source".into())}
        if self.seen_store.as_ref().is_some_and(|w|!w.is_for(actors)) {return Err("SAI startup ActorStore belongs to another world allocation".into())}
        if self.seen_store.is_none() {self.seen_store=Some(actors.identity_witness());}
        Ok(())
    }
    /// An attempted full startup is never converted into success by its native
    /// ready-byte prefix on a later call. This is a binding capability guard.
    pub(crate) fn begin_startup(&mut self)->Result<(),String> {
        if self.startup_attempted {return Err("SAI world startup was already attempted; retain its partial owner state".into())}
        self.startup_attempted=true;Ok(())
    }
    pub(crate) fn prepend_actor(&mut self,region:u32,water:bool,index:usize,actor:u32)->Result<(),String> {
        self.region_actors.get_mut(region as usize).ok_or("SAI current actor region is absent")?.prepend(water,index,actor)
    }
    fn validate(&self, catalog: &crate::catalog::Catalog<'_>, bank:&crate::definitions::Bank, zones:&aa_zones::tile::ZoneMap) -> Result<(),String> {
        let pc=source_precision_in_bank(catalog, bank)?;
        let layout=self.topology.layout;
        if pc!=self.source_precision || self.source_files!=catalog.actors.sources
            || zones.geometry.width!=layout.width || zones.geometry.height!=layout.height
            || zones.geometry.shift!=layout.coordinate_shift || zones.geometry.extent_bits!=layout.extent_bits
            || zones.geometry.tile_size_bits!=(127+layout.coordinate_shift)<<23
            || zones.tiles.len()!=self.topology.tile_rectangles.len() {
            return Err("SAI map current source/GMap owner differs".into());
        }
        Ok(())
    }
    /// Concrete loaded-map7ABC30 preparation. The dynamic flag is written
    /// before any fallible child; readiness remains solely in the SAI manager.
    pub fn prepare(&mut self, catalog:&crate::catalog::Catalog<'_>, bank:&crate::definitions::Bank,
        zones:&aa_zones::tile::ZoneMap) -> Result<(),String> {
        self.validate(catalog,bank,zones)?;
        if self.lifecycle.dynamic {
            return Err("SAI loaded-map preparation was already attempted; preserve its native prefix".into());
        }
        self.lifecycle.dynamic=true;
        self.graph=Some(self.topology.prepare_graph().map_err(|e|e.to_string())?);
        let count=self.topology.regions.len();
        self.lifecycle.areas.try_reserve_exact(count).map_err(|e|e.to_string())?;
        self.lifecycle.areas.resize(count,0);
        for id in (0..count).rev() {
            self.lifecycle.areas[id]=self.topology.region_area(id).map_err(|e|e.to_string())?;
        }
        self.lifecycle.bottlenecks.try_reserve_exact(count).map_err(|e|e.to_string())?;
        self.lifecycle.bottlenecks.resize(count,false);
        //7BC8B0 first resolves the SVar on the first actual region call.
        //No region means no such read; its unused depth cannot affect output.
        let depth=if self.topology.regions.is_empty() {0} else {
            let name="StaticMapBottleneckDetectionDepth";
            if catalog.actors.native_data_find("WorldRule",name).is_some() {
                return Err("SAI bottleneck WorldRule source owner is unbound".into());
            }
            let mut values=Vec::new();
            for node in catalog.actors.resolved_definitions("SVars").map_err(|e|e.to_string())? {
                if let Some(value)=node.field(name,self.source_precision).map_err(|e|e.to_string())? {
                    let aa_rules::native::TypedValue::Integer {value,source,..}=value else {
                        return Err("SAI bottleneck source declaration is not native Integer".into());
                    };
                    values.push((value,source));
                }
            }
            let [(depth,source)]=values.as_slice() else {
                return Err("SAI bottleneck depth needs its unique typed source assignment".into());
            };
            let rows=catalog.settings.as_ref().ok_or("SAI bottleneck Settings owner is unbound")?
                .declarations.get(&name.to_ascii_lowercase()).ok_or("SAI bottleneck Settings assignment is unbound")?;
            let [row]=rows.as_slice() else {return Err("SAI bottleneck Settings override requires its actual publication owner".into())};
            if &row.source!=source || !row.field_type.as_deref().is_some_and(|s|s.eq_ignore_ascii_case("int"))
                || row.raw.trim().parse::<i32>().ok()!=Some(*depth) {
                return Err("SAI bottleneck Settings raw value/type/span differs from its registered source".into());
            }
            *depth
        };
        aa_navigation::preparation::mark_bottlenecks(self.graph.as_ref().unwrap(),
            &mut self.lifecycle.bottlenecks,depth).map_err(|e|e.to_string())?;
        self.lifecycle.route_cache=Some(aa_navigation::route_cache::RouteCache::build(count)
            .map_err(|e|e.to_string())?);
        Ok(())
    }
    ///79E170's concrete descending region phase over current Kingdom storage
    /// and live GMap fog/supply fields. The caller must then perform its actual
    /// AI-controller notifications; this method does not stand for that loop.
    #[allow(clippy::too_many_arguments)]
    pub fn register_kingdom_regions(
        &mut self,catalog:&crate::catalog::Catalog<'_>,bank:&crate::definitions::Bank,
        inventory:&aa_mapworld::WorldInventory,actors:&crate::actor_world::ActorStore,
        kingdoms:&crate::load_kingdom_init_world::KingdomRegistryState,
        zones:&aa_zones::tile::ZoneMap,id:aa_actors::ActorId,
    ) -> Result<aa_navigation::sai_state::Kingdom,String> {
        use aa_navigation::{sai_state::{Kingdom,TileKnowledge,VisibilitySettings},startup::Object};
        self.validate(catalog,bank,zones)?;
        if inventory.map_source!=self.map_source || !self.lifecycle.dynamic || self.graph.is_none() {
            return Err("SAI Kingdom regions require this loaded/prepared map owner".into());
        }
        let key=actors.key(id).ok_or("SAI Kingdom current generation is absent")?;
        let object=actors.object(key).map_err(|e|e.to_string())?;
        let crate::actor_world::Source::Special(special)=&actors.entity(key).map_err(|e|e.to_string())?.source else {
            return Err("SAI Kingdom is outside actual derived storage".into());
        };
        let crate::actor_world::SpecialSource::Kingdom(saved)=&special.source else {
            return Err("SAI Kingdom has a different saved constructor class".into());
        };
        let proof=&object.definition.proof;
        catalog.require_untranslated_actor_identity(&proof.id)?;
        let canonical=catalog.actors.find(&proof.id).ok_or("SAI Kingdom canonical source is absent")?;
        let token=bank.token("ActorDefinition",&proof.id).ok_or("SAI Kingdom source token is absent")?;
        let source=bank.combat_source()?;
        source.completed_values(token)?;
        if saved.serialized_id!=id.raw() || object.definition.native_class!=4
            || object.kind!=aa_actors::loading::LoadKind::Actor || object.actor.is_none() || object.object.is_none()
            || canonical.source!=proof.source || canonical.template_chain!=proof.template_chain
            || catalog.actors.native_category(&proof.id).map_err(|e|e.to_string())?!=proof.category
            || !crate::load_spatial::same_proof(&object.thing.definition,proof)
            || !source.definition_id(token)?.eq_ignore_ascii_case(&proof.id)
            || source.definition_source(token)?!=&proof.source
            || object.definition.component_mask!=Some(catalog.canonical_component_mask(&proof.id,self.source_precision)?) {
            return Err("SAI Kingdom source/constructor ownership differs".into());
        }
        let derived=special.derived.as_ref().ok_or("SAI Kingdom derived allocation is absent")?;
        let word=|offset|derived.words.get(&offset).copied().ok_or_else(||format!("SAI Kingdom+{offset:X} is unavailable"));
        let all_index=word(0x188)? as usize;
        let major_index=word(0x18c)? as i32;
        let category=word(0x190)?;
        if kingdoms.all().get(all_index)!=Some(&Some(id))
            || (category==0 && (major_index<0 || kingdoms.majors().get(major_index as usize)!=Some(&Some(id)))) {
            return Err("SAI Kingdom current world slot owner differs".into());
        }
        let kingdom=Kingdom {object:Object(u64::from(id.raw())),native_id:id.raw(),major_index,category};
        let tiles=zones.tiles.iter().map(|t|TileKnowledge {word14:t.fog,word18:t.supply}).collect::<Vec<_>>();
        let fog=crate::saved_action_settings::fog_of_war(catalog,inventory,self.source_precision)?;
        self.regions.register_kingdom_regions(kingdom,&self.topology,&tiles,
            VisibilitySettings {fog_enabled:fog,visible_bit_offset:aa_zones::tile::VISIBLE_SHIFT})
            .map_err(|e|e.to_string())?;
        Ok(kingdom)
    }
    pub(crate) fn remember_memory(&mut self,actors:&crate::actor_world::ActorStore,
        actor:aa_actors::ActorId,record:aa_actors::detection_memory::RecordId)->Result<startup::Object,String> {
        if self.seen_store.as_ref().is_some_and(|w|!w.is_for(actors)) {
            return Err("SAI seen references belong to another ActorStore allocation".into());
        }
        if self.seen_store.is_none() {self.seen_store=Some(actors.identity_witness());}
        let key=actors.key(actor).ok_or("SAI seen actor generation is absent")?;
        let memory=&actors.object(key).map_err(|e|e.to_string())?.object.as_ref().ok_or("SAI seen Object allocation is absent")?.memory;
        let definition=memory.record(record).map_err(|e|e.to_string())?.definition.as_ref().ok_or("SAI seen record source context is null")?;
        let context=match self.contexts.iter().find(|(_,v)|v.eq_ignore_ascii_case(definition)) {
            Some((&key,_))=>key,
            None=>{let key=u64::try_from(self.contexts.len()).map_err(|e|e.to_string())?.checked_add(1).ok_or("SAI context identity exhausted")?;
                self.contexts.insert(key,definition.clone());key},
        };
        let id=match self.seen.iter().find(|(_,v)|**v==(actor,record)) {
            Some((&id,_))=>id,
            None=>{let id=self.next_seen;self.next_seen=id.checked_add(1).ok_or("SAI seen reference identity exhausted")?;id},
        };
        self.seen.insert(id,(actor,record));self.seen_contexts.insert(id,context);Ok(startup::Object(id))
    }
    pub(crate) fn memory_key(&self,actors:&crate::actor_world::ActorStore,id:startup::Object)
        ->Result<(aa_actors::ActorId,aa_actors::detection_memory::RecordId),String> {
        if self.seen_store.as_ref().is_none_or(|w|!w.is_for(actors)) {return Err("SAI seen references have another ActorStore owner".into())}
        self.seen.get(&id.0).copied().ok_or_else(||"SAI seen record identity is absent".into())
    }
    pub(crate) fn memory_context(&self,id:startup::Object)->Result<startup::Object,String> {
        self.seen_contexts.get(&id.0).copied().map(startup::Object).ok_or_else(||"SAI seen source-context identity is absent".into())
    }
    pub(crate) fn context_definition(&self,id:startup::Object)->Result<&str,String> {
        self.contexts.get(&id.0).map(String::as_str).ok_or_else(||"SAI source-context definition is absent".into())
    }
    pub(crate) fn register_seen_ref(&mut self,region:u32,kingdom:aa_actors::ActorId,record:startup::Object)->Result<(),String> {
        let state=self.regions.regions.get_mut(region as usize).ok_or("SAI seen region absent")?;
        let row=state.kingdoms.iter_mut().flatten().find(|r|r.kingdom.0==u64::from(kingdom.raw())).ok_or("SAI seen region-Kingdom owner absent")?;
        row.register_seen(record).map_err(|e|e.to_string())
    }
}
