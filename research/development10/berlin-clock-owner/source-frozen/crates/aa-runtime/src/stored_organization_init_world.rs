//! Organization57C870/579DD0 on the installed stored component and moved Bank.
//! Layout/member stores commit before each native child; positive unbound
//! formation and Element callbacks retain their precise reached boundary.
use crate::{
    actor_world::{ActorStore, ObjectKey, RuntimeComponent},
    catalog::{ActorBindings, Catalog},
    restoration::Staged,
};
use aa_actors::{
    ActorId,
    actor_creation::{Error, Result},
    components::{ComponentSlot as Slot, OtherComponents},
    ground_states::OrganizationState,
    organization_state::{self, MaximumMoraleWorld},
};
use aa_body::F32;
use aa_numeric::{Binary, Precision};
use std::{collections::BTreeMap, sync::Arc};
fn error(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}
fn load_error(e: impl std::fmt::Display) -> organization_state::LoadError {
    organization_state::LoadError::World(e.to_string())
}
fn org(actors: &ActorStore, id: ActorId) -> Result<&OrganizationState<String>> {
    match actors.component(id, Slot::Organization).map_err(error)? {
        Some(RuntimeComponent::Organization(v)) if v.header.owner == id => Ok(v),
        _ => Err(error(
            "stored57C870 actual Organization allocation/owner differs",
        )),
    }
}
fn org_mut(actors: &mut ActorStore, id: ActorId) -> Result<&mut OrganizationState<String>> {
    match actors
        .component_mut(id, Slot::Organization)
        .map_err(error)?
    {
        Some(RuntimeComponent::Organization(v)) if v.header.owner == id => Ok(v),
        _ => Err(error(
            "stored57C870 actual Organization allocation/owner differs",
        )),
    }
}
fn checked(
    catalog: &Catalog<'_>,
    staged: &Staged,
    key: ObjectKey,
) -> Result<(ActorId, Arc<ActorBindings>)> {
    let binding =
        crate::map_component_init_create::actual_binding(staged, catalog, key).map_err(error)?;
    let id = staged
        .actors
        .id(key)
        .ok_or_else(|| error("stored57C870 actual GTID absent"))?;
    let r = staged.actors.object(key).map_err(error)?;
    let source =
        crate::live_object_init::Source::bind_in_bank(catalog, &staged.definitions, &r.definition)?;
    if staged.actors.key(id) != Some(key)
        || r.kind != aa_actors::loading::LoadKind::Actor
        || r.actor.is_none()
        || r.object.is_none()
        || r.definition.native_class != 3
        || r.definition.native_class != r.definition.proof.category.runtime_class
        || source.source_precision() != binding.precision
        || !crate::load_spatial::same_proof(&r.thing.definition, &r.definition.proof)
        || !crate::load_spatial::same_proof(&r.definition.proof, &binding.definition.proof)
        || r.definition.component_mask != binding.definition.component_mask
        || r.definition.flags != binding.definition.flags
        || r.definition
            .component_mask
            .is_none_or(|m| m & (1 << Slot::Organization.index()) == 0)
    {
        return Err(error("stored57C870 actual source/factory/storage differs"));
    }
    let leaves = aa_actors::component_virtuals::leaves(
        aa_actors::init_create::FactoryKind::Standard(Slot::Organization),
    );
    if leaves.init_existing != 0x57c870 || leaves.vf14 != 0x579dd0 {
        return Err(error("stored Organization virtual8/14 differs"));
    }
    org(&staged.actors, id)?;
    Ok((id, binding))
}

///Read-only other component/member loans while the SAME Organization is lent.
struct Morale<'a, 'source> {
    catalog: &'a Catalog<'source>,
    source_pc: Precision,
    pc: Precision,
    owner: ActorId,
    current: OtherComponents<'a, RuntimeComponent>,
    identities: &'a aa_actors::IdentityRegistry<ObjectKey>,
    others: BTreeMap<ObjectKey, &'a crate::actor_world::Entity>,
    variables: &'a crate::load_view_variables::State,
    bank: &'a crate::definitions::Bank,
}
impl Morale<'_, '_> {
    fn properties(
        &self,
        id: ActorId,
    ) -> organization_state::Result<Option<&aa_body::properties::PropertiesState>> {
        let component = if id == self.owner {
            self.current.get(Slot::Properties)
        } else {
            let key = self
                .identities
                .get(id)
                .ok_or_else(|| load_error("578300 member generation absent"))?
                .value;
            self.others
                .get(&key)
                .ok_or_else(|| load_error("578300 member allocation absent"))?
                .runtime
                .as_ref()
                .ok_or_else(|| load_error("578300 member runtime absent"))?
                .components()
                .map_err(load_error)?
                .get(Slot::Properties)
        };
        match component {
            None => Ok(None),
            Some(RuntimeComponent::Properties(p)) if p.owner.0 == id.raw() => Ok(Some(p)),
            _ => Err(load_error("578300 actual Properties owner differs")),
        }
    }
}
impl MaximumMoraleWorld<String> for Morale<'_, '_> {
    fn modifier(&mut self, id: ActorId) -> organization_state::Result<Option<u32>> {
        Ok(self.properties(id)?.map(|_| id.raw()))
    }
    fn apply(&mut self, id: u32, value: u32) -> organization_state::Result<Binary> {
        let p = self
            .properties(ActorId::from_raw(id))?
            .ok_or_else(|| load_error("578300 current modifier absent"))?;
        self.variables
            .validate_in_bank(self.catalog, self.bank)
            .map_err(load_error)?;
        self.variables.require_published().map_err(load_error)?;
        let floor = F32(self
            .variables
            .combat_values()
            .bits(crate::load_combat_variables::Field::ModifierClampAdd));
        let v = p
            .modifier
            .adjust_with_floor(0x20, F32(value), floor, self.pc)
            .map_err(load_error)?;
        Binary::from_f32_bits(v.0).map_err(load_error)
    }
    fn contribution(&self, id: &String) -> organization_state::Result<u32> {
        self.catalog
            .require_untranslated_actor_identity(id)
            .map_err(load_error)?;
        aa_rules::native::actor_morale::actor_morale_contribution(
            self.catalog.actors,
            id,
            self.source_pc,
        )
        .map_err(load_error)
    }
}
fn maximum_morale(
    catalog: &Catalog<'_>,
    staged: &mut Staged,
    key: ObjectKey,
    id: ActorId,
    source_pc: Precision,
    pc: Precision,
) -> Result<()> {
    let info = aa_mapworld::restore::RestoreDefinitions::organization_layout_info(
        catalog,
        &org(&staged.actors, id)?.layout.definition,
        source_pc,
    )
    .map_err(error)?;
    let world = staged
        .load_world
        .as_ref()
        .ok_or_else(|| error("578300 actual World current SVar owner absent"))?;
    let (_, mut actors) = staged.actors.split_minds();
    let (_, focus) = actors.property_focus(key).map_err(error)?;
    let a = focus
        .entity
        .runtime
        .as_mut()
        .ok_or_else(|| error("578300 actual runtime absent"))?
        .actor
        .as_mut()
        .ok_or_else(|| error("578300 actual Actor absent"))?;
    let (component, current) = a.components.split_slot(Slot::Organization);
    let Some(RuntimeComponent::Organization(value)) = component else {
        return Err(error("578300 actual Organization vanished"));
    };
    organization_state::recompute_maximum_morale_fields(
        value,
        &info,
        id,
        &mut Morale {
            catalog,
            source_pc,
            pc,
            owner: id,
            current,
            identities: focus.identities,
            others: focus.others,
            variables: &world.variables,
            bank: &staged.definitions,
        },
        pc,
    )
    .map_err(error)
}
struct Existing<'a, 'source> {
    catalog: &'a Catalog<'source>,
    staged: &'a mut Staged,
    key: ObjectKey,
    id: ActorId,
    binding: &'a ActorBindings,
    pc: Precision,
}
impl Existing<'_, '_> {
    fn state_group(&self) -> Result<Option<&'static aa_actors::state::Descriptor>> {
        let state = self
            .staged
            .actors
            .object(self.key)
            .map_err(error)?
            .components()
            .map_err(error)?
            .state;
        aa_actors::state::group(u64::from(state.low) | (u64::from(state.high) << 32), 4)
            .map_err(error)
    }
    fn apply_flag(&mut self, value: u8) -> Result<()> {
        organization_state::formation_byte_fields(self, value).map_err(error)
    }
    fn set_formation(&mut self, formation: u32) -> Result<()> {
        organization_state::set_formation_fields(self, formation, self.id).map_err(error)
    }
}

impl Existing<'_, '_> {
    fn state_words(&self) -> Result<Option<organization_state::StateWords>> {
        let Some(descriptor) = self.state_group()? else {
            return Ok(None);
        };
        let registry = self
            .catalog
            .actors
            .native_data_registry("ActorState")
            .ok_or_else(|| error("57BFF0 actual ActorState manager absent"))?;
        if registry.mode != Some(2) {
            return Err(error("57BFF0 actual ActorState manager is not mode2"));
        }
        let source = self
            .catalog
            .actors
            .resolved_definition("ActorState", descriptor.name, self.binding.precision)
            .map_err(error)?;
        let mode = |name: &str,
                    parse: fn(&str) -> std::result::Result<u32, String>|
         -> Result<u32> {
            let value = source
                .field(name, self.binding.precision)
                .map_err(error)?
                .ok_or_else(|| error(format!("ActorState {}.{name} absent", descriptor.name)))?;
            let aa_rules::native::TypedValue::String { value, .. } = value else {
                return Err(error(
                    "ActorState formation/behavior requires actual String field",
                ));
            };
            parse(&value).map_err(error)
        };
        Ok(Some(organization_state::StateWords {
            word_20: mode(
                "formation_required",
                crate::organization_state_modes::formation,
            )?,
            word_24: mode(
                "behavior_required",
                crate::organization_state_modes::behavior,
            )?,
        }))
    }
}
impl organization_state::FormationFields for Existing<'_, '_> {
    fn word(&mut self, o: u32) -> organization_state::Result<u32> {
        org(&self.staged.actors, self.id)
            .map_err(load_error)?
            .words
            .get(&o)
            .copied()
            .ok_or_else(|| load_error(format!("Organization+{o:X} field absent")))
    }
    fn put_word(&mut self, o: u32, v: u32) -> organization_state::Result<()> {
        org_mut(&mut self.staged.actors, self.id)
            .map_err(load_error)?
            .words
            .insert(o, v);
        Ok(())
    }
    fn byte(&mut self, o: u32) -> organization_state::Result<u8> {
        org(&self.staged.actors, self.id)
            .map_err(load_error)?
            .bytes
            .get(&o)
            .copied()
            .ok_or_else(|| load_error(format!("Organization+{o:X} byte absent")))
    }
    fn put_byte(&mut self, o: u32, v: u8) -> organization_state::Result<()> {
        org_mut(&mut self.staged.actors, self.id)
            .map_err(load_error)?
            .bytes
            .insert(o, v);
        Ok(())
    }
    fn state_group4(
        &mut self,
    ) -> organization_state::Result<Option<organization_state::StateWords>> {
        self.state_words().map_err(load_error)
    }
    fn layout_info(&mut self) -> organization_state::Result<organization_state::LayoutInfo> {
        aa_mapworld::restore::RestoreDefinitions::organization_layout_info(
            self.catalog,
            &org(&self.staged.actors, self.id)
                .map_err(load_error)?
                .layout
                .definition,
            self.binding.precision,
        )
        .map_err(load_error)
    }
    fn formation_facts(&mut self, f: u32) -> organization_state::Result<(u8, u32)> {
        aa_mapworld::restore::RestoreDefinitions::organization_formation_facts(
            self.catalog,
            f,
            self.binding.precision,
        )
        .map_err(load_error)
    }
    fn has_applied_property(
        &mut self,
        p: u32,
        source: ActorId,
    ) -> organization_state::Result<bool> {
        crate::stored_organization_members::has_applied(
            self.catalog,
            self.staged,
            self.key,
            p,
            source,
            self.pc,
        )
        .map_err(load_error)
    }
    fn remove_property(&mut self, p: u32, source: ActorId) -> organization_state::Result<()> {
        match self
            .staged
            .actors
            .component(self.id, Slot::Properties)
            .map_err(load_error)?
        {
            None => return Ok(()),
            Some(RuntimeComponent::Properties(v)) if v.owner.0 == self.id.raw() => {
                if v.effects.is_empty() {
                    return Ok(());
                }
            }
            _ => return Err(load_error("4CEF70 actual Properties payload/owner differs")),
        }
        let reference = aa_rules::native::layout_data::PropertyRef::from_handle(p);
        let world = self
            .staged
            .load_world
            .as_mut()
            .ok_or_else(|| load_error("581810 actual World variables absent"))?;
        let (_, mut actors) = self.staged.actors.split_minds();
        crate::stored_state_properties::remove_reference(
            self.catalog,
            &mut actors,
            &mut self.staged.definitions,
            &mut world.variables,
            self.id,
            reference,
            source,
            self.pc,
        )
        .map_err(load_error)
    }
    fn apply_property(
        &mut self,
        p: u32,
        kind: u32,
        source: ActorId,
    ) -> organization_state::Result<()> {
        crate::staged_property_admission::apply(
            self.staged,
            self.catalog,
            self.key,
            aa_rules::native::layout_data::PropertyRef::from_handle(p),
            kind,
            Some(source),
            self.pc,
        )
        .map(|_| ())
        .map_err(load_error)
    }
    fn formation_refresh(&mut self) -> organization_state::Result<()> {
        crate::stored_organization_members::refresh_formation(
            self.catalog,
            self.staged,
            self.key,
            self.pc,
        )
        .map_err(load_error)
    }
    fn owner_object_164(&mut self) -> organization_state::Result<Option<u32>> {
        let reference = *self
            .staged
            .actors
            .object(self.key)
            .map_err(load_error)?
            .actor
            .as_ref()
            .ok_or_else(|| load_error("4D1900 current Actor absent"))?
            .words
            .get(&0x164)
            .ok_or_else(|| load_error("4D1900 current Actor+164 absent"))?;
        Ok(self
            .staged
            .actors
            .key(ActorId::from_raw(reference))
            .map(|_| reference))
    }
    fn object_refresh(&mut self, _: u32) -> organization_state::Result<()> {
        //Actual4B2830 is a single RET; its receiver identity was just resolved.
        Ok(())
    }
}
impl aa_actors::component_virtuals::OrganizationExistingWorld for Existing<'_, '_> {
    fn member_count(&mut self) -> Result<u32> {
        u32::try_from(org(&self.staged.actors, self.id)?.members.len()).map_err(error)
    }
    fn set_definition_layout(&mut self) -> Result<()> {
        let default = self
            .binding
            .definition
            .ground
            .organization
            .as_ref()
            .ok_or_else(|| error("57C450 actual ActorData+2F0 default Layout absent"))?;
        let data = self
            .catalog
            .layout_data(self.binding.precision)
            .map_err(error)?;
        let layout = data
            .find_layout(&default.layout.definition)
            .ok_or_else(|| error("57C450 actual default LayoutData absent"))?;
        let info = data.layout_info(layout).map_err(error)?;
        if default.layout.slots != info.slot_groups.len()
            || default.layout.parameters.len() != info.entries.len()
        {
            return Err(error("57C450 source layout dimensions differ"));
        }
        //Portable source guard: verify the actual default's restrictions. The
        //native assert latch/diagnostic remains required if this check fails.
        for (entry, parameter) in info.entries.iter().zip(&default.layout.parameters) {
            if let Some(id) = parameter {
                if !data
                    .actor_has_property(
                        self.catalog.actors,
                        id,
                        entry.property,
                        self.binding.precision,
                    )
                    .map_err(error)?
                {
                    return Err(Error::Unsupported(
                        "57C450/4C2910 failed default restriction actual assert latch855D30",
                    ));
                }
            } else if entry.required {
                return Err(Error::Unsupported(
                    "57C450/4C2910 missing required default actual assert latch855D30",
                ));
            }
        }
        //4C2290 temporary-copy then4C21E0 actual member layout copy. Strings
        //and parameters are value copies at this native boundary, not actors.
        let count = info.slot_groups.len();
        organization_state::initialize_default_fields(
            org_mut(&mut self.staged.actors, self.id)?,
            &default.layout,
            count,
        );
        let world = self
            .staged
            .load_world
            .as_ref()
            .ok_or_else(|| error("4CF0F0 actual World absent"))?;
        let (_, mut actors) = self.staged.actors.split_minds();
        crate::stored_economy_refresh::refresh(
            self.catalog,
            &mut actors,
            &mut self.staged.definitions,
            world.live_spatial().map_err(error)?,
            &world.variables,
            self.id,
            self.pc,
        )
        .map_err(error)?;
        drop(actors);
        maximum_morale(
            self.catalog,
            self.staged,
            self.key,
            self.id,
            self.binding.precision,
            self.pc,
        )?;
        //45D8A0 zero-constructs newly allocated XY capacity, preserving prior
        //logical offsets when the resize reuses existing elements.
        org_mut(&mut self.staged.actors, self.id)?
            .offsets_78
            .resize(count, [F32::ZERO; 2]);
        let index = usize::try_from(info.formation_40).map_err(error)?;
        let formation = *info
            .formations
            .get(index)
            .ok_or_else(|| error("57C450 default formation index outside actual LayoutData+34"))?;
        self.set_formation(formation)
    }
    fn refresh_properties(&mut self) -> Result<()> {
        crate::stored_actor_properties::refresh(
            self.catalog,
            &mut self.staged.actors,
            &self.staged.definitions,
            self.key,
        )
        .map_err(error)
    }
    fn refresh_actions(&mut self) -> Result<()> {
        let world = self
            .staged
            .load_world
            .as_ref()
            .ok_or_else(|| error("57B390 actual World absent"))?;
        let (arena, mut actors) = self.staged.actors.split_minds();
        crate::stored_actor_actions::organization_on_owners(
            self.catalog,
            &mut actors,
            arena,
            &mut self.staged.definitions,
            self.key,
            &self.staged.inventory,
            world.live_spatial().map_err(error)?,
            Some(&world.variables),
            self.pc,
        )
        .map_err(error)
    }
    fn formation_flag(&mut self) -> Result<u8> {
        let f = *org(&self.staged.actors, self.id)?
            .words
            .get(&0x70)
            .ok_or_else(|| error("57C870 Organization+70 unwritten"))?;
        if f == 0 {
            return Ok(0);
        }
        aa_mapworld::restore::RestoreDefinitions::organization_formation_facts(
            self.catalog,
            f,
            self.binding.precision,
        )
        .map(|v| v.0)
        .map_err(error)
    }
    fn apply_formation_flag(&mut self, value: u8) -> Result<()> {
        self.apply_flag(value)
    }
    fn world_version(&mut self) -> Result<u32> {
        Ok(self.staged.report.world_version)
    }
    fn pending(&mut self) -> Result<bool> {
        crate::stored_component_schedule::World::new(
            self.staged,
            self.key,
            Slot::Organization,
            self.pc,
        )?
        .pending()
    }
    fn add_periodic(&mut self) -> Result<()> {
        crate::stored_component_schedule::World::new(
            self.staged,
            self.key,
            Slot::Organization,
            self.pc,
        )?
        .add_periodic(1)
    }
}
pub fn init_existing(
    catalog: &Catalog<'_>,
    staged: &mut Staged,
    key: ObjectKey,
    pc: Precision,
) -> Result<bool> {
    let (id, binding) = checked(catalog, staged, key)?;
    aa_actors::component_virtuals::organization_init_existing(&mut Existing {
        catalog,
        staged,
        key,
        id,
        binding: &binding,
        pc,
    })
}
pub fn after_init(
    catalog: &Catalog<'_>,
    staged: &mut Staged,
    key: ObjectKey,
    pc: Precision,
) -> Result<()> {
    let (id, binding) = checked(catalog, staged, key)?;
    let present = staged
        .actors
        .object(key)
        .map_err(error)?
        .actor
        .as_ref()
        .unwrap()
        .zones_148
        .zones()
        .slots
        .get(4)
        .and_then(Option::as_ref)
        .is_some();
    if !present {
        let radius = binding
            .source_ground
            .organization
            .as_ref()
            .ok_or_else(|| error("45D4C0 actual OrganizationData absent"))?
            .control_range
            .bits;
        org_mut(&mut staged.actors, id)?.words.insert(0xac, radius);
        let world = staged
            .load_world
            .as_ref()
            .ok_or_else(|| error("579DD0 actual World absent"))?;
        let pose = staged
            .actors
            .placement(key, world.live_spatial().map_err(error)?)
            .map_err(error)?;
        let world = staged
            .load_world
            .as_mut()
            .ok_or_else(|| error("579DD0 actual World absent"))?;
        let actor = staged
            .actors
            .object_mut(key)
            .map_err(error)?
            .actor
            .as_mut()
            .unwrap();
        if crate::load_detection_activation::create_slot_fields(
            id,
            pose.flags,
            actor.zones_148.zones_mut(),
            &mut world.zone_identities,
            4,
        )
        .map_err(error)?
        {
            let radius = org(&staged.actors, id)?.words[&0xac];
            let pose = staged
                .actors
                .placement(key, world.live_spatial().map_err(error)?)
                .map_err(error)?;
            crate::stored_zone_activation::operate_slot(
                catalog,
                &staged.definitions,
                &mut staged.actors,
                world,
                &staged.clock,
                key,
                4,
                Some(([pose.position_bits[0], pose.position_bits[1]], radius)),
                staged.inactive_load.as_ref(),
                pc,
            )
            .map_err(error)?;
        }
    }
    if let Some(member) = org(&staged.actors, id)?
        .members
        .iter()
        .find(|m| !m.is_null())
    {
        return Err(error(format!(
            "579DD0 actual member Element5709D0 attachment/terrain callback owner={} member={}",
            id.raw(),
            member.raw()
        )));
    }
    Ok(())
}
