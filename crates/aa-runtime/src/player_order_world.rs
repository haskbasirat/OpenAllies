//! Native actor authorization and command delivery over the actual live owners.
//! Source selection is supplied by the real player's VWorldState. Commands
//! enter52C320 in the same Mind/command/queue arenas; positions are never moved
//! directly by this adapter.
use crate::{
    actor_world::{ActorAccess, RuntimeComponent, RuntimeObject, Spatial},
    catalog::Catalog,
    live_object_init::Source,
    mind_world::{self, PlainCommand},
};
use aa_actors::{ActorId, components::ComponentSlot};
use aa_mind::{Arena, MindId};
use aa_orders::commands::{self, CommandDelivery, TryCommandRouting};

#[path="player_order_bank.rs"]mod bank;
#[path = "player_order_actor_read.rs"]
mod actor_read;
pub use actor_read::{ActorRead, ReadBank};
#[path = "player_order_guess.rs"]
pub mod guess;
#[path = "player_order_command_sync.rs"]
pub mod command_sync;
pub use bank::DefinitionBankLoan;
pub(crate) use bank::Banks;
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("player order requires {0}")]
    Required(String),
    #[error(transparent)]
    Order(#[from] aa_orders::Error),
    #[error(transparent)]
    Mind(#[from] aa_mind::Error),
}
pub type Result<T> = std::result::Result<T, Error>;
fn required<T>(s: impl Into<String>) -> Result<T> {
    Err(Error::Required(s.into()))
}
fn check<T>(r: std::result::Result<T, impl std::fmt::Display>) -> Result<T> {
    r.map_err(|e| Error::Required(e.to_string()))
}
/// Actual40EAA0 and76AC90 are deliberately separate reached reads. A missing
/// registered application variable cannot become true from an inferred mode.
pub trait Globals {
    fn can_process_40eaa0(&mut self) -> Result<bool>;
    fn session_phase_76ac90(&mut self) -> Result<u32>;
    /// 4DA570 UVar "SwapRightClickWithAltRightClick": 4BE130 caches the
    /// B39938 kind1 record (static B3CB9C bit0), 4BE150 reads its byte on
    /// every call. The actual profile-backed UVars owner is not bound here.
    fn swap_right_click_with_alt_right_click_4be150(&mut self) -> Result<bool> {
        required("4BE130/4BE150 actual UVar SwapRightClickWithAltRightClick (B39938) owner")
    }
    /// 4DA570 UVar "HotkeyAltRightClickCommand": 4B2E60 kind4 record cached
    /// in B3CB94 (B3CB9C bit1), UTF-16 text at record+10, read per call.
    fn hotkey_alt_right_click_command_4b2e60(&mut self) -> Result<Vec<u16>> {
        required("4B2E60 actual UVar HotkeyAltRightClickCommand (B39938) owner")
    }
}
///Native76A4B0/76A5A0 current low16 registry lookup. Positive B4 dispatch is
///admitted only for the proved ordinary Actor/Kingdom allocation domain.
pub fn resolve_thing_reference(catalog:&Catalog<'_>,actors:&crate::actor_world::ActorStore,
    bank:&crate::definitions::Bank,check_kind:aa_orders::payload::ThingCheck,slot:u16)->Result<Option<u16>>{
    use aa_orders::payload::ThingCheck;
    if !matches!(check_kind,ThingCheck::Any|ThingCheck::B4|ThingCheck::B0){return required("actual reached Thing virtual type predicate")}
    let Some(entry)=actors.identities().iter().find(|entry|entry.id.slot()==slot)else{return Ok(None)};
    if check_kind==ThingCheck::Any{return Ok(Some(entry.id.slot()))}
    let object=check(actors.object(entry.value))?;
    if check_kind==ThingCheck::B0{
        //Actual Object/Actor/Kingdom/Area/Camera/Doodad vtable+B0=6A71D0.
        //The current canonical/storage guard is portability binding, not an
        //inference from an Object field or from the unrelated+B4 predicate.
        if !matches!(object.definition.native_class,2|3|4|6|7|9){return required("actual nonordinary Thing B0 dispatch")}
        check(Source::bind_in_bank(catalog,bank,&object.definition))?;
        let actor=matches!(object.definition.native_class,3|4);
        if !crate::load_spatial::same_proof(&object.thing.definition,&object.definition.proof)
            ||object.definition.native_class!=object.definition.proof.category.runtime_class
            ||entry.is_actor!=actor||object.object.is_none()
            ||if actor{object.kind!=aa_actors::loading::LoadKind::Actor||object.actor.is_none()}
                else{object.kind!=aa_actors::loading::LoadKind::Object||object.actor.is_some()}{
            return required("actual B0 source/registry/storage predicate agreement")
        }
        return Ok(Some(entry.id.slot()))
    }
    if !matches!(object.definition.native_class,3|4){return required("actual nonordinary Thing B4 virtual dispatch")}
    check(Source::bind_in_bank(catalog,bank,&object.definition))?;
    if !crate::load_spatial::same_proof(&object.thing.definition,&object.definition.proof)
        ||object.kind!=aa_actors::loading::LoadKind::Actor||object.actor.is_none()||!entry.is_actor{
        return required("actual Actor B4 source/registry predicate agreement")
    }
    Ok(Some(entry.id.slot()))
}
pub(crate) fn object_with_bank<'a, A: ActorRead + ?Sized>(
    catalog: &Catalog<'_>,
    banks: &dyn Banks,
    actors: &'a A,
    actor: ActorId,
) -> Result<&'a RuntimeObject> {
    let key = actors
        .key(actor)
        .ok_or_else(|| Error::Required("current actor generation".into()))?;
    let o = check(actors.object(key))?;
    if !matches!(o.definition.native_class, 3 | 4) {
        return required("actual ordinary Actor/Kingdom virtual dispatch");
    }
    let bank=banks.borrow_bank()?;
    check(Source::bind_in_bank(catalog, &bank, &o.definition))?;
    if !crate::load_spatial::same_proof(&o.thing.definition, &o.definition.proof) {
        return required("actual actor shell/source identity agreement");
    }
    Ok(o)
}
pub(crate) fn actor_word_with_bank<A: ActorRead + ?Sized>(
    catalog: &Catalog<'_>,
    banks: &dyn Banks,
    actors: &A,
    actor: ActorId,
    offset: u32,
) -> Result<u32> {
    object_with_bank(catalog, banks, actors, actor)?
        .actor
        .as_ref()
        .and_then(|a| a.words.get(&offset))
        .copied()
        .ok_or_else(|| Error::Required(format!("actual Actor+{offset:X} word")))
}
pub(crate) fn definition_with_bank(
    catalog: &Catalog<'_>,
    banks: &dyn Banks,
    command: &PlainCommand,
) -> Result<aa_rules::native::actor_command::ActorCommandDefinition> {
    let bank = banks.borrow_bank()?;
    let entry = check(bank.require_usable(command.definition_id))?;
    if entry.key.domain != "actorcommand" {
        return required("canonical ActorCommand definition domain");
    }
    let d = check(aa_rules::native::actor_command::predefined_actor_command(
        &entry.key.name,
    ))?;
    let projected = aa_mind::command::Definition {
        component_slot: d.component_slot,
        mind_type: d.mind_type,
        behavior_flags: d.behavior_flags,
    };
    if command.definition != Some(projected) {
        return required("canonical command type/component/behavior identity");
    }
    Ok(d)
}
///4D1F90/96D710 reads CL; shifts64..255 return zero,256 wraps to zero.
pub fn native_command_bit(bits: u64, index: u32) -> bool {
    let shift = index as u8;
    let mask = if shift < 64 { 1u64 << shift } else { 0 };
    bits & mask != 0
}
pub(crate) fn allows_with_bank<A: ActorRead + ?Sized>(
    catalog: &Catalog<'_>,
    banks: &dyn Banks,
    actors: &A,
    actor: ActorId,
    index: u32,
) -> Result<bool> {
    let low = actor_word_with_bank(catalog, banks, actors, actor, 0xf0)?;
    let high = actor_word_with_bank(catalog, banks, actors, actor, 0xf4)?;
    Ok(native_command_bit(
        u64::from(low) | (u64::from(high) << 32),
        index,
    ))
}
trait PermissionWorld {
    fn flags(&mut self, actor: ActorId) -> Result<u32>;
    fn parent(&mut self, actor: ActorId) -> Result<Option<ActorId>>;
    fn present(&mut self, actor: ActorId, slot: ComponentSlot) -> Result<bool>;
    fn gate(&mut self) -> Result<bool>;
    fn phase(&mut self) -> Result<u32>;
    fn owner(&mut self, actor: ActorId) -> Result<Option<ActorId>>;
    fn actions(&mut self, actor: ActorId) -> Result<u64>;
}
fn eligible_native(world: &mut impl PermissionWorld, actor: ActorId, argument: u8) -> Result<bool> {
    let mut actor = actor;
    let mut seen = Vec::new();
    loop {
        if seen.contains(&actor) {
            return required("finite4CF120 Element-owner chain (native would loop)");
        }
        seen.push(actor);
        if world.flags(actor)? & (1 << 2) != 0 {
            return Ok(false);
        }
        if let Some(parent) = world.parent(actor)? {
            actor = parent;
            continue;
        }
        if argument != 0 && world.gate()? && world.phase()? == 7 {
            return Ok(true);
        }
        return Ok(!world.present(actor, ComponentSlot::Mover)?
            || world.present(actor, ComponentSlot::MobileDeployment)?
            || world.present(actor, ComponentSlot::Recruit)?
            || world.present(actor, ComponentSlot::Denizen)?);
    }
}
fn permission_native(
    world: &mut impl PermissionWorld,
    actor: ActorId,
    kingdom: Option<ActorId>,
    resolved_type: Option<u32>,
    selected: u8,
) -> Result<bool> {
    let Some(index) = resolved_type else {
        return Ok(false);
    }; //4CF430 unresolved4DA570
    if selected != 0 {
        let owner = world.owner(actor)?;
        if owner.is_none() || owner != kingdom || !eligible_native(world, actor, 1)? {
            return Ok(false);
        }
    }
    Ok(native_command_bit(world.actions(actor)?, index))
}
struct PermissionBinding<'a, 'src, A: ?Sized, P, G> {
    catalog: &'a Catalog<'src>,
    banks: &'a dyn Banks,
    actors: &'a A,
    spatial: &'a P,
    globals: &'a mut G,
}
impl<A: ActorRead + ?Sized, P: Spatial, G: Globals> PermissionWorld for PermissionBinding<'_, '_, A, P, G> {
    fn flags(&mut self, actor: ActorId) -> Result<u32> {
        object_with_bank(self.catalog, self.banks, self.actors, actor)?;
        let key = self
            .actors
            .key(actor)
            .ok_or_else(|| Error::Required("current actor generation".into()))?;
        Ok(check(self.actors.placement(key, self.spatial))?.flags)
    }
    fn parent(&mut self, actor: ActorId) -> Result<Option<ActorId>> {
        object_with_bank(self.catalog, self.banks, self.actors, actor)?;
        match check(self.actors.component(actor, ComponentSlot::Element))? {
            None => Ok(None),
            Some(RuntimeComponent::Element(e)) if e.header.owner == actor => Ok(e.organization),
            Some(_) => required("actual Element slot payload/owner identity"),
        }
    }
    fn present(&mut self, actor: ActorId, slot: ComponentSlot) -> Result<bool> {
        Ok(object_with_bank(self.catalog, self.banks, self.actors, actor)?
            .components()
            .map_err(|e| Error::Required(e.to_string()))?
            .present(slot))
    }
    fn gate(&mut self) -> Result<bool> {
        self.globals.can_process_40eaa0()
    }
    fn phase(&mut self) -> Result<u32> {
        self.globals.session_phase_76ac90()
    }
    fn owner(&mut self, actor: ActorId) -> Result<Option<ActorId>> {
        let raw = actor_word_with_bank(self.catalog, self.banks, self.actors, actor, 0xe8)?;
        Ok((raw != 0).then_some(ActorId::from_raw(raw)))
    }
    fn actions(&mut self, actor: ActorId) -> Result<u64> {
        Ok(
            u64::from(actor_word_with_bank(self.catalog, self.banks, self.actors, actor, 0xf0)?)
                | (u64::from(actor_word_with_bank(self.catalog, self.banks, self.actors, actor, 0xf4)?) << 32),
        )
    }
}
/// Actual4CF120 ordinary virtual+20 over authoritative live fields. Element+10
/// tail-calls preserve the argument and actual source identities.
pub(crate) fn eligible_with_bank<A: ActorRead + ?Sized>(
    catalog: &Catalog<'_>,
    banks: &dyn Banks,
    actors: &A,
    spatial: &impl Spatial,
    actor: ActorId,
    argument: u8,
    globals: &mut impl Globals,
) -> Result<bool> {
    eligible_native(
        &mut PermissionBinding {
            catalog,
            banks,
            actors,
            spatial,
            globals,
        },
        actor,
        argument,
    )
}
/// Concrete actor virtual+150 = 4CF430(kingdom, &cmd, selected): an
/// explicit command passes def+0C, otherwise 4DA570(cmd, actor, false)
/// chooses it (null -> false) and may complete the command; then 4CE680.
#[allow(clippy::too_many_arguments)]
fn authorize_4cf430<A: ActorRead + ?Sized>(
    catalog: &Catalog<'_>,
    banks: &dyn Banks,
    actors: &A,
    spatial: &impl Spatial,
    actor: ActorId,
    kingdom: Option<ActorId>,
    command: &mut PlainCommand,
    selected: u8,
    globals: &mut impl Globals,
) -> Result<bool> {
    let resolved = if command.flags & (1 << 6) != 0 {
        Some(definition_with_bank(catalog, banks, command)?.mind_type)
    } else {
        guess::guess_plain_with_bank(catalog, banks, actors, globals, command, actor, false)?
            .map(|g| g.definition.type_0c)
    };
    permission_native(
        &mut PermissionBinding {
            catalog,
            banks,
            actors,
            spatial,
            globals,
        },
        actor,
        kingdom,
        resolved,
        selected,
    )
}
/// 4CF430 on a caller-borrowed command: a 4DA994 completion would need the
/// caller's mutable command, so it is refused instead of being dropped.
#[allow(clippy::too_many_arguments)]
fn authorize_borrowed(
    catalog: &Catalog<'_>,
    banks: &dyn Banks,
    actors: &ActorAccess<'_>,
    spatial: &impl Spatial,
    actor: ActorId,
    kingdom: Option<ActorId>,
    command: &PlainCommand,
    selected: u8,
    globals: &mut impl Globals,
) -> Result<bool> {
    let mut probe = command.clone();
    let allowed = authorize_4cf430(catalog, banks, actors, spatial, actor, kingdom, &mut probe, selected, globals)?;
    if probe != *command {
        return required("4CF430 caller-owned mutable command for the 4DA570 completion");
    }
    Ok(allowed)
}
/// 4DA570 on the one live arena command; its 4DA994 stores persist there,
/// also when a later leaf fails.
#[allow(clippy::too_many_arguments)]
pub(crate) fn guess_arena_with_bank(
    catalog: &Catalog<'_>,
    banks: &dyn Banks,
    actors: &ActorAccess<'_>,
    commands: &mut mind_world::Commands,
    command: u32,
    actor: ActorId,
    non_group_excluded: bool,
    globals: &mut impl Globals,
) -> Result<Option<guess::Guessed>> {
    let mut value = check(commands.value(command))?.clone();
    let result = guess::guess_plain_with_bank(catalog, banks, actors, globals, &mut value, actor, non_group_excluded);
    *check(commands.value_mut(command))? = value;
    result
}
/// 61E220 actor vt+150(NULL, &cmd, false) on the live arena command.
#[allow(clippy::too_many_arguments)]
pub(crate) fn authorize_actor_arena_with_bank(
    catalog: &Catalog<'_>,
    banks: &dyn Banks,
    actors: &ActorAccess<'_>,
    spatial: &impl Spatial,
    commands: &mut mind_world::Commands,
    command: u32,
    actor: ActorId,
    globals: &mut impl Globals,
) -> Result<bool> {
    let mut value = check(commands.value(command))?.clone();
    let result = authorize_4cf430(catalog, banks, actors, spatial, actor, None, &mut value, 0, globals);
    *check(commands.value_mut(command))? = value;
    result
}
/// 4D9A90(def): command+08 = def and flag bit6 set; null clears both.
fn store_definition_4d9a90(
    banks: &dyn Banks,
    commands: &mut mind_world::Commands,
    command: u32,
    index: Option<u32>,
) -> Result<()> {
    let definition = index
        .map(|i| {
            aa_rules::native::actor_command::PREDEFINED_ACTOR_COMMANDS
                .get(i as usize)
                .copied()
                .ok_or_else(|| Error::Required("canonical ActorCommand ordinal".into()))
        })
        .transpose()?;
    let token = match definition {
        Some(d) => banks.borrow_bank_mut()?.intern("ActorCommand", d.id).map_err(Error::Required)?,
        None => 0,
    };
    let value = check(commands.value_mut(command))?;
    value.definition_id = token;
    value.definition = definition.map(|d| aa_mind::command::Definition {
        component_slot: d.component_slot,
        mind_type: d.mind_type,
        behavior_flags: d.behavior_flags,
    });
    if definition.is_some() {
        value.flags |= 0x40;
    } else {
        value.flags &= !0x40;
    }
    Ok(())
}
/// Concrete selected virtual+150 ->4CF430/4CE680 with selected=true.
/// Explicit commands bind canonical definitions; a default command is
/// resolved by 4DA570 (the borrowed command cannot take its completion).
#[allow(clippy::too_many_arguments)]
pub(crate) fn authorize_selected_with_bank(
    catalog: &Catalog<'_>,
    banks: &dyn Banks,
    actors: &ActorAccess<'_>,
    spatial: &impl Spatial,
    actor: ActorId,
    kingdom: Option<ActorId>,
    command: &PlainCommand,
    globals: &mut impl Globals,
) -> Result<bool> {
    authorize_borrowed(catalog, banks, actors, spatial, actor, kingdom, command, 1, globals)
}
/// 4CF430(kingdom, &cmd, true) between frames over the retained store and
/// a read-only Bank: UIWorldState 0x436950's per-actor vt+0x150 on the UI
/// side. The command is the UI's own (0x4DA570 may complete it).
#[allow(clippy::too_many_arguments)]
pub fn authorize_selected_command_read(
    catalog: &Catalog<'_>,
    bank: &ReadBank<'_>,
    actors: &crate::actor_world::ActorStore,
    spatial: &impl Spatial,
    actor: ActorId,
    kingdom: Option<ActorId>,
    command: &mut PlainCommand,
    globals: &mut impl Globals,
) -> Result<bool> {
    authorize_4cf430(catalog, bank, actors, spatial, actor, kingdom, command, 1, globals)
}
/// Public 4CF430(kingdom, &cmd, true) over a caller-owned command, e.g. the
/// UIWorldState 436950 CanSend path, in the actual moved Bank.
#[allow(clippy::too_many_arguments)]
pub fn authorize_selected_command_in_bank(
    catalog: &Catalog<'_>,
    bank: &DefinitionBankLoan<'_>,
    actors: &ActorAccess<'_>,
    spatial: &impl Spatial,
    actor: ActorId,
    kingdom: Option<ActorId>,
    command: &mut PlainCommand,
    globals: &mut impl Globals,
) -> Result<bool> {
    authorize_4cf430(catalog, bank, actors, spatial, actor, kingdom, command, 1, globals)
}
///61E220's actual virtual+150 arguments: null player and selected=false.
///4CF430 still resolves the command (4DA570 for a default one) and calls
///4CE680; the false-selection branch skips player ownership/eligibility.
pub(crate) fn authorize_actor_with_bank(
    catalog: &Catalog<'_>,
    banks: &dyn Banks,
    actors: &ActorAccess<'_>,
    spatial: &impl Spatial,
    actor: ActorId,
    command: &PlainCommand,
    globals: &mut impl Globals,
) -> Result<bool> {
    authorize_borrowed(catalog, banks, actors, spatial, actor, None, command, 0, globals)
}
/// Original4D9E20/4CA760 fields. Inputs are raw caller words; no target/path,
/// direction, speed or Shift policy is invented by this constructor adapter.
pub enum Input {
    Move {
        destination: [u32; 2],
        auxiliary: [u32; 2],
        speed: u32,
        trailing: u8,
    },
    Attack {
        target: ActorId,
        trailing: u8,
    },
}
fn command_fields(
    d: aa_rules::native::actor_command::ActorCommandDefinition,
    token: u32,
    input: Input,
) -> PlainCommand {
    let (flags, word10, position, auxiliary, scalar, trailing) = match input {
        Input::Move {
            destination,
            auxiliary,
            speed,
            trailing,
        } => (0x5c, 0, destination, auxiliary, speed, trailing),
        Input::Attack { target, trailing } => (
            0x40 | u32::from(!target.is_null()),
            target.raw(),
            [0; 2],
            [0; 2],
            0,
            trailing,
        ),
    };
    PlainCommand {
        definition_id: token,
        definition: Some(aa_mind::command::Definition {
            component_slot: d.component_slot,
            mind_type: d.mind_type,
            behavior_flags: d.behavior_flags,
        }),
        flags,
        word10,
        object_definition: None,
        position_bits: position,
        auxiliary_bits: auxiliary,
        scalar_bits: scalar,
        parameter_definition: None,
        extra_definition: None,
        boolean: 0,
        trailing,
    }
}
/// Low-level native constructor fields. Resolved target identities are a
/// caller precondition here; issue_selected validates its actual target owner.
pub(crate) fn allocate_with_bank(
    catalog: &Catalog<'_>,
    banks: &dyn Banks,
    runtime: &mut mind_world::Runtime,
    input: Input,
) -> Result<u32> {
    check(catalog.require_source_owner())?;
    let name = match &input {
        Input::Move { .. } => "move",
        Input::Attack { .. } => "kill",
    };
    let d = check(aa_rules::native::actor_command::predefined_actor_command(
        name,
    ))?;
    let token = banks.borrow_bank_mut()?
        .intern("ActorCommand", d.id)
        .map_err(Error::Required)?;
    check(runtime.commands.allocate(command_fields(d, token, input)))
}
struct Routing<'a, 'w, 'store, 'src, S> {
    catalog: &'a Catalog<'src>,
    banks: &'a dyn Banks,
    world: &'a mut mind_world::World<'w, 'store, S>,
    arena: &'a mut Arena,
    command: u32,
}
impl<S: mind_world::Services> TryCommandRouting for Routing<'_, '_, '_, '_, S> {
    type Actor = ActorId;
    type Handler = (ActorId, ComponentSlot);
    type Error = Error;
    fn handler(&mut self, actor: ActorId, slot: u32) -> Result<Option<Self::Handler>> {
        let o = object_with_bank(self.catalog, self.banks, &self.world.actors, actor)?;
        let slot = ComponentSlot::from_index(slot as usize)
            .ok_or_else(|| Error::Required("native valid component dispatch slot".into()))?;
        Ok(o.components()
            .map_err(|e| Error::Required(e.to_string()))?
            .present(slot)
            .then_some((actor, slot)))
    }
    fn accepts(&mut self, actor: ActorId) -> Result<bool> {
        let command = check(self.world.runtime.commands.value(self.command))?;
        let d = definition_with_bank(self.catalog, self.banks, command)?;
        allows_with_bank(self.catalog, self.banks, &self.world.actors, actor, d.mind_type)
    }
    fn handle(&mut self, (actor, slot): Self::Handler) -> Result<u8> {
        if slot == ComponentSlot::Recruit {
            // 4D9BE5: [actor+60+10*4] KKC_RecruitComponent vt+24 = 583A70
            // (this = component, arg = command): 77DC50, 4D9A40("_cmd",
            // command) (command_sync::sync_command_4d9a40 with CMD_LABEL), the
            // command+34 ParameterSet, then type 0x0F -> 5838B0(this,
            // command+14 ActorData, ParameterSet). Owned by live_recruit.
            return required("583A70 KKC_RecruitComponent command virtual+24: live_recruit handler (actor, Recruit component, command id with +14 object definition, SyncA binding) is not bound");
        }
        if slot != ComponentSlot::Mind {
            return required(format!("actual component{slot:?} command virtual+24"));
        }
        let Some(RuntimeComponent::Mind(installed)) =
            check(self.world.actors.component(actor, slot))?
        else {
            return required("actual Mind slot payload");
        };
        let mind: MindId = installed.mind_id;
        if check(self.arena.mind(mind))?.actor != actor.raw() {
            return required("same current Mind/Actor owner");
        }
        Ok(u8::from(self.world.process_command(
            mind,
            self.command,
            self.arena,
        )?))
    }
    fn redirect(&mut self, actor: ActorId) -> Result<Option<Option<ActorId>>> {
        object_with_bank(self.catalog, self.banks, &self.world.actors, actor)?;
        match check(self.world.actors.component(actor, ComponentSlot::Structure))? {
            None => Ok(None),
            Some(RuntimeComponent::Structure(s))
                if s.header.owner == actor && s.object_handle_10 == 0 =>
            {
                Ok(None)
            }
            Some(RuntimeComponent::Structure(_)) => {
                required("4CE980/588990 actual parent Settlement+14 attachment owner")
            }
            Some(_) => required("actual Structure slot payload"),
        }
    }
    fn container_member(&mut self, actor: ActorId) -> Result<Option<ActorId>> {
        object_with_bank(self.catalog, self.banks, &self.world.actors, actor)?;
        match check(
            self.world
                .actors
                .component(actor, ComponentSlot::Organization),
        )? {
            None => Ok(None),
            Some(RuntimeComponent::Organization(o)) if o.header.owner == actor => {
                let raw = *o
                    .words
                    .get(&0x44)
                    .ok_or_else(|| Error::Required("actual Organization+44 member".into()))?;
                Ok((raw != 0).then_some(ActorId::from_raw(raw)))
            }
            Some(_) => required("actual Organization slot payload/owner"),
        }
    }
}
///4D9B60 itself does not authorize player ownership. Call after the native
/// selected/actor decree boundary; its returned byte is diagnostic to4ED360.
pub(crate) fn deliver_with_bank<S: mind_world::Services>(
    catalog: &Catalog<'_>,
    banks: &dyn Banks,
    world: &mut mind_world::World<'_, '_, S>,
    arena: &mut Arena,
    command: u32,
    actor: Option<ActorId>,
) -> Result<CommandDelivery> {
    // Native null-actor branch never dereferences the command definition.
    if actor.is_none() {
        return Ok(CommandDelivery {
            result: 0,
            null_actor_logged: true,
        });
    }
    let d = definition_with_bank(catalog, banks, check(world.runtime.commands.value(command))?)?;
    commands::try_deliver_actor_command(
        &mut Routing {
            catalog,
            banks,
            world,
            arena,
            command,
        },
        d.component_slot,
        actor,
    )
}
pub use crate::player_selection::Outcome as SelectionOutcome;
pub(crate) fn selection_byte_with_bank(
    catalog: &Catalog<'_>,
    banks: &dyn Banks,
    actors: &mut ActorAccess<'_>,
    actor: ActorId,
    increment: bool,
) -> Result<()> {
    object_with_bank(catalog, banks, actors, actor)?;
    let key = actors
        .key(actor)
        .ok_or_else(|| Error::Required("selected actor generation".into()))?;
    let o = check(actors.object_mut(key))?
        .object
        .as_mut()
        .ok_or_else(|| Error::Required("actual Object4C byte".into()))?;
    o.byte_4c = if increment {
        o.byte_4c.wrapping_add(1)
    } else {
        o.byte_4c.wrapping_sub(1)
    };
    Ok(())
}
fn update_group(view: &mut impl crate::player_selection::View) -> Result<()> {
    let list = view.list();
    let single = match (list.head(), list.tail()) {
        (None, None) => true,
        (Some(h), Some(t)) => check(list.node(h))?.actor == check(list.node(t))?.actor,
        _ => return required("consistent actual selection head/tail owner"),
    };
    if single {
        view.clear_group();
        Ok(())
    } else {
        required(
            "4ECF00 same-owner wildcard/static-group storage at the first positive group store",
        )
    }
}
pub(crate) fn rebuild_single_masks_with_bank(
    catalog: &Catalog<'_>,
    banks: &dyn Banks,
    actors: &ActorAccess<'_>,
    view: &mut impl crate::player_selection::View,
) -> Result<()> {
    // Four immediate zero stores of4ECD70 precede its first actor callback.
    view.mask_words_mut().fill(0);
    let Some(head) = view.list().head() else {
        return Ok(());
    };
    let tail = view
        .list()
        .tail()
        .ok_or_else(|| Error::Required("actual selection tail".into()))?;
    if check(view.list().node(head))?.actor != check(view.list().node(tail))?.actor {
        return required("4ECD70 actual ActorCommandData+3C/group owners for multiple selection");
    }
    // The real singleton branch bypasses ActorCommandData+3C. Both admitted
    // class3/4 vtables dispatch+40 to887B50 (false, ret8), never F0/F4 here.
    for index in 0..39 {
        let mask = 1u64 << index;
        let low = mask as u32;
        let high = (mask >> 32) as u32;
        view.mask_words_mut()[2] |= low;
        view.mask_words_mut()[3] |= high;
        let mut node = view.list().head();
        while let Some(id) = node {
            let entry = check(view.list().node(id))?;
            object_with_bank(catalog, banks, actors, entry.actor)?;
            view.mask_words_mut()[2] &= !low;
            view.mask_words_mut()[3] &= !high;
            node = entry.next;
        }
    }
    Ok(())
}
struct SelectionBinding<'a, 'store, 'src> {
    catalog: &'a Catalog<'src>,
    banks: &'a dyn Banks,
    actors: &'a mut ActorAccess<'store>,
    session: Option<&'a dyn crate::load_world_globals::SessionPhase>,
}
impl crate::player_selection::World for SelectionBinding<'_, '_, '_> {
    fn application_state(&mut self) -> std::result::Result<u32, String> {
        self.session
            .ok_or("actual SessionE4 owner")?
            .application_state()
    }
    fn is_actor(&mut self, actor: ActorId) -> std::result::Result<bool, String> {
        object_with_bank(self.catalog, self.banks, self.actors, actor)
            .map(|_| true)
            .map_err(|e| e.to_string())
    }
    fn element_parent(&mut self, actor: ActorId) -> std::result::Result<Option<ActorId>, String> {
        match self
            .actors
            .component(actor, ComponentSlot::Element)
            .map_err(|e| e.to_string())?
        {
            None => Ok(None),
            Some(RuntimeComponent::Element(e)) if e.header.owner == actor => Ok(e.organization),
            Some(_) => Err("actual Element slot/owner".into()),
        }
    }
    fn change_byte_4c(
        &mut self,
        actor: ActorId,
        increment: bool,
    ) -> std::result::Result<(), String> {
        selection_byte_with_bank(self.catalog, self.banks, self.actors, actor, increment).map_err(|e| e.to_string())
    }
    fn update_group(
        &mut self,
        view: &mut impl crate::player_selection::View,
    ) -> std::result::Result<(), String> {
        update_group(view).map_err(|e| e.to_string())
    }
    fn rebuild_masks(
        &mut self,
        view: &mut impl crate::player_selection::View,
    ) -> std::result::Result<(), String> {
        rebuild_single_masks_with_bank(self.catalog, self.banks, self.actors, view).map_err(|e| e.to_string())
    }
}
/// Concrete4ECFF0 over actual ordinary Actor fields and the same scoped view.
/// Positive group/multiple services remain required at their reached owner;
/// this is no fixed selected-unit cap and earlier stores remain committed.
pub(crate) fn select_with_bank(
    catalog: &Catalog<'_>,
    banks: &dyn Banks,
    actors: &mut ActorAccess<'_>,
    view: &mut impl crate::player_selection::View,
    session: &dyn crate::load_world_globals::SessionPhase,
    actor: Option<ActorId>,
) -> Result<SelectionOutcome> {
    check(crate::player_selection::select(
        &mut SelectionBinding {
            catalog,
            banks,
            actors,
            session: Some(session),
        },
        view,
        actor,
    ))
}
pub(crate) fn deselect_with_bank(
    catalog: &Catalog<'_>,
    banks: &dyn Banks,
    actors: &mut ActorAccess<'_>,
    view: &mut impl crate::player_selection::View,
    actor: ActorId,
) -> Result<bool> {
    check(crate::player_selection::deselect(
        &mut SelectionBinding {
            catalog,
            banks,
            actors,
            session: None,
        },
        view,
        actor,
    ))
}
pub(crate) fn clear_selection_with_bank(
    catalog: &Catalog<'_>,
    banks: &dyn Banks,
    actors: &mut ActorAccess<'_>,
    view: &mut impl crate::player_selection::View,
) -> Result<()> {
    check(crate::player_selection::clear(
        &mut SelectionBinding {
            catalog,
            banks,
            actors,
            session: None,
        },
        view,
    ))
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectedDelivery {
    Empty,
    Delivered,
}
/// Null-group4ED360 branch after its diagnostic4D9A40.
/// The real current order-node284 must match this actual scoped player's view.
/// A default command is guessed per actor (4DA570 with argument2 = distinct
/// head/tail actors), stored by 4D9A90 for 4CF430/4D9B60 and cleared after.
/// Positive group formation and live-group preparation remain required.
/// Return means delivery was attempted; native ignores acceptance.
pub(crate) fn deliver_selected_with_bank<S: mind_world::Services>(
    catalog: &Catalog<'_>,
    banks: &dyn Banks,
    world: &mut mind_world::World<'_, '_, S>,
    arena: &mut Arena,
    spatial: &impl Spatial,
    globals: &mut impl Globals,
    ordering_player_284: &u32,
    view: &mut impl crate::player_selection::View,
    command: u32,
) -> Result<SelectedDelivery> {
    if *ordering_player_284 == 0 || *ordering_player_284 != view.player_id() {
        return required("actual current admin284 player and same scoped selection owner");
    }
    let cmd = check(world.runtime.commands.value(command))?;
    // 4ED37C..4ED3A3: [esp+13] = default command (flag bit6 clear).
    let guessing = cmd.flags & (1 << 6) == 0;
    if !guessing {
        // 4ED3AD: ebp = [cmd+8]. With a NULL group (+20) nothing reads its
        // type: all 39 explicit types take the same per-actor path
        // (research/phase8-orders-input/recruit/native-selected-delivery-4ed360.json).
        definition_with_bank(catalog, banks, cmd)?;
    }
    if !check(view.group_is_null())? {
        return required("4ED360 actual positive selection-group/formation services");
    }
    let mut node = view.list().head();
    if node.is_none() {
        return Ok(SelectedDelivery::Empty);
    }
    while let Some(id) = node {
        // Native captures the next node before virtual+B4/authorization/delivery.
        let entry = check(view.list().node(id))?;
        node = entry.next;
        object_with_bank(catalog, banks, &world.actors, entry.actor)?;
        if guessing {
            // 4ED551..4ED56A: argument2 = head and tail actors differ.
            let list = view.list();
            let multiple = match (list.head(), list.tail()) {
                (None, _) => false,
                (Some(h), Some(t)) => check(list.node(h))?.actor != check(list.node(t))?.actor,
                (Some(_), None) => return required("4ED558 null selection tail with a head"),
            };
            let Some(g) = guess_arena_with_bank(catalog, banks, &world.actors, &mut world.runtime.commands,
                command, entry.actor, multiple, globals)? else {
                continue; // 4ED575: null skips this actor and the 4D9A90(0) reset.
            };
            store_definition_4d9a90(banks, &mut world.runtime.commands, command, Some(g.index))?;
        }
        let cmd = check(world.runtime.commands.value(command))?;
        if authorize_selected_with_bank(catalog, banks,
            &world.actors,
            spatial,
            entry.actor,
            view.kingdom(),
            cmd,
            globals,
        )? {
            let group = ActorId::from_raw(actor_word_with_bank(catalog, banks, &world.actors, entry.actor, 0x164)?);
            if world.actors.key(group).is_some() {
                return required("4D1900/4DFF30 actual live command-group preparation");
            }
            let _ = deliver_with_bank(catalog, banks, world, arena, command, Some(entry.actor))?;
        }
        if guessing {
            store_definition_4d9a90(banks, &mut world.runtime.commands, command, None)?;
        }
    }
    Ok(SelectedDelivery::Delivered)
}
/// Own the one original temporary command reference through synchronous selected
/// delivery, then release it as4CA2D0 does, including on a required-child error.
#[allow(clippy::too_many_arguments)]
pub(crate) fn issue_selected_with_bank<S: mind_world::Services>(
    catalog: &Catalog<'_>,
    banks: &dyn Banks,
    world: &mut mind_world::World<'_, '_, S>,
    arena: &mut Arena,
    spatial: &impl Spatial,
    globals: &mut impl Globals,
    ordering_player_284: &u32,
    view: &mut impl crate::player_selection::View,
    input: Input,
) -> Result<SelectedDelivery> {
    if let Input::Attack { target, .. } = &input
        && !target.is_null()
    {
        object_with_bank(catalog, banks, &world.actors, *target)?;
    }
    let command = allocate_with_bank(catalog, banks, world.runtime, input)?;
    let result = deliver_selected_with_bank(catalog, banks,
        world,
        arena,
        spatial,
        globals,
        ordering_player_284,
        view,
        command,
    );
    let released = world.runtime.commands.release(command);
    match (result, released) {
        (Ok(v), Ok(())) => Ok(v),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(Error::Mind(e)),
        (Err(e), Err(release)) => required(format!(
            "{e}; original command reference remains owned: {release}"
        )),
    }
}


//Compatibility APIs retain original lazy Catalog loans; actual Frame callers
//use the explicit same-Bank counterparts below.
pub(crate) fn object<'a>(
    catalog: &Catalog<'_>,
    actors: &'a ActorAccess<'_>,
    actor: ActorId,
) -> Result<&'a RuntimeObject> {
    object_with_bank(catalog, &catalog.definition_bank, actors, actor)
}

pub(crate) fn actor_word(
    catalog: &Catalog<'_>,
    actors: &ActorAccess<'_>,
    actor: ActorId,
    offset: u32,
) -> Result<u32> {
    actor_word_with_bank(catalog, &catalog.definition_bank, actors, actor, offset)
}

pub(crate) fn definition(
    catalog: &Catalog<'_>,
    command: &PlainCommand,
) -> Result<aa_rules::native::actor_command::ActorCommandDefinition> {
    definition_with_bank(catalog, &catalog.definition_bank, command)
}

pub fn eligible(
    catalog: &Catalog<'_>,
    actors: &ActorAccess<'_>,
    spatial: &impl Spatial,
    actor: ActorId,
    argument: u8,
    globals: &mut impl Globals,
) -> Result<bool> {
    eligible_with_bank(catalog, &catalog.definition_bank, actors, spatial, actor, argument, globals)
}

///Same algorithm over the actual moved Bank loan; no Catalog fallback.
pub fn eligible_in_bank(
    catalog: &Catalog<'_>,
    bank: &DefinitionBankLoan<'_>,
    actors: &ActorAccess<'_>,
    spatial: &impl Spatial,
    actor: ActorId,
    argument: u8,
    globals: &mut impl Globals,
) -> Result<bool> {
    eligible_with_bank(catalog, bank, actors, spatial, actor, argument, globals)
}

pub fn authorize_selected(
    catalog: &Catalog<'_>,
    actors: &ActorAccess<'_>,
    spatial: &impl Spatial,
    actor: ActorId,
    kingdom: Option<ActorId>,
    command: &PlainCommand,
    globals: &mut impl Globals,
) -> Result<bool> {
    authorize_selected_with_bank(catalog, &catalog.definition_bank, actors, spatial, actor, kingdom, command, globals)
}

///Same algorithm over the actual moved Bank loan; no Catalog fallback.
pub fn authorize_selected_in_bank(
    catalog: &Catalog<'_>,
    bank: &DefinitionBankLoan<'_>,
    actors: &ActorAccess<'_>,
    spatial: &impl Spatial,
    actor: ActorId,
    kingdom: Option<ActorId>,
    command: &PlainCommand,
    globals: &mut impl Globals,
) -> Result<bool> {
    authorize_selected_with_bank(catalog, bank, actors, spatial, actor, kingdom, command, globals)
}

pub fn authorize_actor(
    catalog: &Catalog<'_>,
    actors: &ActorAccess<'_>,
    spatial: &impl Spatial,
    actor: ActorId,
    command: &PlainCommand,
    globals: &mut impl Globals,
) -> Result<bool> {
    authorize_actor_with_bank(catalog, &catalog.definition_bank, actors, spatial, actor, command, globals)
}

///Same algorithm over the actual moved Bank loan; no Catalog fallback.
pub fn authorize_actor_in_bank(
    catalog: &Catalog<'_>,
    bank: &DefinitionBankLoan<'_>,
    actors: &ActorAccess<'_>,
    spatial: &impl Spatial,
    actor: ActorId,
    command: &PlainCommand,
    globals: &mut impl Globals,
) -> Result<bool> {
    authorize_actor_with_bank(catalog, bank, actors, spatial, actor, command, globals)
}

pub fn allocate(
    catalog: &Catalog<'_>,
    runtime: &mut mind_world::Runtime,
    input: Input,
) -> Result<u32> {
    allocate_with_bank(catalog, &catalog.definition_bank, runtime, input)
}

///Same algorithm over the actual moved Bank loan; no Catalog fallback.
pub fn allocate_in_bank(
    catalog: &Catalog<'_>,
    bank: &DefinitionBankLoan<'_>,
    runtime: &mut mind_world::Runtime,
    input: Input,
) -> Result<u32> {
    allocate_with_bank(catalog, bank, runtime, input)
}

pub fn deliver<S: mind_world::Services>(
    catalog: &Catalog<'_>,
    world: &mut mind_world::World<'_, '_, S>,
    arena: &mut Arena,
    command: u32,
    actor: Option<ActorId>,
) -> Result<CommandDelivery> {
    deliver_with_bank(catalog, &catalog.definition_bank, world, arena, command, actor)
}

///Same algorithm over the actual moved Bank loan; no Catalog fallback.
pub fn deliver_in_bank<S: mind_world::Services>(
    catalog: &Catalog<'_>,
    bank: &DefinitionBankLoan<'_>,
    world: &mut mind_world::World<'_, '_, S>,
    arena: &mut Arena,
    command: u32,
    actor: Option<ActorId>,
) -> Result<CommandDelivery> {
    deliver_with_bank(catalog, bank, world, arena, command, actor)
}

pub fn select(
    catalog: &Catalog<'_>,
    actors: &mut ActorAccess<'_>,
    view: &mut impl crate::player_selection::View,
    session: &dyn crate::load_world_globals::SessionPhase,
    actor: Option<ActorId>,
) -> Result<SelectionOutcome> {
    select_with_bank(catalog, &catalog.definition_bank, actors, view, session, actor)
}

///Same algorithm over the actual moved Bank loan; no Catalog fallback.
pub fn select_in_bank(
    catalog: &Catalog<'_>,
    bank: &DefinitionBankLoan<'_>,
    actors: &mut ActorAccess<'_>,
    view: &mut impl crate::player_selection::View,
    session: &dyn crate::load_world_globals::SessionPhase,
    actor: Option<ActorId>,
) -> Result<SelectionOutcome> {
    select_with_bank(catalog, bank, actors, view, session, actor)
}

pub fn deselect(
    catalog: &Catalog<'_>,
    actors: &mut ActorAccess<'_>,
    view: &mut impl crate::player_selection::View,
    actor: ActorId,
) -> Result<bool> {
    deselect_with_bank(catalog, &catalog.definition_bank, actors, view, actor)
}

///Same algorithm over the actual moved Bank loan; no Catalog fallback.
pub fn deselect_in_bank(
    catalog: &Catalog<'_>,
    bank: &DefinitionBankLoan<'_>,
    actors: &mut ActorAccess<'_>,
    view: &mut impl crate::player_selection::View,
    actor: ActorId,
) -> Result<bool> {
    deselect_with_bank(catalog, bank, actors, view, actor)
}

pub fn clear_selection(
    catalog: &Catalog<'_>,
    actors: &mut ActorAccess<'_>,
    view: &mut impl crate::player_selection::View,
) -> Result<()> {
    clear_selection_with_bank(catalog, &catalog.definition_bank, actors, view)
}

///Same algorithm over the actual moved Bank loan; no Catalog fallback.
pub fn clear_selection_in_bank(
    catalog: &Catalog<'_>,
    bank: &DefinitionBankLoan<'_>,
    actors: &mut ActorAccess<'_>,
    view: &mut impl crate::player_selection::View,
) -> Result<()> {
    clear_selection_with_bank(catalog, bank, actors, view)
}

pub fn deliver_selected<S: mind_world::Services>(
    catalog: &Catalog<'_>,
    world: &mut mind_world::World<'_, '_, S>,
    arena: &mut Arena,
    spatial: &impl Spatial,
    globals: &mut impl Globals,
    ordering_player_284: &u32,
    view: &mut impl crate::player_selection::View,
    command: u32,
) -> Result<SelectedDelivery> {
    deliver_selected_with_bank(catalog, &catalog.definition_bank, world, arena, spatial, globals, ordering_player_284, view, command)
}

///Same algorithm over the actual moved Bank loan; no Catalog fallback.
pub fn deliver_selected_in_bank<S: mind_world::Services>(
    catalog: &Catalog<'_>,
    bank: &DefinitionBankLoan<'_>,
    world: &mut mind_world::World<'_, '_, S>,
    arena: &mut Arena,
    spatial: &impl Spatial,
    globals: &mut impl Globals,
    ordering_player_284: &u32,
    view: &mut impl crate::player_selection::View,
    command: u32,
) -> Result<SelectedDelivery> {
    deliver_selected_with_bank(catalog, bank, world, arena, spatial, globals, ordering_player_284, view, command)
}

pub fn issue_selected<S: mind_world::Services>(
    catalog: &Catalog<'_>,
    world: &mut mind_world::World<'_, '_, S>,
    arena: &mut Arena,
    spatial: &impl Spatial,
    globals: &mut impl Globals,
    ordering_player_284: &u32,
    view: &mut impl crate::player_selection::View,
    input: Input,
) -> Result<SelectedDelivery> {
    issue_selected_with_bank(catalog, &catalog.definition_bank, world, arena, spatial, globals, ordering_player_284, view, input)
}

///Same algorithm over the actual moved Bank loan; no Catalog fallback.
pub fn issue_selected_in_bank<S: mind_world::Services>(
    catalog: &Catalog<'_>,
    bank: &DefinitionBankLoan<'_>,
    world: &mut mind_world::World<'_, '_, S>,
    arena: &mut Arena,
    spatial: &impl Spatial,
    globals: &mut impl Globals,
    ordering_player_284: &u32,
    view: &mut impl crate::player_selection::View,
    input: Input,
) -> Result<SelectedDelivery> {
    issue_selected_with_bank(catalog, bank, world, arena, spatial, globals, ordering_player_284, view, input)
}

#[cfg(test)]
#[path = "player_order_world_tests.rs"]
mod tests;
