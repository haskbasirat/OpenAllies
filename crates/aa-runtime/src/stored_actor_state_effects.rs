//! 4D4A90/4D16B0 field bindings over the same stored Actor allocations.
//! Earlier state/Organization/property stores survive every reached child error.
use crate::{
    actor_world::{ActorAccess, ObjectKey, RuntimeComponent, Spatial},
    catalog::Catalog,
    definitions::Bank,
    spatial_world::SpatialWorld,
};
use aa_actors::{
    ActorId,
    components::ComponentSlot as Slot,
    state_effects::{self as native, Applicability, World},
};
use aa_body::{F32, credit::Object};
use aa_numeric::Precision;
use aa_rules::native::{
    ScalarType, TypedValue,
    layout_data::{PropertyRef, RootPosition},
};
use std::{cell::RefCell, collections::BTreeMap, sync::Arc};

type Result<T> = std::result::Result<T, String>;
fn error(e: impl std::fmt::Display) -> native::Error {
    native::Error::World(e.to_string())
}
fn required<T>(s: &'static str) -> native::Result<T> {
    Err(native::Error::Missing(s))
}
///Borrow of current [GWorld+28]+A8 and its existing same-allocation alias.
///None is an explicit missing loan, distinct from native SettingsMode::Null.
pub type CurrentSettings<'a> = Option<(crate::live_placement::SettingsMode, &'a Option<u32>)>;
fn current_active_fx_only(current: CurrentSettings<'_>) -> native::Result<bool> {
    let (binding, owner) = current.ok_or(native::Error::Missing("4F0460 actual current MapResource mode loan"))?;
    crate::live_placement::read_settings_mode(true, None, binding, Some(owner))
        .map(|mode| mode.is_some_and(|mode| mode != 0))
        .map_err(error)
}
pub struct Context<'a, 'store> {
    pub actors: &'a mut ActorAccess<'store>,
    pub arena: &'a mut aa_mind::Arena,
    pub bank: &'a mut Bank,
    pub spatial: &'a mut SpatialWorld,
    pub variables: &'a mut crate::load_view_variables::State,
    pub clock: &'a aa_simulation::NativeClock,
    pub inventory: &'a aa_mapworld::WorldInventory,
    pub current_settings: CurrentSettings<'a>,
    pub presentation: &'a mut crate::fx_manager::State<crate::fx_graph::Command>,
    pub queues: &'a mut aa_world::queues::Queues,
    pub owners: &'a crate::scheduler_owners::Owners,
}
struct Bound<'a, 'store, 'source> {
    catalog: &'a Catalog<'source>,
    actors: RefCell<&'a mut ActorAccess<'store>>,
    arena: &'a mut aa_mind::Arena,
    bank: RefCell<&'a mut Bank>,
    spatial: &'a mut SpatialWorld,
    variables: &'a mut crate::load_view_variables::State,
    clock: &'a aa_simulation::NativeClock,
    inventory: &'a aa_mapworld::WorldInventory,
    current_settings: CurrentSettings<'a>,
    presentation: &'a mut crate::fx_manager::State<crate::fx_graph::Command>,
    queues: &'a mut aa_world::queues::Queues,
    owners: &'a crate::scheduler_owners::Owners,
    source_pc: Precision,
    pc: Precision,
    graphs: RefCell<BTreeMap<(ActorId, bool), Arc<crate::fx_graph::Graph>>>,
    nodes: RefCell<Vec<(Arc<crate::fx_graph::Graph>, crate::fx_graph::NodeId)>>,
}

fn source_precision(catalog: &Catalog<'_>, bank: &Bank) -> Result<Precision> {
    catalog.require_source_owner()?;
    let source = bank.combat_source()?;
    source.completed_means()?;
    if source.source_identities() != catalog.actors.sources.as_slice() {
        return Err("stored state source Bank universe differs".into());
    }
    Ok(source.source_precision())
}
fn run(
    catalog: &Catalog<'_>,
    context: &mut Context<'_, '_>,
    actor: ActorId,
    index: u8,
    old: Option<u8>,
    enter: bool,
    pc: Precision,
) -> Result<()> {
    let source_pc = source_precision(catalog, context.bank)?;
    let key = context
        .actors
        .key(actor)
        .ok_or("stored state actual GTID absent")?;
    let object = context.actors.object(key).map_err(|e| e.to_string())?;
    crate::live_object_init::Source::bind_in_bank(catalog, context.bank, &object.definition)
        .map_err(|e| e.to_string())?;
    if object.definition.native_class != 3 || object.actor.is_none() {
        return Err("stored state actual ordinary Actor layout differs".into());
    }
    let mut world = Bound {
        catalog,
        actors: RefCell::new(&mut *context.actors),
        arena: context.arena,
        bank: RefCell::new(&mut *context.bank),
        spatial: context.spatial,
        variables: context.variables,
        clock: context.clock,
        inventory: context.inventory,
        current_settings: context.current_settings,
        presentation: context.presentation,
        queues: context.queues,
        owners: context.owners,
        source_pc,
        pc,
        graphs: RefCell::new(BTreeMap::new()),
        nodes: RefCell::new(Vec::new()),
    };
    if enter {
        native::enter(&mut world, actor, index, old, pc)
    } else {
        native::leave(&mut world, actor, index, pc)
    }
    .map_err(|e| e.to_string())
}
pub fn enter(
    catalog: &Catalog<'_>,
    context: &mut Context<'_, '_>,
    actor: ActorId,
    index: u8,
    old: Option<u8>,
    pc: Precision,
) -> Result<()> {
    run(catalog, context, actor, index, old, true, pc)
}
pub fn leave(
    catalog: &Catalog<'_>,
    context: &mut Context<'_, '_>,
    actor: ActorId,
    index: u8,
    pc: Precision,
) -> Result<()> {
    run(catalog, context, actor, index, None, false, pc)
}
///Same entry for callers which have not yet split their ActorStore/Mind loans.
pub fn enter_staged(
    staged: &mut crate::restoration::Staged,
    catalog: &Catalog<'_>,
    key: ObjectKey,
    index: u8,
    old: Option<u8>,
    pc: Precision,
) -> Result<()> {
    staged_call(staged, catalog, key, index, old, true, pc)
}
pub fn leave_staged(
    staged: &mut crate::restoration::Staged,
    catalog: &Catalog<'_>,
    key: ObjectKey,
    index: u8,
    pc: Precision,
) -> Result<()> {
    staged_call(staged, catalog, key, index, None, false, pc)
}
fn staged_call(
    staged: &mut crate::restoration::Staged,
    catalog: &Catalog<'_>,
    key: ObjectKey,
    index: u8,
    old: Option<u8>,
    enter: bool,
    pc: Precision,
) -> Result<()> {
    let owner = staged
        .actors
        .id(key)
        .ok_or("stored state current GTID absent")?;
    let world = staged
        .load_world
        .as_mut()
        .ok_or("stored state actual World absent")?;
    let crate::load_world::SpatialOwner::Adopted(spatial) = &mut world.spatial else {
        return Err("stored state actual adopted SpatialWorld absent".into());
    };
    if spatial.error.is_some()
        || !spatial.pending.is_empty()
        || spatial.world.index().precision() != pc
    {
        return Err("stored state spatial adoption/live PC differs".into());
    }
    let (arena, mut actors) = staged.actors.split_minds();
    run(
        catalog,
        &mut Context {
            actors: &mut actors,
            arena,
            bank: &mut staged.definitions,
            spatial: &mut spatial.world,
            variables: &mut world.variables,
            clock: &staged.clock,
            inventory: &staged.inventory,
            current_settings: Some((world.global_settings_mode, world.map_mode.as_ref())),
            presentation: &mut world.presentation,
            queues: &mut staged.queues,
            owners: &staged.owners,
        },
        owner,
        index,
        old,
        enter,
        pc,
    )
}

fn string(value: Option<TypedValue>, field: &str) -> Result<String> {
    match value {
        Some(TypedValue::String { value, .. }) => Ok(value),
        _ => Err(format!("native state source requires typed {field}")),
    }
}
fn root_position(
    catalog: &Catalog<'_>,
    sources: &[aa_rules::resources::Provenance],
) -> Result<RootPosition> {
    let first = sources
        .first()
        .ok_or("state source root provenance absent")?;
    Ok(RootPosition {
        file: catalog
            .actors
            .sources
            .iter()
            .position(|s| s.path.eq_ignore_ascii_case(&first.path))
            .ok_or("state source file identity absent")?,
        offset: first.span.start,
    })
}
/// The same4C4CF0 source services used by ActorData support-property loading.
/// A descriptor reload keeps the existing list and prepends this source pass.
struct DescriptorProperties<'a, 'b, 'source> {
    children: Vec<aa_rules::actors::ResolvedNode<'a>>,
    catalog: &'b Catalog<'source>,
    position: RootPosition,
    precision: Precision,
}
impl aa_rules::native_member_property_data::Source for DescriptorProperties<'_, '_, '_> {
    fn child_count(&mut self) -> aa_rules::native::layout_data::Result<i32> {
        i32::try_from(self.children.len()).map_err(|_| {
            aa_rules::native::layout_data::LayoutError::Unsupported(
                "ActorState Property count outside signed native domain".into(),
            )
        })
    }
    fn property_ids(&mut self, index: i32) -> aa_rules::native::layout_data::Result<String> {
        let child = self.children.get(index as usize).ok_or_else(|| {
            aa_rules::native::layout_data::LayoutError::Unsupported(
                "ActorState Property child is unavailable".into(),
            )
        })?;
        string(child.field("property_IDS", self.precision)?, "property_IDS")
            .map_err(aa_rules::native::layout_data::LayoutError::Unsupported)
    }
    fn find_property(
        &mut self,
        ids: &str,
    ) -> aa_rules::native::layout_data::Result<Option<PropertyRef>> {
        let layout = self
            .catalog
            .layout_data(self.precision)
            .map_err(aa_rules::native::layout_data::LayoutError::Unsupported)?;
        Ok(layout.find_property(ids, Some(self.position)))
    }
}
impl Bound<'_, '_, '_> {
    fn key(&self, actor: ActorId) -> native::Result<ObjectKey> {
        self.actors
            .borrow()
            .key(actor)
            .ok_or(native::Error::Missing("stored state current GTID"))
    }
    fn component(&self, actor: ActorId, slot: Slot) -> native::Result<bool> {
        Ok(self
            .actors
            .borrow()
            .component(actor, slot)
            .map_err(error)?
            .is_some())
    }
    fn word(&self, actor: ActorId, offset: u32) -> native::Result<u32> {
        let actors = self.actors.borrow();
        let object = actors
            .object(
                actors
                    .key(actor)
                    .ok_or(native::Error::Missing("stored state current GTID"))?,
            )
            .map_err(error)?;
        object
            .actor
            .as_ref()
            .and_then(|a| a.words.get(&offset))
            .copied()
            .ok_or_else(|| error(format!("stored state Actor+{offset:X} is unwritten")))
    }
    fn definition_id(&self, actor: ActorId) -> native::Result<String> {
        Ok(self
            .actors
            .borrow()
            .object(self.key(actor)?)
            .map_err(error)?
            .definition
            .proof
            .id
            .clone())
    }
    fn property_definition(&self, symbol: Object) -> native::Result<Object> {
        let reference = PropertyRef::from_handle(symbol.0).ok_or(native::Error::Missing(
            "stored state Property source handle",
        ))?;
        let mut actors = self.actors.borrow_mut();
        let mut bank = self.bank.borrow_mut();
        actors
            .property_allocations
            .bind_in_bank(self.catalog, &mut bank)
            .map_err(error)?;
        actors
            .property_allocations
            .definition_for_reference(reference)
            .map_err(error)
    }
    fn property_symbol(&self, id: &str, at: RootPosition) -> native::Result<Object> {
        let data = self.catalog.layout_data(self.source_pc).map_err(error)?;
        data.find_property(id, Some(at))
            .ok_or(native::Error::Missing(
                "state source immediate Property Find",
            ))
            .and_then(|p| p.handle().map(Object).map_err(error))
    }
    fn source_stances(&self, actor: ActorId) -> native::Result<Vec<native::ViewRule>> {
        let id = self.definition_id(actor)?;
        let source = self.catalog.actors.resolved_actor(&id).map_err(error)?;
        let at = root_position(self.catalog, &source.sources()).map_err(error)?;
        source
            .children("Stance")
            .map_err(error)?
            .iter()
            .map(|node| {
                let limit = match node.field("health", self.source_pc).map_err(error)? {
                    Some(TypedValue::Scalar(v)) if v.field_type == ScalarType::Fixed => v.bits,
                    _ => return required("4B4640 actual Fixed stance health"),
                };
                let states = string(
                    node.field("actor_state", self.source_pc).map_err(error)?,
                    "actor_state",
                )
                .map_err(error)?;
                let mut mask = 0u32;
                for name in states.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                    let state = aa_actors::state::DESCRIPTORS
                        .iter()
                        .find(|d| d.name.eq_ignore_ascii_case(name))
                        .ok_or(native::Error::Missing("4B4640 registered ActorState token"))?;
                    mask |= 1u32.wrapping_shl(u32::from(state.index));
                }
                let property = string(
                    node.field("property_IDS", self.source_pc).map_err(error)?,
                    "property_IDS",
                )
                .map_err(error)?;
                let property = if property.is_empty() {
                    None
                } else {
                    Some(self.property_symbol(&property, at)?)
                };
                Ok(native::ViewRule {
                    health_limit: F32(limit),
                    required_low_states: mask,
                    property,
                })
            })
            .collect()
    }
    fn actor_property(&self, actor: ActorId, definition: Object) -> native::Result<bool> {
        let actors = self.actors.borrow();
        let Some(component) = actors.component(actor, Slot::Properties).map_err(error)? else {
            return Ok(false);
        };
        let RuntimeComponent::Properties(properties) = component else {
            return required("stored state actual Properties payload");
        };
        native::property_present(
            properties,
            actors.property_allocations.instances(),
            definition,
        )
    }
}
impl Applicability for Bound<'_, '_, '_> {
    fn has_properties(&self, a: ActorId) -> native::Result<bool> {
        self.component(a, Slot::Properties)
    }
    fn property_requirements(&self, p: Object) -> native::Result<Vec<Object>> {
        let definition = self.property_definition(p)?;
        Ok(self
            .actors
            .borrow()
            .property_allocations
            .facts(definition)
            .map_err(error)?
            .required()
            .to_vec())
    }
    fn actor_has_property(&self, a: ActorId, p: Object) -> native::Result<bool> {
        self.actor_property(a, p)
    }
    fn organization_count(&self, a: ActorId) -> native::Result<Option<usize>> {
        match self
            .actors
            .borrow()
            .component(a, Slot::Organization)
            .map_err(error)?
        {
            None => Ok(None),
            Some(RuntimeComponent::Organization(o)) if o.header.owner == a => {
                Ok(Some(o.members.len()))
            }
            _ => required("stored state Organization payload/owner"),
        }
    }
    fn organization_member(&self, a: ActorId, index: usize) -> native::Result<Option<ActorId>> {
        match self
            .actors
            .borrow()
            .component(a, Slot::Organization)
            .map_err(error)?
        {
            Some(RuntimeComponent::Organization(o)) if o.header.owner == a => o
                .members
                .get(index)
                .copied()
                .map(|id| (!id.is_null()).then_some(id))
                .ok_or(native::Error::Missing("stored state live member index")),
            _ => required("stored state Organization payload/owner"),
        }
    }
    fn vacant_member_definition(&self, a: ActorId, index: usize) -> native::Result<Option<Object>> {
        let actors = self.actors.borrow();
        let Some(RuntimeComponent::Organization(o)) =
            actors.component(a, Slot::Organization).map_err(error)?
        else {
            return required("stored state Organization allocation");
        };
        let data = self.catalog.layout_data(self.source_pc).map_err(error)?;
        let layout = data
            .find_layout(&o.layout.definition)
            .ok_or(native::Error::Missing("stored state actual LayoutData"))?;
        let group = *layout
            .slot_groups
            .get(index)
            .ok_or(native::Error::Missing("stored state layout slot"))?;
        o.layout
            .parameters
            .get(group as usize)
            .ok_or(native::Error::Missing("stored state layout group"))?
            .as_ref()
            .map(|id| {
                self.catalog
                    .require_untranslated_actor_identity(id)
                    .map_err(error)?;
                self.bank
                    .borrow_mut()
                    .intern("ActorData", id)
                    .map(Object)
                    .map_err(error)
            })
            .transpose()
    }
    fn definition_component_mask(&self, d: Object) -> native::Result<u32> {
        let bank = self.bank.borrow();
        let entry = bank.entry(d.0).map_err(error)?;
        if entry.key.domain != "actordata" {
            return required("stored state vacant ActorData identity domain");
        }
        self.catalog
            .canonical_component_mask(&entry.key.name, self.source_pc)
            .map_err(error)
    }
    fn definition_properties(&self, d: Object) -> native::Result<Vec<Object>> {
        let bank = self.bank.borrow();
        let entry = bank.entry(d.0).map_err(error)?;
        if entry.key.domain != "actordata" {
            return required("stored state vacant ActorData identity domain");
        }
        let data = self.catalog.layout_data(self.source_pc).map_err(error)?;
        let references = aa_rules::native::layout_data::native_actor_properties(
            self.catalog.actors,
            &data,
            &entry.key.name,
            self.source_pc,
        )
        .map_err(error)?;
        let actors = self.actors.borrow();
        references
            .into_iter()
            .map(|p| {
                actors
                    .property_allocations
                    .definition_for_reference(p)
                    .map_err(error)
            })
            .collect()
    }
}
impl World for Bound<'_, '_, '_> {
    fn descriptor(&self, index: u8) -> native::Result<native::Descriptor> {
        let descriptor = aa_actors::state::DESCRIPTORS
            .get(usize::from(index))
            .ok_or(native::Error::Missing("registered ActorState index"))?;
        let registry = self
            .catalog
            .actors
            .native_data_registry("ActorState")
            .ok_or(native::Error::Missing("actual mode2 ActorState manager"))?;
        if registry.mode != Some(2) {
            return required("actual mode2 ActorState manager");
        }
        //47DD32 gates the source reads on the fixed registered group, not
        //on the authored template shape. Other groups retain474270's zero
        //1E/1F bytes even if their source inherits ActorStateMind fields.
        let mind = if descriptor.group == Some(4) {
            let source = self
                .catalog
                .actors
                .resolved_definition("ActorState", descriptor.name, self.source_pc)
                .map_err(error)?;
            let flag = |name: &str| {
                source
                    .field(name, self.source_pc)
                    .map_err(error)?
                    .ok_or(native::Error::Missing(
                        "ActorState group4 typed source field",
                    ))?
                    .boolean()
                    .map_err(error)
            };
            Some((flag("entrenchable")?, flag("remove_from_group")?))
        } else {
            None
        };
        //47DC50/4C4CF0 appends to the registered descriptor's existing head across
        //source reloads. Each source pass prepends its authored Property children.
        let mut roots = Vec::new();
        for node in self
            .catalog
            .actors
            .resolved_definitions("ActorState")
            .map_err(error)?
        {
            if string(
                node.field("IDS", self.source_pc).map_err(error)?,
                "ActorState.IDS",
            )
            .map_err(error)?
            .eq_ignore_ascii_case(descriptor.name)
            {
                let at = root_position(self.catalog, &node.sources()).map_err(error)?;
                roots.push((at, node));
            }
        }
        roots.sort_by_key(|(at, _)| *at);
        let mut properties = Vec::new();
        for (at, node) in roots {
            aa_rules::native_member_property_data::load_list(
                &mut DescriptorProperties {
                    children: node.children("Property").map_err(error)?,
                    catalog: self.catalog,
                    position: at,
                    precision: self.source_pc,
                },
                &mut properties,
            )
            .map_err(error)?;
        }
        let properties = properties
            .into_iter()
            .map(|p| p.handle().map(Object).map(Some).map_err(error))
            .collect::<native::Result<_>>()?;
        let (entrenchable, remove_from_group) = mind.unwrap_or((false, false));
        Ok(native::Descriptor {
            index,
            properties,
            entrenchable,
            remove_from_group,
        })
    }
    fn has_component(&self, a: ActorId, s: Slot) -> native::Result<bool> {
        self.component(a, s)
    }
    fn state_bits(&self, a: ActorId) -> native::Result<u64> {
        let actors = self.actors.borrow();
        let state = actors
            .object(self.key(a)?)
            .map_err(error)?
            .components()
            .map_err(error)?
            .state;
        Ok(u64::from(state.low) | (u64::from(state.high) << 32))
    }
    fn body_health(&self, a: ActorId) -> native::Result<Option<(F32, F32)>> {
        match self
            .actors
            .borrow()
            .component(a, Slot::Body)
            .map_err(error)?
        {
            None => Ok(None),
            Some(RuntimeComponent::Body(b)) if b.header.owner == a => {
                Ok(Some((b.body.health, b.body.maximum)))
            }
            _ => required("stored state Body payload/owner"),
        }
    }
    fn view_rules(&self, a: ActorId) -> native::Result<Vec<native::ViewRule>> {
        self.source_stances(a)
    }
    fn current_view(&self, a: ActorId) -> native::Result<i32> {
        Ok(self.word(a, 0x15c)? as i32)
    }
    fn set_current_view(&mut self, a: ActorId, index: i32) -> native::Result<()> {
        let key = self.key(a)?;
        let mut actors = self.actors.borrow_mut();
        let words = &mut actors
            .object_mut(key)
            .map_err(error)?
            .actor
            .as_mut()
            .ok_or(native::Error::Missing("stored state Actor"))?
            .words;
        *words
            .get_mut(&0x15c)
            .ok_or(native::Error::Missing("stored state Actor15C"))? = index as u32;
        Ok(())
    }
    fn character_view(&mut self, actor: ActorId, index: i32) -> native::Result<()> {
        crate::stored_character_view::set(
            self.catalog,
            &mut Context {
                actors: self.actors.get_mut(),
                arena: self.arena,
                bank: self.bank.get_mut(),
                spatial: self.spatial,
                variables: self.variables,
                clock: self.clock,
                inventory: self.inventory,
                current_settings: self.current_settings,
                presentation: self.presentation,
                queues: self.queues,
                owners: self.owners,
            },
            actor,
            index,
            self.pc,
        )
        .map(|_| ())
        .map_err(error)
    }
    fn add_property(
        &mut self,
        a: ActorId,
        p: Object,
        kind: u32,
        source: ActorId,
    ) -> native::Result<()> {
        let definition = self.property_definition(p)?;
        let reference = self
            .actors
            .borrow()
            .property_allocations
            .facts(definition)
            .map_err(error)?
            .reference();
        crate::stored_state_properties::add(
            self.catalog,
            self.actors.get_mut(),
            self.arena,
            self.bank.get_mut(),
            self.spatial,
            self.variables,
            self.clock,
            self.inventory,
            a,
            reference,
            kind,
            Some(source),
            self.pc,
        )
        .map_err(error)
    }
    fn remove_property(
        &mut self,
        a: ActorId,
        p: Option<Object>,
        source: ActorId,
    ) -> native::Result<()> {
        let reference = p
            .map(|p| {
                PropertyRef::from_handle(p.0).ok_or(native::Error::Missing(
                    "stored state source Property handle",
                ))
            })
            .transpose()?;
        crate::stored_state_properties::remove_reference(
            self.catalog,
            self.actors.get_mut(),
            self.bank.get_mut(),
            self.variables,
            a,
            reference,
            source,
            self.pc,
        )
        .map_err(error)
    }
    fn organization_transition(
        &mut self,
        a: ActorId,
        new: &native::Descriptor,
        old: Option<&native::Descriptor>,
    ) -> native::Result<()> {
        let mut actors = self.actors.borrow_mut();
        let Some(RuntimeComponent::Organization(o)) =
            actors.component_mut(a, Slot::Organization).map_err(error)?
        else {
            return required("57C400 actual Organization allocation");
        };
        if o.header.owner != a {
            return required("57C400 actual Organization owner");
        }
        native::organization_transition(o, new, old, F32(self.clock.begin.0), |_| {
            required("57BFF0 actual queued Organization command")
        })
    }
    fn refresh_economy(&mut self, a: ActorId) -> native::Result<()> {
        crate::stored_economy_refresh::refresh(
            self.catalog,
            self.actors.get_mut(),
            self.bank.get_mut(),
            self.spatial,
            self.variables,
            a,
            self.pc,
        )
        .map_err(error)
    }
    fn state_fx(
        &self,
        actor: ActorId,
        list: native::FxList,
    ) -> native::Result<Vec<native::StateFx>> {
        let enter = matches!(list, native::FxList::Enter);
        let key = (actor, enter);
        if !self.graphs.borrow().contains_key(&key) {
            let actors = self.actors.borrow();
            let object = actors.object(self.key(actor)?).map_err(error)?;
            let bank = self.bank.borrow();
            let graph = if enter {
                crate::fx_graph::Graph::compile_actor_enter_in_bank(
                    self.catalog,
                    &bank,
                    &object.definition,
                )
            } else {
                crate::fx_graph::Graph::compile_actor_leave_in_bank(
                    self.catalog,
                    &bank,
                    &object.definition,
                )
            }
            .map_err(error)?;
            self.graphs.borrow_mut().insert(key, Arc::new(graph));
        }
        let graph = Arc::clone(
            self.graphs
                .borrow()
                .get(&key)
                .expect("same inserted source graph"),
        );
        let mut nodes = self.nodes.borrow_mut();
        let mut result = Vec::new();
        for &id in graph.roots() {
            let state_index = graph
                .nodes()
                .get(id.index())
                .and_then(|n| n.word(0x34))
                .ok_or(native::Error::Missing("actual compiled state FXC+34"))?;
            let handle = Object(
                u32::try_from(nodes.len())
                    .ok()
                    .and_then(|n| n.checked_add(1))
                    .ok_or(native::Error::Missing(
                        "state FX portable node reference exhausted",
                    ))?,
            );
            nodes.push((Arc::clone(&graph), id));
            result.push(native::StateFx {
                handle,
                state_index,
            });
        }
        Ok(result)
    }
    fn active_fx_only(&self) -> native::Result<bool> {
        current_active_fx_only(self.current_settings)
    }
    fn fx_active(&mut self, handle: Object) -> native::Result<bool> {
        let nodes = self.nodes.borrow();
        let (graph, id) = nodes
            .get(
                handle
                    .0
                    .checked_sub(1)
                    .ok_or(native::Error::Missing("state FX node reference is null"))?
                    as usize,
            )
            .ok_or(native::Error::Missing("state FX node reference absent"))?;
        graph
            .actor_state_node_active_in_bank(self.catalog, &self.bank.borrow(), *id)
            .map_err(error)
    }
    fn emit_state_fx(
        &mut self,
        actor: ActorId,
        list: native::FxList,
        state: u8,
        active_only: bool,
    ) -> native::Result<()> {
        //The shared4D0A00 kernel has just found the matching root. Retain the
        //same immutable graph rather than rescanning virtual predicates.
        let graph = Arc::clone(
            self.graphs
                .get_mut()
                .get(&(actor, matches!(list, native::FxList::Enter)))
                .ok_or(native::Error::Missing(
                    "stored state FX current scanned graph",
                ))?,
        );
        crate::stored_actor_state_fx::publish(
            self.catalog,
            self.actors.get_mut(),
            self.bank.get_mut(),
            self.spatial,
            self.clock,
            actor,
            crate::fx_graph::MatchedState {
                graph,
                state: u32::from(state),
                active_only,
            },
            self.presentation,
        )
        .map_err(error)
    }
    fn group_reference(&self, a: ActorId) -> native::Result<ActorId> {
        Ok(ActorId::from_raw(self.word(a, 0x164)?))
    }
    fn resolve_reference(&self, id: ActorId) -> native::Result<Option<ActorId>> {
        Ok(self.actors.borrow().key(id).map(|_| id))
    }
    fn remove_from_group(&mut self, _: ActorId, _: ActorId) -> native::Result<()> {
        required("4DFF30 actual current Group removal")
    }
    fn thing_flags(&self, a: ActorId) -> native::Result<u32> {
        Ok(self
            .actors
            .borrow()
            .placement(self.key(a)?, &*self.spatial)
            .map_err(error)?
            .flags)
    }
    fn definition_flags(&self, a: ActorId) -> native::Result<u32> {
        Ok(self
            .actors
            .borrow()
            .object(self.key(a)?)
            .map_err(error)?
            .definition
            .flags)
    }
    fn set_thing_flags(&mut self, a: ActorId, flags: u32) -> native::Result<()> {
        let key = self.key(a)?;
        let old = self.thing_flags(a)?;
        if old == flags {
            return Ok(());
        }
        if let Ok(p) = self.actors.get_mut().staged_placement_mut(key) {
            p.flags = flags;
            return Ok(());
        }
        if flags == (old | 0x1000) {
            return self.spatial.mark_falling(key).map_err(error);
        }
        required("stored state clearing published Thing flag owner")
    }
}

#[cfg(test)]
mod current_map_mode_tests {
    use super::*;
    use crate::{live_placement::SettingsMode, load_world_settings::SameWorldSettings};
    fn fixture() -> serde_json::Value {
        serde_json::from_str(include_str!("../../../research/development10/state-fx-map-mode-review/native-current-mode.json")).unwrap()
    }
    #[test]
    fn native_current_map_mode_save_load_restore_intervals() {
        let fixture = fixture();
        assert_eq!(fixture["native_executions"], 60);
        let rows = fixture["interval_rows"].as_array().unwrap();
        assert_eq!(rows.len(), 27);
        for row in rows {
            let mut owner = Box::new(Some(row["prior"].as_u64().unwrap() as u32));
            let alias = SettingsMode::SameWorld(SameWorldSettings::from_constructed_world(owner.as_ref()).unwrap());
            let mut reads = vec![current_active_fx_only(Some((alias, owner.as_ref()))).unwrap()];
            *owner = Some(row["captured_load_argument"].as_u64().unwrap() as u32);
            reads.push(current_active_fx_only(Some((alias, owner.as_ref()))).unwrap());
            *owner = Some(row["prior"].as_u64().unwrap() as u32);
            reads.push(current_active_fx_only(Some((alias, owner.as_ref()))).unwrap());
            let expected = row["native"]["active_reads"].as_array().unwrap().iter()
                .map(|v| v.as_u64().unwrap() != 0).collect::<Vec<_>>();
            assert_eq!(reads, expected, "{row}");
        }
    }
    #[test]
    fn native_current_map_mode_null_pointer_swap_and_word_reread() {
        for row in fixture()["pointer_rows"].as_array().unwrap() {
            let unavailable = None;
            let a = Box::new(Some(1));
            let mut b = Box::new(Some(0));
            let alias_a = SettingsMode::SameWorld(SameWorldSettings::from_constructed_world(a.as_ref()).unwrap());
            let alias_b = SettingsMode::SameWorld(SameWorldSettings::from_constructed_world(b.as_ref()).unwrap());
            let mut reads = vec![
                current_active_fx_only(Some((SettingsMode::Null, &unavailable))).unwrap(),
                current_active_fx_only(Some((alias_a, a.as_ref()))).unwrap(),
                current_active_fx_only(Some((alias_b, b.as_ref()))).unwrap(),
            ];
            *b = Some(0x80000000);
            reads.push(current_active_fx_only(Some((alias_b, b.as_ref()))).unwrap());
            let expected = row["native"]["reads"].as_array().unwrap().iter()
                .map(|v| v.as_u64().unwrap() != 0).collect::<Vec<_>>();
            assert_eq!(reads, expected, "{row}");
        }
    }
    #[test]
    fn missing_loan_unbound_word_and_foreign_owner_are_not_native_null() {
        let mut owner = Box::new(Some(1));
        let alias = SettingsMode::SameWorld(SameWorldSettings::from_constructed_world(owner.as_ref()).unwrap());
        assert!(current_active_fx_only(None).is_err());
        assert!(current_active_fx_only(Some((SettingsMode::Unbound, owner.as_ref()))).is_err());
        let foreign = Box::new(Some(1));
        assert!(current_active_fx_only(Some((alias, foreign.as_ref()))).is_err());
        assert!(!current_active_fx_only(Some((SettingsMode::Null, owner.as_ref()))).unwrap());
        *owner = None;
        assert!(current_active_fx_only(Some((alias, owner.as_ref()))).is_err());
    }
}
