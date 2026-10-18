//!522890 over the same ActorAccess/Arena and retained World/command owners.
//!Commands use the shared predefined constructor and actual4D9B60 delivery;
//!their constructor reference is released on both success and child failure.
use crate::{
    actor_world::{ActiveComponent, ActorAccess, RuntimeComponent, RuntimeObject},
    catalog::Catalog,
    definitions::Bank,
    load_world::{SpatialOwner, World},
    mind_world,
    player_order_frame::{Fields, Services},
    player_order_world::{self as orders, DefinitionBankLoan},
    scheduler_owners::Owners,
};
use aa_actors::{ActorId, components::ComponentSlot as Slot, ground_states::OrganizationState};
use aa_mapworld::WorldInventory;
use aa_mind::{
    Arena, MindId, cai_member_command as core,
    construct::{Installed, Kind},
};
use aa_numeric::Precision;
use aa_simulation::{NativeClock, NativeRng};
use aa_world::queues::Queues;
use std::cell::RefCell;
#[path = "stored_member_command_fields.rs"]
mod fields;

fn error(e: impl std::fmt::Display) -> aa_mind::Error {
    aa_mind::Error::World(e.to_string())
}
type Result<T> = aa_mind::Result<T>;

struct Bound<'a, 'store, 'owner, 'source> {
    catalog: &'a Catalog<'source>,
    bank: &'a DefinitionBankLoan<'owner>,
    actors: &'a mut ActorAccess<'store>,
    fields: &'a RefCell<Fields<'owner>>,
    runtime: &'a mut mind_world::Runtime,
    arena: &'a mut Arena,
    queues: &'a mut Queues,
    owners: &'a mut Owners,
    pc: Precision,
}
impl Bound<'_, '_, '_, '_> {
    fn actor(&self, actor: ActorId) -> Result<&RuntimeObject> {
        orders::object_with_bank(self.catalog, self.bank, self.actors, actor).map_err(error)
    }
    fn mind(&self, actor: ActorId) -> Result<&Installed> {
        let object = self.actor(actor)?;
        let Some(RuntimeComponent::Mind(mind)) =
            object.components().map_err(error)?.get(Slot::Mind)
        else {
            return Err(error("522890 current Actor has no typed Mind"));
        };
        let expected = crate::catalog::mind_kind(
            object
                .definition
                .component_mask
                .ok_or_else(|| error("522890 source component mask absent"))?,
        )
        .ok_or_else(|| error("522890 current source has no Mind factory"))?;
        if expected != mind.kind || self.arena.mind(mind.mind_id)?.actor != actor.raw() {
            return Err(error("522890 same Mind/source/Arena identity differs"));
        }
        Ok(mind)
    }
    fn owner(&self, mind: MindId) -> Result<ActorId> {
        let actor = ActorId::from_raw(self.arena.mind(mind)?.actor);
        let installed = self.mind(actor)?;
        if installed.mind_id != mind || installed.kind != Kind::Regiment {
            return Err(error("522890 current owner is not the same Regiment Mind"));
        }
        Ok(actor)
    }
    fn organization_state(&self, actor: ActorId) -> Result<&OrganizationState<String>> {
        self.actor(actor)?;
        match self
            .actors
            .component(actor, Slot::Organization)
            .map_err(error)?
        {
            Some(RuntimeComponent::Organization(org)) if org.header.owner == actor => Ok(org),
            _ => Err(error(
                "522890 current Organization allocation/owner differs",
            )),
        }
    }
    fn captured(&self, receipt: (ActorId, usize)) -> Result<&OrganizationState<String>> {
        let org = self.organization_state(receipt.0)?;
        //Identity-only receipt; never dereferenced or used to reconstruct state.
        if org as *const _ as usize != receipt.1 {
            return Err(error("522890 captured Organization allocation replaced"));
        }
        Ok(org)
    }
    fn position(&self, actor: ActorId) -> Result<[u32; 2]> {
        self.actor(actor)?;
        let key = self
            .actors
            .key(actor)
            .ok_or_else(|| error("522890 current Actor generation absent"))?;
        let fields = self.fields.try_borrow().map_err(error)?;
        let position = self
            .actors
            .placement(key, &*fields.spatial)
            .map_err(error)?
            .position_bits;
        Ok([position[0], position[1]])
    }
    fn issue(
        &mut self,
        actor: ActorId,
        wire: aa_orders::commands::CommandWire,
    ) -> Result<aa_orders::commands::CommandDelivery> {
        let command = crate::player_order_decree::allocate_decoded_in_bank(
            self.catalog,
            self.bank,
            self.runtime,
            &wire,
        )
        .map_err(error)?;
        let delivered = {
            let mut services = Services {
                catalog: self.catalog,
                bank: self.bank,
                fields: self.fields,
                pc: self.pc,
            };
            let mut world = mind_world::World {
                actors: self.actors.reborrow(),
                runtime: &mut *self.runtime,
                queues: &mut *self.queues,
                owners: &mut *self.owners,
                services: &mut services,
            };
            orders::deliver_in_bank(
                self.catalog,
                self.bank,
                &mut world,
                self.arena,
                command,
                Some(actor),
            )
            .map_err(error)
        };
        match (delivered, self.runtime.commands.release(command)) {
            (Ok(delivery), Ok(())) => Ok(delivery),
            (Err(error), Ok(())) => Err(error),
            (Ok(_), Err(error)) => Err(error),
            (Err(first), Err(release)) => Err(error(format!(
                "{first}; 4CA2D0 command reference remains owned: {release}"
            ))),
        }
    }
}
impl core::World for Bound<'_, '_, '_, '_> {
    type Organization = (ActorId, usize);
    type Tile = usize;
    type Zone = (ActorId, u32);
    fn organization(&mut self, mind: MindId) -> Result<Self::Organization> {
        let actor = self.owner(mind)?;
        Ok((actor, self.organization_state(actor)? as *const _ as usize))
    }
    fn leader_position(&mut self, captured: Self::Organization) -> Result<[u32; 2]> {
        let leader = *self
            .captured(captured)?
            .words
            .get(&0x40)
            .ok_or_else(|| error("522890 actual Organization+40 unwritten"))?;
        if leader == 0 {
            return Err(error("522890 native null leader dereference"));
        }
        self.position(ActorId::from_raw(leader))
    }
    fn relocate_current_organization(&mut self, mind: MindId, point: [u32; 2]) -> Result<()> {
        let owner = self.owner(mind)?;
        aa_actors::element_position::set_position(owner, point.map(aa_body::F32), self)
            .map_err(error)
    }
    fn formation_origin(&mut self, captured: Self::Organization) -> Result<[u32; 2]> {
        Ok(self.captured(captured)?.position_xy.map(|v| v.0))
    }
    fn member_slot(&mut self, member: u32) -> Result<i32> {
        let member = ActorId::from_raw(member);
        self.actor(member)?;
        match self
            .actors
            .component(member, Slot::Element)
            .map_err(error)?
        {
            Some(RuntimeComponent::Element(element)) if element.header.owner == member => {
                Ok(element.slot_index)
            }
            _ => Err(error(
                "522890 actual member Element allocation/owner differs",
            )),
        }
    }
    fn current_slot_offset(&mut self, mind: MindId, slot: i32) -> Result<[u32; 2]> {
        let index = usize::try_from(slot).map_err(error)?;
        self.organization_state(self.owner(mind)?)?
            .offsets_78
            .get(index)
            .map(|p| p.map(|v| v.0))
            .ok_or_else(|| error("522890 current Organization+78 slot is outside allocation"))
    }
    fn tile(&mut self, position: [u32; 2]) -> Result<Option<usize>> {
        let fields = self.fields.try_borrow().map_err(error)?;
        fields::tile(&fields.zones.geometry, position).map_err(error)
    }
    fn current_zone4(&mut self, mind: MindId) -> Result<Option<Self::Zone>> {
        let owner = self.owner(mind)?;
        let actor = self
            .actor(owner)?
            .actor
            .as_ref()
            .ok_or_else(|| error("522890 actual owner Actor absent"))?;
        Ok(actor
            .zones_148
            .zones()
            .slots
            .get(4)
            .and_then(Option::as_ref)
            .map(|z| (owner, z.key)))
    }
    fn contains_zone(&mut self, tile: usize, captured: Self::Zone) -> Result<bool> {
        let actor = self
            .actor(captured.0)?
            .actor
            .as_ref()
            .ok_or_else(|| error("522890 captured Zone4 Actor absent"))?;
        let zone = actor
            .zones_148
            .zones()
            .slots
            .get(4)
            .and_then(Option::as_ref)
            .filter(|z| z.key == captured.1)
            .ok_or_else(|| error("522890 captured Zone4 allocation replaced"))?;
        let fields = self.fields.try_borrow().map_err(error)?;
        let tile = fields.zones.tile(tile).map_err(error)?;
        aa_zones::query::tile_has_zone(&fields.zones.pool, tile.set, Some((zone.actor, zone.flags)))
            .map_err(error)
    }
    fn member_position(&mut self, member: u32) -> Result<[u32; 2]> {
        self.position(ActorId::from_raw(member))
    }
    fn member_active_4cede0(&mut self, member: u32) -> Result<bool> {
        let member = ActorId::from_raw(member);
        self.actor(member)?;
        member_active_on_owners(self.actors, member)
    }
    fn member_target_position(&mut self, member: u32) -> Result<[u32; 2]> {
        let mind = self.mind(ActorId::from_raw(member))?;
        fields::target(mind.kind, |offset| {
            mind.words
                .get(&offset)
                .copied()
                .ok_or_else(|| format!("522890 actual Mind+{offset:X} unwritten"))
        })
        .map_err(error)
    }
    fn current_sleep_point(&mut self, mind: MindId) -> Result<[u32; 2]> {
        let mind = self.mind(self.owner(mind)?)?;
        let word = |offset| {
            mind.words.get(&offset).copied().ok_or_else(|| {
                error(format!(
                    "522890 actual current Regiment Mind+{offset:X} unwritten"
                ))
            })
        };
        Ok([word(0x2b4)?, word(0x2b8)?])
    }
    fn issue_stop(&mut self, member: u32) -> Result<()> {
        self.issue(ActorId::from_raw(member), command_fields(6, None))
            .map(|_| ())
    }
    fn issue_move(&mut self, member: u32, destination: [u32; 2], sleep: [u32; 2]) -> Result<()> {
        self.issue(
            ActorId::from_raw(member),
            command_fields(5, Some((destination, sleep))),
        )
        .map(|_| ())
    }
}

struct Predicate<'a> {
    mind: &'a Installed,
    arena: &'a Arena,
}
impl crate::organization_mind_callbacks::Fields for Predicate<'_> {
    fn embedded_status(&mut self, offset: u32) -> std::result::Result<u32, String> {
        let id = *self
            .mind
            .state_bindings
            .get(&offset)
            .ok_or_else(|| format!("57C680 actual embedded state+{offset:X} absent"))?;
        let state = self.arena.state(id).map_err(|e| e.to_string())?;
        if state.parent != self.mind.mind_id {
            return Err("57C680 current embedded state parent differs".into());
        }
        Ok(state.status)
    }
    fn move_speed_changed(&mut self, _: u32, _: u32) -> std::result::Result<(), String> {
        Err("57C680 position predicate cannot service move-speed callback".into())
    }
    fn put_word(&mut self, _: u32, _: u32) -> std::result::Result<(), String> {
        Err("57C680 position predicate cannot mutate unrelated Mind words".into())
    }
}
fn position_error(e: impl std::fmt::Display) -> aa_actors::element_position::Error {
    aa_actors::element_position::Error::World(e.to_string())
}
impl aa_actors::element_position::World for Bound<'_, '_, '_, '_> {
    type Definition = String;
    fn world_mode(&self) -> aa_actors::element_position::Result<u32> {
        Err(position_error(
            "57C680 entry does not borrow native World mode; required if reused for57C800",
        ))
    }
    fn actor(
        &self,
        id: ActorId,
    ) -> aa_actors::element_position::Result<aa_actors::element_position::Actor> {
        let object = Bound::actor(self, id).map_err(position_error)?;
        let mind = match object.components().map_err(position_error)?.get(Slot::Mind) {
            None => None,
            Some(RuntimeComponent::Mind(_)) => Some(id.raw()),
            _ => return Err(position_error("57C680 current typed Mind slot differs")),
        };
        let key = self
            .actors
            .key(id)
            .ok_or_else(|| position_error("57C680 current owner generation absent"))?;
        let fields = self.fields.try_borrow().map_err(position_error)?;
        let placement = self
            .actors
            .placement(key, &*fields.spatial)
            .map_err(position_error)?;
        Ok(aa_actors::element_position::Actor {
            flags: placement.flags,
            position: [
                aa_body::F32(placement.position_bits[0]),
                aa_body::F32(placement.position_bits[1]),
            ],
            mind,
        })
    }
    fn organization(
        &self,
        id: ActorId,
    ) -> aa_actors::element_position::Result<&OrganizationState<String>> {
        self.organization_state(id).map_err(position_error)
    }
    fn organization_mut(
        &mut self,
        id: ActorId,
    ) -> aa_actors::element_position::Result<&mut OrganizationState<String>> {
        self.organization_state(id).map_err(position_error)?;
        match self
            .actors
            .component_mut(id, Slot::Organization)
            .map_err(position_error)?
        {
            Some(RuntimeComponent::Organization(org)) if org.header.owner == id => Ok(org),
            _ => Err(position_error(
                "57C680 current mutable Organization allocation/owner differs",
            )),
        }
    }
    fn selected_member(&self, id: ActorId) -> aa_actors::element_position::Result<Option<ActorId>> {
        let value = *self
            .organization_state(id)
            .map_err(position_error)?
            .words
            .get(&0x40)
            .ok_or_else(|| position_error("57C680 actual Organization+40 unwritten"))?;
        Ok((value != 0).then_some(ActorId::from_raw(value)))
    }
    fn mind_predicate(
        &mut self,
        mind: u32,
        slot: u32,
    ) -> aa_actors::element_position::Result<bool> {
        let mind = self.mind(ActorId::from_raw(mind)).map_err(position_error)?;
        crate::organization_mind_callbacks::position_active(
            mind.kind,
            slot,
            &mut Predicate {
                mind,
                arena: self.arena,
            },
        )
        .map_err(position_error)
    }
    fn relocate_owner(
        &mut self,
        _: ActorId,
        _: [aa_body::F32; 2],
    ) -> aa_actors::element_position::Result<()> {
        Err(position_error(
            "57C680 actual4EEB40 ground relocation children after goal stores",
        ))
    }
    fn interpolate(&mut self, _: ActorId) -> aa_actors::element_position::Result<bool> {
        Err(position_error(
            "57C680 actual57C080 interpolation after goal stores",
        ))
    }
    fn has_scheduled(
        &mut self,
        organization: ActorId,
        event: u32,
        data: u32,
    ) -> aa_actors::element_position::Result<bool> {
        let key = self
            .actors
            .key(organization)
            .ok_or_else(|| position_error("57C680 scheduler owner generation absent"))?;
        let owner = self
            .owners
            .get(crate::scheduler_owners::Target::Component {
                object: key,
                slot: Slot::Organization.index() as u8,
            })
            .map_err(position_error)?;
        let event = u8::try_from(event).map_err(position_error)?;
        Ok(self.queues.has_from_owner_head(owner, event, data))
    }
    fn schedule(&mut self, _: ActorId, _: u32) -> aa_actors::element_position::Result<()> {
        Err(position_error(
            "57C680 actual50AC00 Organization scheduler publication",
        ))
    }
}

///The ordinary current-member4CEDE0 leaf shared by522890 and5200B0.
///Caller owns the source guard; this reads the actual current typed pointers.
pub(crate) fn member_active_on_owners(actors: &ActorAccess<'_>, member: ActorId) -> Result<bool> {
    let key = actors
        .key(member)
        .ok_or_else(|| error("4CEDE0 current member generation absent"))?;
    let actor = actors.object(key).map_err(error)?;
    let components = actor.components().map_err(error)?;
    if components.present(Slot::Organization) {
        return Err(error("4CEDE0 actual positive Organization branch"));
    }
    if !components.present(Slot::Mover) {
        return Ok(false);
    }
    match components.get(Slot::Mover) {
        Some(RuntimeComponent::Mover(mover)) if mover.header.owner == member => {}
        _ => return Err(error("4CEDE0 actual member Mover allocation/owner differs")),
    }
    match actor.active_component {
        ActiveComponent::None => Ok(false),
        ActiveComponent::Slot(slot) => Ok(slot == Slot::Mover),
        ActiveComponent::UnresolvedNativeWord(_) => Err(error(
            "4CEDE0 actual active-component pointer identity remains unresolved",
        )),
    }
}
///Native4D9E20 zero fields plus5229FC/4CA760 Move writes, before same allocation.
fn command_fields(
    kind: u32,
    movement: Option<([u32; 2], [u32; 2])>,
) -> aa_orders::commands::CommandWire {
    let (flags, position, auxiliary) = match movement {
        None => (0x40, [0; 2], [0; 2]),
        Some((position, auxiliary)) => (0x5c, position, auxiliary),
    };
    aa_orders::commands::CommandWire {
        flags,
        command: Some(kind),
        word: 0,
        object: None,
        position,
        auxiliary,
        scalar: 0,
        parameter: None,
        parameters: None,
        extra: None,
        boolean: false,
        trailing: false,
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn run(
    catalog: &Catalog<'_>,
    actors: &mut ActorAccess<'_>,
    bank: &mut Bank,
    world: &mut World,
    mind: MindId,
    member: ActorId,
    flag: u8,
    arena: &mut Arena,
    queues: &mut Queues,
    owners: &mut Owners,
    random: &mut NativeRng,
    clock: &NativeClock,
    inventory: &WorldInventory,
    pc: Precision,
) -> Result<()> {
    let SpatialOwner::Adopted(adopted) = &mut world.spatial else {
        return Err(error("522890 actual live spatial owner is not adopted"));
    };
    if adopted.error.is_some() || !adopted.pending.is_empty() {
        return Err(error("522890 actual live spatial transfer is incomplete"));
    }
    run_on_owners(
        catalog,
        actors,
        bank,
        &mut adopted.world,
        &mut world.variables,
        &mut world.presentation,
        &world.zones,
        &mut world.mind_runtime,
        mind,
        member,
        flag,
        arena,
        queues,
        owners,
        random,
        clock,
        inventory,
        pc,
    )
}

///Same522890 and order services after the caller has already split the World.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_on_owners(
    catalog: &Catalog<'_>,
    actors: &mut ActorAccess<'_>,
    bank: &mut Bank,
    spatial: &mut crate::spatial_world::SpatialWorld,
    variables: &mut crate::load_view_variables::State,
    presentation: &mut crate::fx_manager::State<crate::fx_graph::Command>,
    zones: &aa_zones::tile::ZoneMap,
    runtime: &mut mind_world::Runtime,
    mind: MindId,
    member: ActorId,
    flag: u8,
    arena: &mut Arena,
    queues: &mut Queues,
    owners: &mut Owners,
    random: &mut NativeRng,
    clock: &NativeClock,
    inventory: &WorldInventory,
    pc: Precision,
) -> Result<()> {
    let bank = DefinitionBankLoan::new(bank);
    let fields = RefCell::new(Fields {
        spatial,
        variables,
        random,
        clock,
        inventory,
        presentation,
        zones,
        terrain: None,
        mission: None,
        phase: None,
    });
    core::run(
        &mut Bound {
            catalog,
            bank: &bank,
            actors,
            fields: &fields,
            runtime,
            arena,
            queues,
            owners,
            pc,
        },
        mind,
        member.raw(),
        flag,
        pc,
    )
}

///52C4DA's direct Stop uses the SAME constructor/delivery/release path.
///The returned byte is the actual4D9B60 handler result, including zero.
#[allow(clippy::too_many_arguments)]
pub(crate) fn stop_on_owners(
    catalog: &Catalog<'_>,
    actors: &mut ActorAccess<'_>,
    bank: &mut Bank,
    spatial: &mut crate::spatial_world::SpatialWorld,
    variables: &mut crate::load_view_variables::State,
    presentation: &mut crate::fx_manager::State<crate::fx_graph::Command>,
    zones: &aa_zones::tile::ZoneMap,
    runtime: &mut mind_world::Runtime,
    actor: ActorId,
    arena: &mut Arena,
    queues: &mut Queues,
    owners: &mut Owners,
    random: &mut NativeRng,
    clock: &NativeClock,
    inventory: &WorldInventory,
    pc: Precision,
) -> Result<aa_orders::commands::CommandDelivery> {
    let bank = DefinitionBankLoan::new(bank);
    let fields = RefCell::new(Fields {
        spatial,
        variables,
        random,
        clock,
        inventory,
        presentation,
        zones,
        terrain: None,
        mission: None,
        phase: None,
    });
    Bound {
        catalog,
        bank: &bank,
        actors,
        fields: &fields,
        runtime,
        arena,
        queues,
        owners,
        pc,
    }
    .issue(actor, command_fields(6, None))
}
