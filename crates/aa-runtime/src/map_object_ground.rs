//! Fresh4EB500 with the real nonnull ActorParams and SAME current map owners.
//! The shared spatial coordinator retains native read/callback order. The
//! height brush and positive paint/destruction children never default to Ok.
use crate::{
    actor_world::{ActorStore, ObjectKey, RuntimeComponent},
    catalog::Catalog,
    definitions::Bank,
    map_actor_creation::Params,
    restoration::Staged,
};
use aa_actors::{ActorId, components::ComponentSlot};
use aa_numeric::{Binary, Precision};
use aa_spatial::{
    Error, Result,
    geometry::Placement,
    position::{HeightEdit, ObjectEffects, TerrainPaint},
};
fn fail(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}

///Move-only representation of this creation call's native stack residue.
///42A5D3 writes current Z into the word later read at4EB500 entryESP-8.
///Only the genuine final4D7230 ExistingFX callback can produce this value.
pub(crate) struct CreationGroundScratch {
    owner: crate::actor_world::ActorStoreWitness,
    key: ObjectKey,
    id: ActorId,
    requested_id: u32,
    word_3c: u32,
    height: u32,
}
impl CreationGroundScratch {
    pub(crate) fn capture(
        staged: &Staged,
        key: ObjectKey,
        requested_id: u32,
        word_3c: u32,
    ) -> std::result::Result<Self, String> {
        if requested_id != word_3c {
            return Err("42A590 scratch capture requires actual equal Params24/3C gate".into());
        }
        let object = staged.actors.object(key).map_err(|e| e.to_string())?;
        let actor = object
            .actor
            .as_ref()
            .ok_or("42A590 scratch capture actual Actor absent")?;
        if object.definition.native_class != 3 || actor.components.state.low & (1 << 21) != 0 {
            return Err("42A590 scratch capture actual final ExistingFX gate differs".into());
        }
        let world = staged
            .load_world
            .as_ref()
            .ok_or("42A590 scratch actual World absent")?;
        let height = staged
            .actors
            .placement(key, world.live_spatial()?)
            .map_err(|e| e.to_string())?
            .position_bits[2];
        Ok(Self {
            owner: staged.actors.identity_witness(),
            key,
            id: staged
                .actors
                .id(key)
                .ok_or("42A590 scratch current GTID absent")?,
            requested_id,
            word_3c,
            height,
        })
    }
    fn consume(self, actors: &ActorStore, key: ObjectKey, params: &Params) -> Result<u32> {
        if !self.owner.is_for(actors)
            || self.key != key
            || actors.id(key) != Some(self.id)
            || params.requested_id != self.requested_id
            || params.word_3c != self.word_3c
        {
            return Err(fail(
                "5512BF native creation scratch belongs to another Actor/Params call",
            ));
        }
        Ok(self.height)
    }
}

pub(crate) fn run(
    staged: &mut Staged,
    catalog: &Catalog<'_>,
    key: ObjectKey,
    params: &Params,
    scratch: Option<CreationGroundScratch>,
    pc: Precision,
) -> std::result::Result<(), String> {
    crate::fresh_post_create_world::canonical(staged, catalog, key)?;
    let source = staged.definitions.combat_source()?;
    source.completed_means()?;
    if source.source_identities() != catalog.actors.sources.as_slice() {
        return Err("4EB500 moved Bank source differs".into());
    }
    let world = staged
        .load_world
        .as_ref()
        .ok_or("4EB500 actual persistent World absent")?;
    if world.terrain.source_precision() != source.source_precision()
        || world.source_fields.source_precision() != source.source_precision()
        || world.source_fields.receipt().sources != catalog.actors.sources
    {
        return Err("4EB500 current Terrain/source fields differ".into());
    }
    let crate::load_world::SpatialOwner::Adopted(adopted) = &world.spatial else {
        return Err("4EB500 actual adopted index absent".into());
    };
    if adopted.error.is_some()
        || !adopted.pending.is_empty()
        || adopted.world.index().precision() != pc
    {
        return Err("4EB500 incomplete spatial adoption or live precision differs".into());
    }
    let mut current = Current {
        staged,
        catalog,
        key,
        params,
        scratch,
        pc,
    };
    current.member().map_err(|e| e.to_string())?;
    aa_spatial::position::object_effects_on_owners(&mut current).map_err(|e| e.to_string())
}
struct Current<'a, 's> {
    staged: &'a mut Staged,
    catalog: &'a Catalog<'s>,
    key: ObjectKey,
    params: &'a Params,
    scratch: Option<CreationGroundScratch>,
    pc: Precision,
}
impl Current<'_, '_> {
    fn world(&self) -> Result<&crate::load_world::World> {
        self.staged
            .load_world
            .as_ref()
            .ok_or_else(|| fail("4EB500 current World absent"))
    }
    fn member(&self) -> Result<&aa_spatial::index::Member> {
        let world = self.world()?;
        let spatial = world.live_spatial().map_err(fail)?;
        let key = spatial.key(self.key).map_err(fail)?;
        let m = spatial.index().member(key)?;
        let id = self
            .staged
            .actors
            .id(self.key)
            .ok_or_else(|| fail("4EB500 current GTID retired"))?;
        if m.reference != id.raw() {
            return Err(fail("4EB500 current spatial generation differs"));
        }
        Ok(m)
    }
    fn parent(&self) -> Result<Option<ObjectKey>> {
        let object = self.staged.actors.object(self.key).map_err(fail)?;
        if matches!(object.definition.native_class, 2 | 6 | 7 | 9) {
            return Ok(None);
        }
        let actor = object
            .actor
            .as_ref()
            .ok_or_else(|| fail("4EA260 current Actor absent"))?;
        if let Some(component) = actor.components.get(ComponentSlot::Structure) {
            let RuntimeComponent::Structure(structure) = component else {
                return Err(fail("4CED60 Structure payload differs"));
            };
            if structure.header.owner
                != self
                    .staged
                    .actors
                    .id(self.key)
                    .ok_or_else(|| fail("4CED60 current generation absent"))?
            {
                return Err(fail("4CED60 Structure owner differs"));
            }
            if structure.object_handle_10 != 0 {
                return Err(fail(
                    "4CED60 actual positive Structure parent owner required",
                ));
            }
        }
        if actor.components.present(ComponentSlot::Wall) {
            return Err(fail("4CED60 actual Wall58CB30 reference required"));
        }
        Ok(None)
    }
}
impl ObjectEffects for Current<'_, '_> {
    fn precision(&self) -> Precision {
        self.pc
    }
    fn flags(&self) -> Result<u32> {
        Ok(self.member()?.actor.flags)
    }
    fn blocking_radius(&self) -> Result<u32> {
        Ok(self.member()?.blocking.radius_bits)
    }
    fn cell_size(&self) -> Result<u32> {
        Ok(self
            .world()?
            .live_spatial()
            .map_err(fail)?
            .index()
            .grid()
            .cell_size_bits)
    }
    fn position(&self) -> Result<[u32; 2]> {
        Ok(self.member()?.actor.position)
    }
    fn blocking_shape(&self) -> Result<aa_spatial::collision::Shape> {
        Ok(self.member()?.blocking.clone())
    }
    fn facing(&self) -> Result<[u32; 2]> {
        Ok(self.member()?.actor.facing)
    }
    fn actor_category(&self) -> Result<bool> {
        Ok(self.member()?.actor_category)
    }
    fn wall_present(&self) -> Result<bool> {
        Ok(self.member()?.wall_present)
    }
    fn session_mode(&self) -> Result<i32> {
        self.catalog
            .load_session
            .ok_or_else(|| fail("555C10 actual SessionE4 owner absent"))?
            .application_state()
            .map(|v| v as i32)
            .map_err(fail)
    }
    fn world_word_48(&self) -> Result<u32> {
        Ok(self.world()?.mission_fields.started48())
    }
    fn height_margin_bits(&self) -> Result<u32> {
        let vars = &self.world()?.variables;
        let r = vars
            .registrations()
            .nth(6)
            .ok_or_else(|| fail("4EB500 SVar index6 absent"))?;
        if r.ordinal() != 6 || r.name() != "ElevationSlopeMax" || r.kind() != 2 {
            return Err(fail(
                "4EB500 current ElevationSlopeMax registration differs",
            ));
        }
        match vars.current(6).map_err(fail)? {
            crate::load_view_variables::CurrentValue::Fixed(bits) => Ok(*bits),
            _ => Err(fail("4EB500 ElevationSlopeMax current kind differs")),
        }
    }
    fn ground_height(&mut self, position: [u32; 2], flags: u32) -> Result<Binary> {
        let world = self.world()?;
        aa_spatial::position::actor_post_create_height(
            self.params.mode,
            || Ok(self.params.word_4c),
            |reference| {
                let Some(object) = self.staged.actors.key(ActorId::from_raw(reference)) else {
                    return Ok(None);
                };
                let placement = self
                    .staged
                    .actors
                    .placement(object, world.live_spatial().map_err(fail)?)
                    .map_err(fail)?;
                Binary::from_f32_bits(placement.position_bits[2])
                    .map(Some)
                    .map_err(Error::from)
            },
            || world.terrain.surface()?.sample(position, flags, self.pc),
        )
    }
    fn edit_height(&mut self, edit: HeightEdit) -> Result<()> {
        let world = self
            .staged
            .load_world
            .as_mut()
            .ok_or_else(|| fail("551140 actual World absent"))?;
        let mut effects = Brush {
            catalog: self.catalog,
            bank: &self.staged.definitions,
            actors: &self.staged.actors,
            key: self.key,
            params: self.params,
            scratch: &mut self.scratch,
            point: world.point.as_ref(),
            phase: world.dispatcher.as_ref().map(|d| &d.phase),
            presentation: &world.presentation,
            messages: &mut self.staged.report.native_messages,
        };
        //Native4EB500 ignores AL: both true and false normal returns continue.
        crate::height_brush::apply(&mut world.terrain, &edit, &mut effects, self.pc)
            .map(|_| ())
            .map_err(fail)
    }
    fn clear_doodads(&mut self, placement: Placement) -> Result<()> {
        aa_spatial::doodad_clear::clear(
            &mut Doodads {
                staged: self.staged,
                catalog: self.catalog,
                pc: self.pc,
            },
            &placement,
        )
    }
    fn ground_variation(&mut self) -> Result<Option<u32>> {
        let world = self.world()?;
        let ordinal = world
            .view_biome
            .as_ref()
            .ok_or_else(|| fail("4EA260 actual GMap+24 Biome absent"))?
            .ordinal(self.catalog, &self.staged.definitions)
            .map_err(fail)?;
        if let Some(parent) = self.parent()? {
            let definition = &self.staged.actors.object(parent).map_err(fail)?.definition;
            let source = crate::object_ground_source::Source::bind(
                self.catalog,
                &self.staged.definitions,
                definition,
            )
            .map_err(fail)?;
            if let Some(index) = source.variation(ordinal).map_err(fail)? {
                return u32::try_from(index).map(Some).map_err(fail);
            }
        }
        let definition = &self
            .staged
            .actors
            .object(self.key)
            .map_err(fail)?
            .definition;
        let source = crate::object_ground_source::Source::bind(
            self.catalog,
            &self.staged.definitions,
            definition,
        )
        .map_err(fail)?;
        source
            .variation(ordinal)
            .map_err(fail)?
            .map(|i| u32::try_from(i).map_err(fail))
            .transpose()
    }
    fn footprint_padding_bits(&self) -> Result<u32> {
        //4EB67B always reads CURRENT ObjectData padding, even if a parent
        //Variation won4EA260. The wall guard precedes this source getter.
        let definition = &self
            .staged
            .actors
            .object(self.key)
            .map_err(fail)?
            .definition;
        crate::object_ground_source::Source::bind(
            self.catalog,
            &self.staged.definitions,
            definition,
        )
        .map(|s| s.padding())
        .map_err(fail)
    }
    fn paint_terrain(&mut self, _: TerrainPaint) -> Result<()> {
        Err(fail(
            "555C10 positive Variation requires same Terrain stamp/commit binding",
        ))
    }
}
struct Brush<'a, 's> {
    catalog: &'a Catalog<'s>,
    bank: &'a Bank,
    actors: &'a ActorStore,
    key: ObjectKey,
    params: &'a Params,
    scratch: &'a mut Option<CreationGroundScratch>,
    point: &'a crate::load_world_globals::PointShape,
    phase: Option<&'a u32>,
    presentation: &'a crate::fx_manager::State<crate::fx_graph::Command>,
    messages: &'a mut Vec<aa_mapworld::restore::NativeMessage>,
}
impl crate::height_brush::Effects for Brush<'_, '_> {
    fn orientation(&mut self) -> std::result::Result<[u32; 3], String> {
        self.catalog.current_renderer_orientation(self.bank)
    }
    fn point_shape(&mut self) -> std::result::Result<aa_spatial::collision::Shape, String> {
        Ok(self.point.shape().clone())
    }
    fn falloff_distance(&mut self) -> std::result::Result<u32, String> {
        self.scratch
            .take()
            .ok_or("5512BF actual final ExistingFX caller scratch is unbound")?
            .consume(self.actors, self.key, self.params)
            .map_err(|e| e.to_string())
    }
    fn protected_diagnostic(
        &mut self,
        cell: [i32; 2],
        area: u8,
        requested: u8,
        current: u32,
        target: u32,
        result: u32,
    ) -> std::result::Result<(), String> {
        self.messages.try_reserve(1).map_err(|e| e.to_string())?;
        self.messages.push(aa_mapworld::restore::NativeMessage{source_ordinal:usize::MAX,function:0x404ed0,line:0,
            text:format!("551140 protected cell {cell:?}, area{area}/{requested}, height{current:08x}/{target:08x}/{result:08x}")});
        Ok(())
    }
    fn notify(
        &mut self,
        _: [u32; 2],
        _: u32,
        map: &crate::battle_terrain::MapNotification,
    ) -> std::result::Result<(), String> {
        if *self.phase.ok_or("50C310 actual WorldD8 owner absent")? == 6 {
            return Ok(());
        }
        if map.dirty() {
            return Err("50C310 dirty map helper requires actual55A550 clear".into());
        }
        if self.presentation.attached_view().is_some() {
            return Err("50C310 actual attached-view43B240 terrain notification required".into());
        }
        Ok(())
    }
}
struct Doodads<'a, 's> {
    staged: &'a mut Staged,
    catalog: &'a Catalog<'s>,
    pc: Precision,
}
impl aa_spatial::doodad_clear::World for Doodads<'_, '_> {
    type Object = ObjectKey;
    fn collect_overlaps(&mut self, placement: &Placement) -> Result<Vec<u32>> {
        let world = self
            .staged
            .load_world
            .as_mut()
            .ok_or_else(|| fail("4F5560 actual World absent"))?;
        let crate::load_world::SpatialOwner::Adopted(adopted) = &mut world.spatial else {
            return Err(fail("4F5560 actual adopted index absent"));
        };
        if adopted.error.is_some()
            || !adopted.pending.is_empty()
            || adopted.world.index().precision() != self.pc
        {
            return Err(fail("4F5560 adoption/precision differs"));
        }
        let (_, geometry) = crate::load_object_init_world::map_geometry_index(
            &world.terrain,
            &world.header,
            adopted.world.index(),
            &world.zones,
            self.pc,
        )
        .map_err(fail)?;
        let (index, bindings) = adopted.world.split_index();
        crate::map_actor_placement::collect_placed_overlaps(
            self.catalog,
            &self.staged.actors,
            &self.staged.definitions,
            &world.source_fields,
            index,
            bindings,
            &geometry,
            placement.clone(),
        )
    }
    fn resolve(&mut self, id: u32) -> Result<ObjectKey> {
        self.staged
            .actors
            .key(ActorId::from_raw(id))
            .ok_or_else(|| fail("4F5560 collected GTID is stale"))
    }
    fn runtime_class(&mut self, key: ObjectKey) -> Result<u32> {
        Ok(self
            .staged
            .actors
            .object(key)
            .map_err(fail)?
            .definition
            .native_class)
    }
    fn flags(&mut self, key: ObjectKey) -> Result<u32> {
        let world = self
            .staged
            .load_world
            .as_ref()
            .ok_or_else(|| fail("4F5560 actual World absent"))?;
        self.staged
            .actors
            .placement(key, world.live_spatial().map_err(fail)?)
            .map(|p| p.flags)
            .map_err(fail)
    }
    fn referenced_by_trigger(&mut self, key: ObjectKey) -> Result<bool> {
        let world = self
            .staged
            .load_world
            .as_ref()
            .ok_or_else(|| fail("54C200 actual World absent"))?;
        let manager = world
            .trigger_manager
            .as_ref()
            .ok_or_else(|| fail("54C200 actual process TriggerManager absent"))?;
        Ok(aa_triggers::thing_references::first_referencing(
            manager,
            &mut Thing {
                actors: &self.staged.actors,
                key,
            },
        )
        .map_err(fail)?
        .is_some())
    }
    fn definition_ids(&mut self, key: ObjectKey) -> Result<String> {
        Ok(self
            .staged
            .actors
            .object(key)
            .map_err(fail)?
            .definition
            .proof
            .id
            .clone())
    }
    fn current_gtid(&mut self, key: ObjectKey) -> Result<u32> {
        self.staged
            .actors
            .id(key)
            .map(ActorId::raw)
            .ok_or_else(|| fail("4F5560 current GTID retired"))
    }
    fn log_purge(&mut self, id: u32, ids: String) -> Result<()> {
        self.staged
            .report
            .native_messages
            .try_reserve(1)
            .map_err(fail)?;
        self.staged
            .report
            .native_messages
            .push(aa_mapworld::restore::NativeMessage {
                source_ordinal: usize::MAX,
                function: 0x404f90,
                line: 0,
                text: format!("Removing doodad {ids} ({id}) after ground placement"),
            });
        Ok(())
    }
    fn destroy(&mut self, key: ObjectKey) -> Result<()> {
        crate::map_replacement_drain::destroy_doodad(self.staged, self.catalog, key, self.pc)
            .map_err(fail)
    }
}
struct Thing<'a> {
    actors: &'a ActorStore,
    key: ObjectKey,
}
impl aa_triggers::thing_references::Thing for Thing<'_> {
    fn current_gtid(&mut self) -> aa_triggers::manager::Result<u32> {
        self.actors
            .id(self.key)
            .map(ActorId::raw)
            .ok_or_else(|| aa_triggers::manager::Error::State("543A90 current GTID retired".into()))
    }
    fn name(&mut self) -> aa_triggers::manager::Result<Vec<u16>> {
        Ok(self
            .actors
            .object(self.key)
            .map_err(|e| aa_triggers::manager::Error::State(e.to_string()))?
            .thing
            .name_utf16
            .clone())
    }
}
