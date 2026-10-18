//!4EEB40/4EE5C0/4D0EE0 over the same stored Actor and spatial owners.
//!Numerics, membership and orchestration remain shared aa-spatial kernels.
//!No index/component loan spans a child; reached missing services retain prefix.
use crate::{
    actor_world::{ActorAccess, RuntimeComponent, RuntimeObject},
    battle_terrain::BattleTerrain,
    catalog::Catalog,
    definitions::Bank,
    spatial_world::SpatialWorld,
};
use aa_actors::{ActorId, components::ComponentSlot as Slot, ground_states::OrganizationState};
use aa_numeric::{Binary, Precision};
use aa_spatial::{
    Error, Result,
    index::{ActorKey, Member},
    position::{
        self as native, Components, Diagnostic, HeightEdit, ObjectEffects, PositionHistory,
        TerrainPaint,
    },
};
fn fail(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}
///All references point into the current World. Native phase is World+D8,
///distinct from MapResource+A8 and Session+F8/E4.
pub struct Context<'a, 'store> {
    pub actors: &'a mut ActorAccess<'store>,
    pub spatial: &'a mut SpatialWorld,
    pub bank: &'a mut Bank,
    pub terrain: &'a BattleTerrain,
    pub variables: &'a mut crate::load_view_variables::State,
    pub mission: &'a crate::mission_fields::State,
    pub phase: Option<&'a u32>,
}
///Native positive children. Defaults refuse the exact reached boundary; no
///callback pretends to have changed terrain, components, queues or renderer.
pub trait Effects {
    fn diagnostic(&mut self, _actor: ActorId, _value: Diagnostic) -> Result<()> {
        Err(fail("4D0EE0 actual SyncA77DA70/77D930 scope required"))
    }
    fn cell_changed(
        &mut self,
        _context: &mut Context<'_, '_>,
        _actor: ActorId,
        _old: [i32; 2],
    ) -> Result<()> {
        Err(fail("4D6350 actual cell/terrain/zone transition required"))
    }
    fn edit_height(
        &mut self,
        _context: &mut Context<'_, '_>,
        _actor: ActorId,
        _edit: HeightEdit,
    ) -> Result<()> {
        Err(fail(
            "4EB500 NULL-Params caller height-brush/scratch service required",
        ))
    }
    fn clear_doodads(
        &mut self,
        _context: &mut Context<'_, '_>,
        _actor: ActorId,
        _placement: aa_spatial::geometry::Placement,
    ) -> Result<()> {
        Err(fail(
            "4F5560 actual same-owner doodad destruction service required",
        ))
    }
    fn ground_variation(
        &mut self,
        _context: &mut Context<'_, '_>,
        _actor: ActorId,
    ) -> Result<Option<u32>> {
        Err(fail(
            "4EA260 actual current Biome/base-terrain source service required",
        ))
    }
    fn paint_terrain(
        &mut self,
        _context: &mut Context<'_, '_>,
        _actor: ActorId,
        _paint: TerrainPaint,
    ) -> Result<()> {
        Err(fail(
            "555C10 actual same-owner variation paint service required",
        ))
    }
    fn mind_predicate(
        &mut self,
        _context: &mut Context<'_, '_>,
        _actor: ActorId,
        _slot: u32,
    ) -> Result<bool> {
        Err(fail("57C680 actual current Mind64/7C predicate required"))
    }
    fn interpolate(&mut self, _context: &mut Context<'_, '_>, _actor: ActorId) -> Result<bool> {
        Err(fail(
            "57C080 actual current Organization interpolation required",
        ))
    }
    fn has_scheduled(
        &mut self,
        _context: &mut Context<'_, '_>,
        _actor: ActorId,
        _event: u32,
        _data: u32,
    ) -> Result<bool> {
        Err(fail("50B220 actual Organization scheduler owner required"))
    }
    fn schedule(
        &mut self,
        _context: &mut Context<'_, '_>,
        _actor: ActorId,
        _data: u32,
    ) -> Result<()> {
        Err(fail(
            "50AC00 actual Organization scheduler publication required",
        ))
    }
    fn follower_target(
        &mut self,
        _context: &mut Context<'_, '_>,
        _actor: ActorId,
        _position: [u32; 3],
    ) -> Result<()> {
        Err(fail("577240 actual follower Mover target service required"))
    }
    fn presentation_position_changed(
        &mut self,
        _context: &mut Context<'_, '_>,
        _actor: ActorId,
    ) -> Result<()> {
        Err(fail(
            "50C250/43B2E0 actual current view position notification required",
        ))
    }
    fn presentation_available(&self) -> Result<bool>;
}
struct Bound<'a, 'context, 'store, 'source, E> {
    catalog: &'a Catalog<'source>,
    context: &'a mut Context<'context, 'store>,
    effects: &'a mut E,
    pc: Precision,
    path: Vec<ActorId>,
}
impl<E: Effects> Bound<'_, '_, '_, '_, E> {
    fn id(&self, key: ActorKey) -> Result<ActorId> {
        let m = self.context.spatial.index().member(key)?;
        let id = ActorId::from_raw(m.reference);
        let object = self.context.spatial.object(key).map_err(fail)?;
        if self.context.actors.key(id) != Some(object)
            || self.context.spatial.index().resolve_reference(id.raw()) != Some(key)
        {
            return Err(fail("4D0EE0 actual actor/index generation differs"));
        }
        Ok(id)
    }
    fn key(&self, id: ActorId) -> Result<ActorKey> {
        let object = self
            .context
            .actors
            .key(id)
            .ok_or_else(|| fail("4D0EE0 current Actor generation absent"))?;
        self.context.spatial.key(object).map_err(fail)
    }
    fn object(&self, id: ActorId) -> Result<&RuntimeObject> {
        let key = self
            .context
            .actors
            .key(id)
            .ok_or_else(|| fail("4D0EE0 current Actor absent"))?;
        let object = self.context.actors.object(key).map_err(fail)?;
        if object.definition.native_class != 3 || object.actor.is_none() || object.object.is_none()
        {
            return Err(fail("4D0EE0 current ordinary Actor dispatch required"));
        }
        crate::live_object_init::Source::bind_in_bank(
            self.catalog,
            self.context.bank,
            &object.definition,
        )
        .map_err(fail)?;
        Ok(object)
    }
    fn member(&self, key: ActorKey) -> Result<&Member> {
        self.id(key)?;
        self.context.spatial.index().member(key)
    }
    fn word(&self, id: ActorId, offset: u32) -> Result<u32> {
        self.object(id)?
            .actor
            .as_ref()
            .unwrap()
            .words
            .get(&offset)
            .copied()
            .ok_or_else(|| fail(format!("4D0EE0 Actor+{offset:X} unbound")))
    }
    fn org(&self, id: ActorId) -> Result<&OrganizationState<String>> {
        self.object(id)?;
        match self
            .context
            .actors
            .component(id, Slot::Organization)
            .map_err(fail)?
        {
            Some(RuntimeComponent::Organization(v)) if v.header.owner == id => Ok(v),
            _ => Err(fail("4D0EE0 actual Organization allocation/owner differs")),
        }
    }
    fn org_mut(&mut self, id: ActorId) -> Result<&mut OrganizationState<String>> {
        self.org(id)?;
        match self
            .context
            .actors
            .component_mut(id, Slot::Organization)
            .map_err(fail)?
        {
            Some(RuntimeComponent::Organization(v)) => Ok(v),
            _ => unreachable!(),
        }
    }
    fn relocate(&mut self, id: ActorId, point: [u32; 2]) -> Result<()> {
        if self.path.contains(&id) {
            return Err(fail("4EEB40 invalid cyclic position callback ownership"));
        }
        self.object(id)?;
        let object = self.context.actors.key(id).ok_or_else(|| fail("4EEB40 current generation absent"))?;
        if self.context.actors.staged_placement_mut(object).is_ok() {
            return Err(fail("4EEB40 requires the actual published placement owner"));
        }
        let key = self.key(id)?;
        let member = self.member(key)?;
        let definition = self.context.bank.require_usable(member.actor.definition).map_err(fail)?;
        let current = self.object(id)?;
        if definition.key.domain != "actordefinition" || !definition.key.name.eq_ignore_ascii_case(&current.definition.proof.id) {
            return Err(fail("4EEB40 current spatial/source Bank definition differs"));
        }
        let geometry = self.context.spatial.source_geometry(member.actor.definition).map_err(fail)?;
        if geometry.source.id != current.definition.proof.id || member.blocking.kind != geometry.source.blocking.words[0]
            || member.blocking.radius_bits != geometry.source.blocking.words[1] || member.blocking.bounds != geometry.source.blocking.words[2..6] {
            return Err(fail("4EEB40 current spatial/source shape differs"));
        }
        self.path.push(id);
        let result = native::relocate_on_owners(self, key, point);
        self.path.pop();
        result
    }
}
struct Ground<'a, 'b, 'c, 'store, 'source, E> {
    bound: &'a mut Bound<'b, 'c, 'store, 'source, E>,
    actor: ActorKey,
}
impl<E: Effects> ObjectEffects for Ground<'_, '_, '_, '_, '_, E> {
    fn precision(&self) -> Precision {
        self.bound.pc
    }
    fn flags(&self) -> Result<u32> {
        Ok(self.bound.member(self.actor)?.actor.flags)
    }
    fn blocking_radius(&self) -> Result<u32> {
        Ok(self.bound.member(self.actor)?.blocking.radius_bits)
    }
    fn cell_size(&self) -> Result<u32> {
        Ok(self.bound.context.spatial.index().grid().cell_size_bits)
    }
    fn position(&self) -> Result<[u32; 2]> {
        Ok(self.bound.member(self.actor)?.actor.position)
    }
    fn blocking_shape(&self) -> Result<aa_spatial::collision::Shape> {
        Ok(self.bound.member(self.actor)?.blocking.clone())
    }
    fn facing(&self) -> Result<[u32; 2]> {
        Ok(self.bound.member(self.actor)?.actor.facing)
    }
    fn actor_category(&self) -> Result<bool> {
        Ok(self.bound.member(self.actor)?.actor_category)
    }
    fn wall_present(&self) -> Result<bool> {
        Ok(self.bound.member(self.actor)?.wall_present)
    }
    fn session_mode(&self) -> Result<i32> {
        self.bound
            .catalog
            .load_session
            .ok_or_else(|| fail("4D0EE0 actual SessionE4 loan required"))?
            .application_state()
            .map(|v| v as i32)
            .map_err(fail)
    }
    fn world_word_48(&self) -> Result<u32> {
        Ok(self.bound.context.mission.started48())
    }
    fn height_margin_bits(&self) -> Result<u32> {
        let vars = &*self.bound.context.variables;
        vars.validate_in_bank(self.bound.catalog, self.bound.context.bank)
            .map_err(fail)?;
        let r = vars
            .registrations()
            .nth(6)
            .ok_or_else(|| fail("4EB500 current SVar6 absent"))?;
        if r.name() != "ElevationSlopeMax" || r.kind() != 2 {
            return Err(fail("4EB500 SVar6 registration differs"));
        }
        match vars.current(6).map_err(fail)? {
            crate::load_view_variables::CurrentValue::Fixed(v) => Ok(*v),
            _ => Err(fail("4EB500 current SVar6 is not Fixed")),
        }
    }
    fn ground_height(&mut self, xy: [u32; 2], flags: u32) -> Result<Binary> {
        self.bound
            .context
            .terrain
            .surface()?
            .sample(xy, flags, self.bound.pc)
    }
    fn edit_height(&mut self, edit: HeightEdit) -> Result<()> {
        let id = self.bound.id(self.actor)?;
        self.bound.effects.edit_height(self.bound.context, id, edit)
    }
    fn clear_doodads(&mut self, placement: aa_spatial::geometry::Placement) -> Result<()> {
        let id = self.bound.id(self.actor)?;
        self.bound
            .effects
            .clear_doodads(self.bound.context, id, placement)
    }
    fn ground_variation(&mut self) -> Result<Option<u32>> {
        let id = self.bound.id(self.actor)?;
        self.bound.effects.ground_variation(self.bound.context, id)
    }
    fn footprint_padding_bits(&self) -> Result<u32> {
        let id = self.bound.id(self.actor)?;
        let object = self.bound.object(id)?;
        crate::object_ground_source::Source::bind(
            self.bound.catalog,
            self.bound.context.bank,
            &object.definition,
        )
        .map(|v| v.padding())
        .map_err(fail)
    }
    fn paint_terrain(&mut self, paint: TerrainPaint) -> Result<()> {
        let id = self.bound.id(self.actor)?;
        self.bound
            .effects
            .paint_terrain(self.bound.context, id, paint)
    }
}
impl<E: Effects> native::ObjectPositionWorld for Bound<'_, '_, '_, '_, E> {
    fn set_membership_request(&mut self, key: ActorKey, value: bool) -> Result<()> {
        self.id(key)?;
        let (index, _) = self.context.spatial.split_index();
        index.set_ordinary_membership_request(key, value)
    }
    fn object_effects(&mut self, key: ActorKey) -> Result<()> {
        native::object_effects_on_owners(&mut Ground {
            bound: self,
            actor: key,
        })
    }
    fn cached_tile(&self, key: ActorKey) -> Result<[i32; 2]> {
        Ok(self.member(key)?.cached_tile)
    }
    fn position(&self, key: ActorKey) -> Result<[u32; 2]> {
        Ok(self.member(key)?.actor.position)
    }
    fn coordinate_shift(&self) -> Result<u32> {
        Ok(self.context.spatial.index().grid().coordinate_shift)
    }
    fn store_cached_tile(&mut self, key: ActorKey, tile: [i32; 2]) -> Result<()> {
        self.id(key)?;
        let (index, _) = self.context.spatial.split_index();
        index.store_position_cached_tile(key, tile)
    }
    fn cell_changed(&mut self, key: ActorKey, old: [i32; 2]) -> Result<()> {
        let id = self.id(key)?;
        self.effects.cell_changed(self.context, id, old)
    }
}
impl<E: Effects> native::ActorPositionWorld for Bound<'_, '_, '_, '_, E> {
    fn reference(&self, key: ActorKey) -> Result<u32> {
        Ok(self.id(key)?.raw())
    }
    fn flags(&self, key: ActorKey) -> Result<u32> {
        Ok(self.member(key)?.actor.flags)
    }
    fn components(&self, key: ActorKey) -> Result<Components> {
        let id = self.id(key)?;
        let object = self.object(id)?;
        let c = object.components().map_err(fail)?;
        let slot = |slot| -> Result<Option<u32>> {
            if let Some(v) = c.get(slot) {
                if v.slot() != slot {
                    return Err(fail("4D0EE0 current component slot differs"));
                }
                Ok(Some(id.raw()))
            } else {
                Ok(None)
            }
        };
        Ok(Components {
            organization: slot(Slot::Organization)?,
            element: slot(Slot::Element)?,
            mover: slot(Slot::Mover)?,
            settlement: slot(Slot::Settlement)?,
            history: (self.word(id, 0x144)? != 0).then_some(id.raw()),
            follower_reference: self.word(id, 0x168)?,
        })
    }
    fn session_mode(&self) -> Result<i32> {
        self.catalog
            .load_session
            .ok_or_else(|| fail("5782D0 actual SessionE4 loan required"))?
            .application_state()
            .map(|v| v as i32)
            .map_err(fail)
    }
    fn diagnostic(&mut self, key: ActorKey, value: Diagnostic) -> Result<()> {
        let id = self.id(key)?;
        self.effects.diagnostic(id, value)
    }
    fn component_owner(&self, component: u32) -> Result<ActorKey> {
        let id = ActorId::from_raw(component);
        self.object(id)?;
        self.key(id)
    }
    fn set_organization_saved_xy(&mut self, component: u32, xy: [u32; 2]) -> Result<()> {
        self.org_mut(ActorId::from_raw(component))?.position_xy = xy.map(aa_body::F32);
        Ok(())
    }
    fn element_position_changed(&mut self, component: u32) -> Result<()> {
        let id = ActorId::from_raw(component);
        let parent = match self
            .context
            .actors
            .component(id, Slot::Element)
            .map_err(fail)?
        {
            Some(RuntimeComponent::Element(v)) if v.header.owner == id => v.organization,
            _ => return Err(fail("5705E0 actual Element allocation/owner differs")),
        };
        if let Some(parent) = parent {
            aa_actors::element_position::organization_moved(parent, id, self).map_err(fail)?;
        }
        Ok(())
    }
    fn is_attached(&mut self, key: ActorKey) -> Result<bool> {
        aa_actors::attachment::is_attached(self.id(key)?, self).map_err(fail)
    }
    fn attachment_parent(&mut self, key: ActorKey) -> Result<ActorKey> {
        let id = self.id(key)?;
        let parent = aa_actors::attachment::parent(id, self)
            .map_err(fail)?
            .ok_or_else(|| fail("4CE9A0 attached Actor parent became null"))?;
        self.key(parent)
    }
    fn settlement_attached_actor(&self, _component: u32) -> Result<Option<ActorKey>> {
        Err(fail("588990 actual typed Settlement+14 reference required"))
    }
    fn move_ground(&mut self, key: ActorKey, xy: [u32; 2]) -> Result<()> {
        let id = self.id(key)?;
        self.relocate(id, xy)
    }
    fn resolve_reference(&self, reference: u32) -> Result<Option<ActorKey>> {
        Ok(self.context.spatial.index().resolve_reference(reference))
    }
    fn follower_offset(&self, _owner: ActorKey) -> Result<[u32; 2]> {
        Err(fail(
            "5771B0 actual canonical follower270/274 source required",
        ))
    }
    fn follower_point(&self, owner: ActorKey, offset: [u32; 2]) -> Result<[u32; 3]> {
        native::follower_point(self.context.spatial.index(), owner, offset)
    }
    fn follower_target(&mut self, key: ActorKey, position: [u32; 3]) -> Result<()> {
        let id = self.id(key)?;
        self.effects.follower_target(self.context, id, position)
    }
    fn history_mut(&mut self, _component: u32) -> Result<&mut PositionHistory> {
        Err(fail(
            "50A710/50A970 actual typed Actor144 history owner required",
        ))
    }
    fn history_interval(&self) -> Result<i32> {
        Err(fail(
            "50A970 actual current HistorySampleInterval SVar required",
        ))
    }
    fn frame(&self) -> Result<i32> {
        Err(fail("50A970 actual current World frame owner required"))
    }
    fn world_time_bits(&self) -> Result<u32> {
        Err(fail("50A970 actual current World time owner required"))
    }
}
impl<E: Effects> native::GroundRelocationWorld for Bound<'_, '_, '_, '_, E> {
    fn move_ground_fields(&mut self, key: ActorKey, point: [u32; 2]) -> Result<()> {
        self.id(key)?;
        let surface = self.context.terrain.surface()?;
        let (index, _) = self.context.spatial.split_index();
        index.move_ground_fields(key, point, &surface)
    }
    fn world_phase(&self) -> Result<u32> {
        self.context
            .phase
            .copied()
            .ok_or_else(|| fail("50C250 actual WorldD8 owner absent"))
    }
    fn presentation_available(&self) -> Result<bool> {
        self.effects.presentation_available()
    }
    fn presentation_position_changed(&mut self, key: ActorKey) -> Result<()> {
        let id = self.id(key)?;
        self.effects.presentation_position_changed(self.context, id)
    }
}
impl<E: Effects> aa_actors::attachment::World for Bound<'_, '_, '_, '_, E> {
    type Actor = ActorId;
    fn structure(
        &self,
        id: ActorId,
    ) -> aa_actors::attachment::Result<Option<aa_actors::attachment::Structure<ActorId>>> {
        self.object(id)
            .map_err(|e| aa_actors::attachment::Error::World(e.to_string()))?;
        match self
            .context
            .actors
            .component(id, Slot::Structure)
            .map_err(|e| aa_actors::attachment::Error::World(e.to_string()))?
        {
            None => Ok(None),
            Some(RuntimeComponent::Structure(v))
                if v.header.owner == id && v.object_handle_10 == 0 =>
            {
                Ok(Some(aa_actors::attachment::Structure {
                    owner: id,
                    parent: None,
                }))
            }
            _ => Err(aa_actors::attachment::Error::Missing(
                "current positive Structure10 parent binding",
            )),
        }
    }
    fn settlement(
        &self,
        id: ActorId,
    ) -> aa_actors::attachment::Result<Option<aa_actors::attachment::Settlement<ActorId>>> {
        self.object(id)
            .map_err(|e| aa_actors::attachment::Error::World(e.to_string()))?;
        if self
            .context
            .actors
            .component(id, Slot::Settlement)
            .map_err(|e| aa_actors::attachment::Error::World(e.to_string()))?
            .is_none()
        {
            Ok(None)
        } else {
            Err(aa_actors::attachment::Error::Missing(
                "current typed Settlement14 reference",
            ))
        }
    }
    fn is_attached_virtual(&mut self, id: ActorId) -> aa_actors::attachment::Result<bool> {
        aa_actors::attachment::is_attached(id, self)
    }
}
impl<E: Effects> aa_actors::element_position::World for Bound<'_, '_, '_, '_, E> {
    type Definition = String;
    fn world_mode(&self) -> aa_actors::element_position::Result<u32> {
        self.context.phase.copied().ok_or_else(|| {
            aa_actors::element_position::Error::World("57C800 actual WorldD8 owner absent".into())
        })
    }
    fn actor(
        &self,
        id: ActorId,
    ) -> aa_actors::element_position::Result<aa_actors::element_position::Actor> {
        let key = self
            .key(id)
            .map_err(|e| aa_actors::element_position::Error::World(e.to_string()))?;
        let m = self
            .member(key)
            .map_err(|e| aa_actors::element_position::Error::World(e.to_string()))?;
        Ok(aa_actors::element_position::Actor {
            flags: m.actor.flags,
            position: m.actor.position.map(aa_body::F32),
            mind: self
                .context
                .actors
                .component(id, Slot::Mind)
                .map_err(|e| aa_actors::element_position::Error::World(e.to_string()))?
                .is_some()
                .then_some(id.raw()),
        })
    }
    fn organization(
        &self,
        id: ActorId,
    ) -> aa_actors::element_position::Result<&OrganizationState<String>> {
        self.org(id)
            .map_err(|e| aa_actors::element_position::Error::World(e.to_string()))
    }
    fn organization_mut(
        &mut self,
        id: ActorId,
    ) -> aa_actors::element_position::Result<&mut OrganizationState<String>> {
        self.org_mut(id)
            .map_err(|e| aa_actors::element_position::Error::World(e.to_string()))
    }
    fn selected_member(&self, id: ActorId) -> aa_actors::element_position::Result<Option<ActorId>> {
        let v = *self.organization(id)?.words.get(&0x40).ok_or_else(|| {
            aa_actors::element_position::Error::World(
                "57C800 actual Organization40 unwritten".into(),
            )
        })?;
        Ok((v != 0).then(|| ActorId::from_raw(v)))
    }
    fn mind_predicate(
        &mut self,
        mind: u32,
        slot: u32,
    ) -> aa_actors::element_position::Result<bool> {
        self.effects
            .mind_predicate(self.context, ActorId::from_raw(mind), slot)
            .map_err(|e| aa_actors::element_position::Error::World(e.to_string()))
    }
    fn relocate_owner(
        &mut self,
        id: ActorId,
        xy: [aa_body::F32; 2],
    ) -> aa_actors::element_position::Result<()> {
        self.relocate(id, xy.map(|v| v.0))
            .map_err(|e| aa_actors::element_position::Error::World(e.to_string()))
    }
    fn interpolate(&mut self, id: ActorId) -> aa_actors::element_position::Result<bool> {
        self.effects
            .interpolate(self.context, id)
            .map_err(|e| aa_actors::element_position::Error::World(e.to_string()))
    }
    fn has_scheduled(
        &mut self,
        id: ActorId,
        event: u32,
        data: u32,
    ) -> aa_actors::element_position::Result<bool> {
        self.effects
            .has_scheduled(self.context, id, event, data)
            .map_err(|e| aa_actors::element_position::Error::World(e.to_string()))
    }
    fn schedule(&mut self, id: ActorId, data: u32) -> aa_actors::element_position::Result<()> {
        self.effects
            .schedule(self.context, id, data)
            .map_err(|e| aa_actors::element_position::Error::World(e.to_string()))
    }
}
///Same canonical ActorAccess, moved source Bank, real heightfield and published
///index. An unsupported child returns after its already-committed native prefix.
pub fn relocate_on_owners(
    catalog: &Catalog<'_>,
    context: &mut Context<'_, '_>,
    actor: ActorId,
    point: [u32; 2],
    effects: &mut impl Effects,
    pc: Precision,
) -> std::result::Result<(), String> {
    catalog.require_source_owner()?;
    let source = context.bank.combat_source()?;
    source.completed_means()?;
    if source.source_identities() != catalog.actors.sources.as_slice()
        || context.terrain.source_precision() != source.source_precision()
        || context.spatial.index().precision() != pc
    {
        return Err("4EEB40 actual source/height/index owner or precision differs".into());
    }
    let mut bound = Bound {
        catalog,
        context,
        effects,
        pc,
        path: Vec::new(),
    };
    bound.relocate(actor, point).map_err(|e| e.to_string())
}

///Native SyncA logs. Its real enabled/suspend checks precede the current OS/SAI
///gate, independently for all three reached records.
pub fn log_position(
    sync: &mut crate::live_clocked::SyncBinding<'_>,
    value: Diagnostic,
) -> Result<()> {
    fn emit(
        owner: &mut aa_orders::net::sync::Synchronizer,
        gate: &mut dyn FnMut() -> std::result::Result<bool, String>,
        value: Diagnostic,
    ) -> Result<()> {
        let open = if owner.enabled && owner.suspend < 1 {
            gate().map_err(fail)?
        } else {
            false
        };
        match value {
            Diagnostic::Identity(v) => owner.log_int(open, "(int)GetID()", v),
            Diagnostic::X(v) => owner.log_float_bits(open, "GetPosition().x", v),
            Diagnostic::Y(v) => owner.log_float_bits(open, "GetPosition().y", v),
        };
        Ok(())
    }
    match sync {
        crate::live_clocked::SyncBinding::Direct(log) => emit(log.owner, log.gate, value),
        crate::live_clocked::SyncBinding::Shared { owner, gate } => {
            let mut same = owner.try_borrow_mut().map_err(fail)?;
            emit(&mut same, *gate, value)
        }
    }
}
struct CatalogEffects<'a, 'source> {
    catalog: &'a Catalog<'source>,
    presentation: &'a crate::fx_manager::State<crate::fx_graph::Command>,
}
impl Effects for CatalogEffects<'_, '_> {
    fn diagnostic(&mut self, _: ActorId, value: Diagnostic) -> Result<()> {
        let mut scope = self
            .catalog
            .load_configuration_sync
            .try_borrow_mut()
            .map_err(fail)?;
        log_position(
            scope
                .as_mut()
                .ok_or_else(|| fail("4D0EE0 actual retained SyncA scope is unbound"))?,
            value,
        )
    }
    fn presentation_available(&self) -> Result<bool> {
        Ok(self.presentation.attached_view().is_some())
    }
}
///Whole-Staged caller uses the same source/phase/terrain owners in place.
///The real native SyncA scope must be installed on Catalog when logs are reached.
pub fn relocate(
    catalog: &Catalog<'_>,
    staged: &mut crate::restoration::Staged,
    actor: ActorId,
    point: [u32; 2],
    pc: Precision,
) -> std::result::Result<(), String> {
    let world = staged
        .load_world
        .as_mut()
        .ok_or("4EEB40 actual World owner absent")?;
    let phase = world.dispatcher.as_ref().map(|d| &d.phase);
    let crate::load_world::SpatialOwner::Adopted(spatial) = &mut world.spatial else {
        return Err("4EEB40 actual published spatial owner absent".into());
    };
    if spatial.error.is_some() || !spatial.pending.is_empty() {
        return Err("4EEB40 actual spatial adoption incomplete".into());
    }
    let (_, mut actors) = staged.actors.split_minds();
    let mut effects = CatalogEffects {
        catalog,
        presentation: &world.presentation,
    };
    relocate_on_owners(
        catalog,
        &mut Context {
            actors: &mut actors,
            spatial: &mut spatial.world,
            bank: &mut staged.definitions,
            terrain: &world.terrain,
            variables: &mut world.variables,
            mission: &world.mission_fields,
            phase,
        },
        actor,
        point,
        &mut effects,
        pc,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use aa_orders::net::sync::{EntryKind, Synchronizer};
    #[test]
    fn position_sync_keeps_same_a_and_native_labels_error_prefix() {
        let mut a = Synchronizer::new(64);
        a.reset(0, 17);
        let original = &a as *const _;
        let mut calls = 0;
        let mut gate = || {
            calls += 1;
            if calls == 2 {
                Err("required current thread/SAI gate".into())
            } else {
                Ok(true)
            }
        };
        let mut sync = crate::live_clocked::SyncBinding::Direct(crate::live_clocked::SyncLog {
            owner: &mut a,
            gate: &mut gate,
        });
        log_position(&mut sync, Diagnostic::Identity(0x20007)).unwrap();
        assert!(log_position(&mut sync, Diagnostic::X(0x80000000)).is_err());
        drop(sync);
        assert_eq!(original, &a as *const _);
        assert_eq!(calls, 2);
        assert_eq!(a.end(), 2);
        let entry = a.entry(1).unwrap();
        assert_eq!(entry.label, "(int)GetID()");
        assert_eq!(entry.kind, EntryKind::Int);
        assert_eq!(entry.value, 0x20007);
    }
    #[test]
    fn disabled_position_logs_do_not_read_gate_and_shared_a_keeps_float_bits() {
        let cell = std::cell::RefCell::new(Synchronizer::new(64));
        let before = cell.borrow().end();
        let mut gate =
            || -> std::result::Result<bool, String> { panic!("disabled native SyncA skips gate") };
        {
            let mut sync = crate::live_clocked::SyncBinding::Shared {
                owner: &cell,
                gate: &mut gate,
            };
            log_position(&mut sync, Diagnostic::X(0x80000000)).unwrap();
        }
        assert_eq!(cell.borrow().end(), before);
        cell.borrow_mut().reset(0, 19);
        let mut gate = || Ok(true);
        let mut sync = crate::live_clocked::SyncBinding::Shared {
            owner: &cell,
            gate: &mut gate,
        };
        log_position(&mut sync, Diagnostic::X(0x80000000)).unwrap();
        log_position(&mut sync, Diagnostic::Y(0x3f800000)).unwrap();
        drop(sync);
        let a = cell.borrow();
        assert_eq!(a.entry(1).unwrap().label, "GetPosition().x");
        assert_eq!(a.entry(1).unwrap().value, 0x80000000);
        assert_eq!(a.entry(2).unwrap().label, "GetPosition().y");
        assert_eq!(a.entry(2).unwrap().value, 0x3f800000);
    }
}
