//! Native WorldInit solver allocations and zone presentation gates.
//! Operational solver scratch/tiles stay in the same ZoneMap. The presentation
//! manager and dispatcher phase are borrowed from their single existing owners.
use crate::{fx_manager, load_detection_activation, load_view_variables};
#[path="zone_spatial_read.rs"]
mod spatial_read;
#[path="zone_effects_gate.rs"]
mod effects_gate;
#[path="zone_actor_reads.rs"]
mod actor_reads;
use aa_zones::{
    Error, Result,
    flood::Scratch,
    tile::{MapGeometry, ZoneMap},
};

fn error(value: impl std::fmt::Display) -> Error {
    Error::World(value.to_string())
}
enum Class {
    Release,
    Move,
}
struct Allocation {
    class: Class,
}
trait Observer {
    fn city_definition(&self,definition:&aa_mapworld::restore::RuntimeDefinition)->Result<()>;
    fn city_source_byte(&self,definition:&aa_mapworld::restore::RuntimeDefinition,offset:u32)->Result<u8>;
    fn city_pending(&self,owner:aa_actors::ActorId,pc:aa_numeric::Precision)->Result<bool>;
    fn city_schedule(&self,owner:aa_actors::ActorId,pc:aa_numeric::Precision,rng:&mut aa_simulation::NativeRng)->Result<()>;
    fn multi_tile(&self,flags:u32,definition:&aa_mapworld::restore::RuntimeDefinition,cell:u32,pc:aa_numeric::Precision)->Result<bool>;
    fn active(&self) -> Result<bool>;
    fn reveal(
        &self,
        services: &mut dyn crate::load_actor_reveal::Services,
        fields: &mut crate::load_detection_memory::Fields<'_>,
        kingdom: u32,
        explored: bool,
        visible: bool,
    ) -> Result<()>;
}
impl Observer for crate::catalog::Catalog<'_> {
    fn city_definition(&self,definition:&aa_mapworld::restore::RuntimeDefinition)->Result<()>{
        crate::live_object_init::Source::bind(self,definition).map(|_|()).map_err(error)
    }
    fn city_source_byte(&self,definition:&aa_mapworld::restore::RuntimeDefinition,offset:u32)->Result<u8>{
        crate::load_city_candidate::source_byte(self,definition,offset).map_err(error)
    }
    fn city_pending(&self,owner:aa_actors::ActorId,pc:aa_numeric::Precision)->Result<bool>{
        crate::load_detection_init_world::pending_component(self,owner,aa_actors::components::ComponentSlot::City,pc).map_err(error)
    }
    fn city_schedule(&self,owner:aa_actors::ActorId,pc:aa_numeric::Precision,rng:&mut aa_simulation::NativeRng)->Result<()>{
        crate::load_detection_init_world::schedule_component(self,owner,aa_actors::components::ComponentSlot::City,rng,pc).map_err(error)
    }
    fn multi_tile(&self,flags:u32,definition:&aa_mapworld::restore::RuntimeDefinition,cell:u32,pc:aa_numeric::Precision)->Result<bool> {
        if flags&0x20000400!=0x20000400 {return Ok(false)}
        let source=crate::live_object_init::Source::bind(self,definition).map_err(error)?;
        aa_spatial::geometry::marks_blocker(flags,source.blocking_shape().radius_bits,cell,pc).map_err(error)
    }
    fn active(&self) -> Result<bool> {
        self.saved_observer_active().map_err(error)
    }
    fn reveal(
        &self,
        services: &mut dyn crate::load_actor_reveal::Services,
        fields: &mut crate::load_detection_memory::Fields<'_>,
        kingdom: u32,
        explored: bool,
        visible: bool,
    ) -> Result<()> {
        services
            .reveal(self, fields, kingdom, explored, visible)
            .map_err(error)
    }
}

/// World+244/+248 are real allocated class owners. Their algorithms operate on
/// ZoneMap's one shared solver state; this type contains no second scratch map,
/// tile sets, phase word or presentation view. No Clone/Default constructor.
pub struct State {
    release: Box<Allocation>,
    movement: Box<Allocation>,
    map_owner: aa_zones::tile::MapOwnerWitness,
    tile_count: usize,
    geometry: MapGeometry,
}
impl State {
    /// Successful WorldInit4F2AE5..2B28:53F840 then5402A0. Native53CDF0's
    /// 8092-entry reservation/cap is replaced by the existing growable solver
    /// queue (user-requested removal of engine capacity limits).
    pub fn from_world_init(map: &ZoneMap) -> Result<Self> {
        if map.tiles.is_empty()
            || map
                .tiles
                .iter()
                .any(|t| t.set.is_some() || t.fog != 0 || t.supply != 0)
            || map.solver.busy
            || map.solver.stamp != 0
            || map.solver.scratch.len() != map.tiles.len()
            || map.solver.scratch.iter().any(|s| *s != Scratch::Bits(0))
        {
            return Err(error(
                "zone WorldInit requires its same fresh native map/solver state",
            ));
        }
        Ok(Self {
            release: Box::new(Allocation {
                class: Class::Release,
            }),
            movement: Box::new(Allocation { class: Class::Move }),
            map_owner: map.owner_witness(),
            tile_count: map.tiles.len(),
            geometry: map.geometry,
        })
    }
    pub(crate) fn validate_map(&self, map: &ZoneMap) -> Result<()> {
        if !self.map_owner.matches(map)
            || map.tiles.len() != self.tile_count
            || map.geometry != self.geometry
        {
            return Err(error(
                "zone solver allocation belongs to another native map owner",
            ));
        }
        Ok(())
    }
    /// Internal World wiring must pass that map's paired actor-list index.
    /// Geometry checks are structural; this is not a battle-readiness token.
    pub(crate) fn bind<'a, T>(
        &'a self,
        catalog: &'a crate::catalog::Catalog<'_>,
        map: &ZoneMap,
        index: &'a aa_spatial::index::SpatialIndex,
        phase: &'a u32,
        manager: &'a fx_manager::State<T>,
        variables: &'a load_view_variables::State,
    ) -> Result<Bound<'a, T>> {
        self.validate_map(map)?;
        let grid = index.grid();
        if grid.dimensions != [map.geometry.width, map.geometry.height]
            || grid.coordinate_shift != map.geometry.shift
            || grid.cell_size_bits != map.geometry.tile_size_bits
        {
            return Err(error(
                "zone global services spatial-index/map owner dimensions differ",
            ));
        }
        variables.validate_catalog(catalog).map_err(error)?;
        variables.require_published().map_err(error)?;
        Ok(Bound {
            state: self,
            phase,
            manager,
            variables,
            observer: catalog,
            index,
            reveal: None,
            effects_state: None,
            city_rng:None,
        })
    }
}

/// `phase` must borrow the actual retained DispatcherState.phase (World+D8).
/// Map's short validation borrow ends before its exclusive zone solve borrow.
pub struct Bound<'a, T> {
    state: &'a State,
    phase: &'a u32,
    manager: &'a fx_manager::State<T>,
    variables: &'a load_view_variables::State,
    observer: &'a dyn Observer,
    index: &'a aa_spatial::index::SpatialIndex,
    reveal: Option<&'a mut dyn crate::load_actor_reveal::Services>,
    effects_state: Option<(&'a dyn crate::load_world_globals::SessionPhase, &'a crate::mission_fields::State)>,
    city_rng:Option<&'a mut aa_simulation::NativeRng>,
}
impl<'a, T> Bound<'a, T> {
    pub(crate) fn with_city_rng(mut self,rng:Option<&'a mut aa_simulation::NativeRng>)->Self{
        self.city_rng=rng;self
    }
    pub(crate) fn with_effect_state(mut self,
        session: &'a dyn crate::load_world_globals::SessionPhase,
        mission: &'a crate::mission_fields::State,
    )->Self {
        self.effects_state=Some((session,mission));
        self
    }
    pub(crate) fn with_reveal(
        mut self,
        services: &'a mut dyn crate::load_actor_reveal::Services,
    ) -> Self {
        self.reveal = Some(services);
        self
    }
}
fn present(phase: u32, mut view: impl FnMut() -> Option<u64>) -> Result<()> {
    if phase == 6 {
        return Ok(());
    }
    if view().is_some() {
        //50C4D4 tests +4, then50C4EC rereads it for the reached child.
        view().ok_or_else(|| error("50C4C0 reached null presentation-view reread"))?;
        return Err(error(
            "50C4C0 requires actual43BA80 attached-view zone presentation",
        ));
    }
    Ok(())
}
impl<T> load_detection_activation::Effects for Bound<'_, T> {
    fn city_definition(&mut self,definition:&aa_mapworld::restore::RuntimeDefinition)->Result<()>{
        self.observer.city_definition(definition)
    }
    fn city_source_byte(&mut self,definition:&aa_mapworld::restore::RuntimeDefinition,offset:u32)->Result<u8>{
        self.observer.city_source_byte(definition,offset)
    }
    fn city_pending(&mut self,owner:aa_actors::ActorId,pc:aa_numeric::Precision)->Result<bool>{
        self.observer.city_pending(owner,pc)
    }
    fn city_schedule(&mut self,owner:aa_actors::ActorId,pc:aa_numeric::Precision)->Result<()>{
        let rng=self.city_rng.as_deref_mut().ok_or_else(||error("City scheduling requires same actual load RNG owner"))?;
        self.observer.city_schedule(owner,pc,rng)
    }
    fn tile_primary_reference(&mut self,tile:usize)->Result<u32> {
        spatial_read::primary_at(self.index,tile).map_err(error)
    }
    fn actor_multi_tile(&mut self,flags:u32,definition:&aa_mapworld::restore::RuntimeDefinition)->Result<bool> {
        self.observer.multi_tile(flags,definition,self.state.geometry.tile_size_bits,self.index.precision())
    }
    fn actor_tile(&mut self,cached:[u32;2])->Result<i64> {
        actor_reads::cached_tile(self.state.geometry.width,cached).map_err(error)
    }
    fn zone_effects_enabled(&mut self)->Result<bool> {
        let (session,mission)=self.effects_state.ok_or_else(||error("zone actual Session+E4 /World+48 owner absent"))?;
        effects_gate::enabled(||session.application_state().map_err(error),||Ok(mission.started48()))
    }
    fn tile_flags(&mut self,tile:usize)->Result<u32> {
        //bind() validated SAME map/index geometry. Read its current word+20.
        spatial_read::flags_at(self.index,tile).map_err(error)
    }
    fn actor_reveal(
        &mut self,
        fields: &mut crate::load_detection_memory::Fields<'_>,
        kingdom: u32,
        explored: bool,
        visible: bool,
    ) -> Result<()> {
        let service = self
            .reveal
            .as_deref_mut()
            .ok_or_else(|| error("zone Actor64 actual reveal context absent"))?;
        self.observer
            .reveal(service, fields, kingdom, explored, visible)
    }
    fn sai_enabled(&mut self) -> Result<bool> {
        self.observer.active()
    }
    fn tile_actors(&mut self, tile: usize) -> Result<Vec<u32>> {
        let width = self.state.geometry.width as usize;
        if tile >= self.state.tile_count {
            return Err(Error::TileOutside(tile as i64));
        }
        let mut node = self
            .index
            .tile([(tile % width) as i32, (tile / width) as i32])
            .map_err(error)?
            .first_node;
        let mut seen = std::collections::BTreeSet::new();
        let mut actors = Vec::new();
        while let Some(key) = node {
            if !seen.insert(key) {
                return Err(error("zone native tile node chain is cyclic"));
            }
            let value = self.index.node(key).map_err(error)?;
            // Persistent generation-bearing identity is readable even when
            // member fields are outside a load lease. Never cast ActorKey.
            actors.push(self.index.reference_for_key(value.actor).map_err(error)?);
            node = value.next;
        }
        Ok(actors)
    }
    fn solver_present(&mut self, release: bool) -> Result<bool> {
        let allocation = if release {
            &self.state.release
        } else {
            &self.state.movement
        };
        match (&allocation.class, release) {
            (Class::Release, true) | (Class::Move, false) => Ok(true),
            _ => Err(error("native zone solver allocation class differs")),
        }
    }
    fn fast_detection_zones(&mut self) -> Result<bool> {
        Ok(self.variables.fast_detection_zones())
    }
    fn fog_enabled(&mut self) -> Result<bool> {
        Ok(self.variables.fog_enabled())
    }
    fn presentation(
        &mut self,
        _owner: u32,
        _x: u32,
        _y: u32,
        _radius: u32,
        _flags: u16,
    ) -> Result<()> {
        present(*self.phase, || self.manager.attached_view())
    }
}

#[cfg(test)]
#[path = "load_zone_globals_tests.rs"]
mod tests;
