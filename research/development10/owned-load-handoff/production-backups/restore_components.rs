//! Constructor-then-Load component state in the actor's one authoritative slot
//! table. Definition inputs are supplied by source-aware native getter adapters.
use crate::{Error, Result, saved_components::SavedComponent};
pub use aa_actors::owned::BodyDefinition;
use aa_actors::{ActorId, components::ComponentSlot};
use aa_body::F32;
use aa_numeric::Precision;
pub type GroundDefinitions = aa_actors::owned::Definitions<String>;
pub type RestoredComponent = aa_actors::owned::Component<
    String,
    RestoredMind,
    aa_combat::constructors::OwnedPerformer,
>;
/// The concrete Mind (factory 4CF480 case 4) constructed and loaded by the
/// restore adapter. aa-mapworld does not name the aa-mind types: the adapter
/// owns the value behind this interface and moves it into the live Mind arena
/// when the restored records are staged. The value is never default-built.
pub trait RestoredMindState: std::fmt::Debug {
    fn clone_box(&self) -> Box<dyn RestoredMindState>;
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any>;
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}
#[derive(Debug)]
pub struct RestoredMind {
    /// Concrete Mind virtual +20: Element 5B6830 -> 8 (Element), Regiment
    /// 7C53E0 -> 4 (Mind itself), Aircraft 55EA50 -> 0x17.
    pub required: ComponentSlot,
    pub state: Box<dyn RestoredMindState>,
}
impl Clone for RestoredMind {
    fn clone(&self) -> Self {
        Self {
            required: self.required,
            state: self.state.clone_box(),
        }
    }
}
/// 4CF480 case 4 plus the concrete constructor; `None` is the native null
/// factory result (no ActorData+174 bit 7/23/8).
pub type ConstructMind<'a> = dyn FnMut(
        &aa_actors::components::Components<RestoredComponent>,
    ) -> Result<Option<RestoredMind>>
    + 'a;
/// The Character's current view set (4D0720) per-kind animation arrays:
/// `(kind, index)` -> the index-th animation of that native type, `Ok(None)`
/// when `index >= count` (native `cmp count, index; jle`). Kinds outside the
/// native type table and negative indices read outside the arrays: errors.
pub type CharacterAnimation<'a> =
    dyn Fn(i32, i32) -> std::result::Result<Option<String>, String> + 'a;
/// Transport Load 58C810 over exactly the decoded bytes; returns consumed.
pub type LoadTransport<'a> = dyn Fn(
        &mut aa_actors::transport_state::TransportState<String>,
        &[u8],
    ) -> std::result::Result<usize, String>
    + 'a;
/// The concrete Mind virtual Load (536340 / 528C20 / 51C3C0) over exactly the
/// bytes the decoder consumed.
pub type LoadMind<'a> = dyn Fn(&mut RestoredMind, &[u8], MindLoadContext<'_>) -> std::result::Result<(), String> + 'a;
/// Current owners read by Regiment528C20: GWorld+E0 and Organization+10's
/// actual LayoutData. The Organization is borrowed from the actor's slot.
pub struct MindLoadContext<'a> {
    pub owner: ActorId,
    pub world_time_bits: u32,
    pub organization: Option<&'a aa_actors::ground_states::OrganizationState<String>>,
}
pub type ConstructPerformer<'a> = dyn FnMut(
        &aa_actors::components::Components<RestoredComponent>,
        &mut u32,
    ) -> Result<aa_combat::constructors::OwnedPerformer>
    + 'a;
pub type ApplyPerformerAbility<'a> =
    dyn Fn(u32, &aa_combat::commander::SavedAbility) -> std::result::Result<(), String> + 'a;
pub type ConstructDefender<'a> =
    dyn FnMut() -> Result<aa_actors::defender_state::DefenderState> + 'a;
pub type AbilityWord14<'a> = dyn Fn(u32) -> std::result::Result<u32, String> + 'a;
/// Services of components whose Ability instances live in the shared world
/// arena (Performer 57F4D0, Defender 567CD0).
pub struct PerformerServices<'a> {
    pub construct: &'a mut ConstructPerformer<'a>,
    pub apply_ability: &'a ApplyPerformerAbility<'a>,
    pub construct_defender: &'a mut ConstructDefender<'a>,
    pub ability_word14: &'a AbilityWord14<'a>,
    /// Mind (slot 4): the shared Mind arena and Element ability jitter.
    pub construct_mind: &'a mut ConstructMind<'a>,
    pub load_mind: &'a LoadMind<'a>,
    pub load_transport: &'a LoadTransport<'a>,
    pub character_animation: &'a CharacterAnimation<'a>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum UpdatableTarget {
    /// Singleton GWorld+4C instance, not an actor or a GTID.
    Sun,
    Component { owner: u32, slot: u8 },
    Kingdom { owner: u32 },
    DynamicDoodad { owner: u32, slot: u8 },
    Area { owner: u32 },
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct UpdatableRegistration {
    pub serialized_id: i32,
    pub target: UpdatableTarget,
}
pub fn apply_saved(
    state: &mut RestoredComponent,
    payload: &SavedComponent,
    definition: &GroundDefinitions,
    owner_flags: &mut u32,
    pc: Precision,
) -> Result<()> {
    apply_saved_with_effects(state, payload, definition, owner_flags, pc, None)
}
pub fn apply_saved_with_effects(
    state: &mut RestoredComponent,
    payload: &SavedComponent,
    definition: &GroundDefinitions,
    owner_flags: &mut u32,
    pc: Precision,
    effects: Option<&mut Vec<OwnerEffect>>,
) -> Result<()> {
    match (state, payload) {
        (
            RestoredComponent::Lair(state),
            SavedComponent::Lair {
                words,
                word_1c,
                word_20,
                ..
            },
        ) => {
            state.load(&words.values, *word_1c, *word_20);
            Ok(())
        }
        (RestoredComponent::City(state), SavedComponent::City { words, .. }) => state
            .load(
                words.as_ref().map(|w| w.values.as_slice()),
                definition
                    .city_zone_radius_bits
                    .ok_or_else(|| Error::Format("ActorData city zone_radius unavailable".into()))?,
            )
            .map_err(|e| Error::Format(e.to_string())),
        (RestoredComponent::Research(state), SavedComponent::Research(input)) => {
            let items = definition
                .research_items
                .as_ref()
                .ok_or_else(|| Error::Format("ActorData research list unavailable".into()))?;
            let saved = input
                .references
                .values
                .iter()
                .map(|v| String::from_utf16(v).map_err(|_| Error::Format("research IDS is not UTF-16".into())))
                .collect::<Result<Vec<_>>>()?;
            let actor = state.header.owner;
            let mut fired = Vec::new();
            let result = state.load(&saved, items, |item| fired.push(item.to_owned()));
            let effects = effects.ok_or_else(|| {
                Error::Format("Research Load fires FX lists; an effect sink is required".into())
            })?;
            for definition in fired {
                effects.push(OwnerEffect::PresentationFx {
                    actor,
                    source: "42A590 Property+54 FX list at actor position (5849E0)",
                    definition,
                });
            }
            result.map_err(|e| Error::Format(e.to_string()))
        }
        (RestoredComponent::Body(state), SavedComponent::Body(input)) => input.apply(state, pc),
        (RestoredComponent::Denizen(state), SavedComponent::Denizen(input)) => {
            let health = definition.denizen_target_health_bits.as_ref().ok_or_else(|| {
                Error::Format("Denizen Inventory target health is unresolved".into())
            })?;
            input.apply(
                state,
                |target| {
                    health.get(target).copied().ok_or_else(|| {
                        aa_actors::organization_state::LoadError::Unsupported(format!(
                            "Denizen Inventory target {target} health is unresolved"
                        ))
                    })
                },
                pc,
            )
        }
        (RestoredComponent::Structure(state), SavedComponent::Structure(input)) => {
            input.apply(state)
        }
        (RestoredComponent::Construction(state), SavedComponent::Construction(input)) => input
            .apply(
                state,
                definition
                    .construction
                    .as_ref()
                    .ok_or_else(|| Error::Format("Construction cost inputs disappeared".into()))?,
                true,
            ),
        (RestoredComponent::Mover(state), SavedComponent::Mover(input)) => input.apply(
            state,
            definition.mover.map(|v| v.speed.0),
            false,
            owner_flags,
            pc,
        ),
        (RestoredComponent::Properties(state), SavedComponent::Properties(input)) => {
            if input
                .property_classes
                .iter()
                .any(|class| *class != u32::MAX)
            {
                return Err(Error::Format(
                    "Properties restoration requires actual nonempty effect construction".into(),
                ));
            }
            state.scheduler_link = None;
            Ok(())
        }
        (
            RestoredComponent::Element(state),
            SavedComponent::Element {
                updatable,
                history_count,
                ..
            },
        ) => {
            if updatable.is_some() {
                state.header.scheduled_head = None;
            }
            if history_count.is_some_and(|count| count as i32 > 0) {
                return Err(Error::Format(
                    "nonempty Element history requires native record application".into(),
                ));
            }
            state.flag_1c = false;
            state.time_24 = F32((-1f32).to_bits());
            Ok(())
        }
        (RestoredComponent::Economy(state), SavedComponent::UpdatableOnly { .. }) => {
            state.header.scheduled_head = None;
            Ok(())
        }
        (RestoredComponent::Builder(state), SavedComponent::UpdatableOnly { .. }) => {
            // 562ED0: 50BCD0 (cancels the scheduled entry) and version 0.
            state.header.scheduled_head = None;
            Ok(())
        }
        (RestoredComponent::Detection(state), SavedComponent::UpdatableOnly { .. }) => {
            state.header.scheduled_head = None;
            Ok(())
        }
        (
            RestoredComponent::MobileDeployment(state),
            SavedComponent::MobileDeployment {
                word_10, word_14, ..
            },
        ) => {
            state.load(*word_10, *word_14);
            Ok(())
        }
        (RestoredComponent::Recruit(state), SavedComponent::Recruit(input)) => {
            state.load(input.byte_1c, input.words_14);
            Ok(())
        }
        (RestoredComponent::Supply(state), SavedComponent::Supply(input)) => state
            .load(
                definition
                    .supply
                    .as_ref()
                    .ok_or_else(|| Error::Format("Supply definition disappeared".into()))?,
                input.updatable.is_some(),
                input.amount_multiplier_bits.zip(input.word_1c),
                pc,
            )
            .map_err(|e| Error::Format(e.to_string())),
        (RestoredComponent::Experience(state), SavedComponent::Experience { word_08, .. }) => {
            state.experience = F32(*word_08);
            Ok(())
        }
        _ => Err(Error::Format("component slot/payload type mismatch".into())),
    }
}

/// Executes a single original4D8290 constructor/Load stage. Slot insertion
/// precedes Load, and an error deliberately preserves the partially loaded
/// component. The source decoder has already retained this payload separately.
/// Native side effects of component constructors on other world objects,
/// applied by the restoration driver in construction order (also after a
/// later Load failure, as the native writes would remain).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub enum OwnerEffect {
    /// 58AA50 -> 4E1F00 on the owner Kingdom (actor+E8 via virtual +108).
    SupplySource { kingdom: ActorId, actor: ActorId },
    /// A native FX action list fired during Load (presentation only). It is
    /// recorded for the presentation layer, never reported as executed.
    /// A native log/warning written by Load code that continues.
    NativeMessage { function: u32, line: u32, text: String },
    PresentationFx {
        actor: ActorId,
        /// e.g. `42A590 Property+54` and the canonical definition IDS.
        source: &'static str,
        definition: String,
    },
}
pub struct ComponentOwner<'a> {
    pub id: ActorId,
    /// actor+E8 at construction time (GActor virtual +108).
    pub owner: Option<ActorId>,
    pub owner_effects: &'a mut Vec<OwnerEffect>,
    pub flags: &'a mut u32,
    pub world_time_bits: u32,
    pub position_bits: [u32; 3],
    pub updatable_registrations: &'a mut Vec<UpdatableRegistration>,
    pub register_updatable: &'a dyn Fn(i32, &UpdatableTarget) -> std::result::Result<(), String>,
}
pub fn apply_component_stage(
    components: &mut aa_actors::components::Components<RestoredComponent>,
    owner: ComponentOwner<'_>,
    slot: ComponentSlot,
    action: ComponentLoadAction,
    payload: Option<&SavedComponent>,
    definition: &GroundDefinitions,
    pc: Precision,
) -> Result<()> {
    apply_component_stage_with_services(
        components, owner, slot, action, payload, definition, pc, None,
    )
}

#[allow(clippy::too_many_arguments)] // Preserve the existing bounded factory API.
pub fn apply_component_stage_with_services(
    components: &mut aa_actors::components::Components<RestoredComponent>,
    owner: ComponentOwner<'_>,
    slot: ComponentSlot,
    action: ComponentLoadAction,
    payload: Option<&SavedComponent>,
    definition: &GroundDefinitions,
    pc: Precision,
    mut performer: Option<PerformerServices<'_>>,
) -> Result<()> {
    if action == ComponentLoadAction::DefinitionMismatch {
        // 4D831C: 858A00 TGC_File::ThrowWarningException -> 41DFB0.
        return Err(Error::Format(crate::restore::load_warning_exception(format!(
            "File Warning: Component {} removed from actor",
            slot.index()
        ))));
    }
    if action == ComponentLoadAction::Skip {
        return Ok(());
    }
    if components.present(slot) {
        return Err(Error::Format(format!(
            "saved component slot {} was already occupied before construction",
            slot.index()
        )));
    }
    let state = if slot == ComponentSlot::Mind {
        let made = (performer
            .as_mut()
            .ok_or_else(|| {
                Error::Format("Mind constructor requires the shared Mind arena service".into())
            })?
            .construct_mind)(components)?;
        match made {
            Some(mind) => RestoredComponent::Mind(mind),
            None if matches!(action, ComponentLoadAction::Saved { .. }) => {
                return Err(Error::Format(
                    "saved Mind but factory 4CF480 constructs none; 4D8290 would call Load on null"
                        .into(),
                ));
            }
            None => return Ok(()),
        }
    } else if slot == ComponentSlot::Defender {
        RestoredComponent::Defender(Box::new((performer
            .as_mut()
            .ok_or_else(|| {
                Error::Format("Defender constructor requires the shared Ability arena".into())
            })?
            .construct_defender)()?))
    } else if slot == ComponentSlot::Performer {
        RestoredComponent::Performer((performer
            .as_mut()
            .ok_or_else(|| {
                Error::Format(
                    "Performer constructor requires the shared Ability arena and actor callback"
                        .into(),
                )
            })?
            .construct)(components, owner.flags)?)
    } else {
        aa_actors::owned::construct(
            slot,
            definition,
            aa_actors::owned::Context {
                owner: owner.id,
                world_time: F32(owner.world_time_bits),
                position: owner.position_bits.map(F32),
                precision: pc,
            },
        )
        .map_err(|e| Error::Format(e.to_string()))?
    };
    if slot == ComponentSlot::Supply
        && let Some(kingdom) = owner.owner
    {
        owner.owner_effects.push(OwnerEffect::SupplySource {
            kingdom,
            actor: owner.id,
        });
    }
    components.insert(slot, state);
    if let ComponentLoadAction::Saved { remove_after_load } = action {
        let payload = payload
            .ok_or_else(|| Error::Format("saved component stage has no decoded payload".into()))?;
        if let Some(id) = payload
            .updatable()
            .and_then(|v| v.serialized_id)
            .filter(|id| *id >= 0)
        {
            let target = UpdatableTarget::Component {
                owner: owner.id.raw(),
                slot: slot.index() as u8,
            };
            (owner.register_updatable)(id, &target).map_err(Error::Format)?;
            owner.updatable_registrations.push(UpdatableRegistration {
                serialized_id: id,
                target,
            });
        }
        if let (
            Some(services),
            RestoredComponent::Performer(state),
            SavedComponent::Performer(input),
        ) = (
            performer.as_ref(),
            components.get_mut(slot).expect("component inserted above"),
            payload,
        ) {
            crate::performer_restore::apply(state, input, pc, services.apply_ability)?;
        } else if let (Some(services), SavedComponent::Mind(input)) = (performer.as_ref(), payload)
            && matches!(components.get(slot), Some(RestoredComponent::Mind(_)))
        {
            // 528C20 does not reenter the Mind slot. Temporarily hold its
            // actual owner to borrow the current Organization independently;
            // reinsert the same component even if the loader rejects input.
            let mut component = components.remove(slot).expect("Mind inserted above");
            let RestoredComponent::Mind(state) = &mut component else { unreachable!() };
            let organization = match components.get(ComponentSlot::Organization) {
                Some(RestoredComponent::Organization(value)) => Some(value.as_ref()),
                _ => None,
            };
            let result = (services.load_mind)(state, &input.bytes, MindLoadContext {
                owner: owner.id,
                world_time_bits: owner.world_time_bits,
                organization,
            });
            components.insert(slot, component);
            result.map_err(Error::Format)?;
        } else if let (
            Some(services),
            RestoredComponent::Character(state),
            SavedComponent::Character(input),
        ) = (
            performer.as_ref(),
            components.get_mut(slot).expect("component inserted above"),
            payload,
        ) {
            apply_character(
                state,
                input,
                owner.id,
                owner.world_time_bits,
                services.character_animation,
                owner.owner_effects,
            )?;
        } else if let (
            Some(services),
            RestoredComponent::Transport(state),
            SavedComponent::Transport(input),
        ) = (
            performer.as_ref(),
            components.get_mut(slot).expect("component inserted above"),
            payload,
        ) {
            let consumed = (services.load_transport)(state, &input.bytes).map_err(Error::Format)?;
            if consumed != input.bytes.len() {
                return Err(Error::Format(
                    "Transport Load length changed between decode and Load".into(),
                ));
            }
        } else if let (
            Some(services),
            RestoredComponent::Defender(state),
            SavedComponent::Defender(input),
        ) = (
            performer.as_ref(),
            components.get_mut(slot).expect("component inserted above"),
            payload,
        ) {
            apply_defender(state, input, services)?;
        } else {
            apply_saved_with_effects(
                components.get_mut(slot).expect("component inserted above"),
                payload,
                definition,
                owner.flags,
                pc,
                Some(&mut *owner.owner_effects),
            )?;
        }
        if remove_after_load {
            components.remove(slot);
        }
    }
    Ok(())
}

/// Character Load 563930 after the Updatable record (50BCD0, whose 50B250
/// cancels the scheduled update): +28; legacy word below version 1; the
/// saved (kind, index) animation through 4D0720's per-kind arrays and the
/// Animator Load 4DAFC0 (version 0, start +4, paused time +8, rate +C,
/// paused flag +14); a missing animation is logged (404F60, line 0x1E9),
/// the Animator loads with null and 4DB120 sets the kind-0 first animation
/// (start = GWorld+E0, unpaused, rate 1); +2C at version 2; then 42A590 on
/// the animation's +38 action list (presentation, recorded).
fn apply_character(
    state: &mut aa_actors::ground_states::CharacterState<String>,
    input: &crate::saved_components::CharacterPayload,
    owner: ActorId,
    world_time_bits: u32,
    animation: &CharacterAnimation<'_>,
    effects: &mut Vec<OwnerEffect>,
) -> Result<()> {
    state.header.scheduled_head = None;
    state.word_28 = input.word_28;
    let found =
        animation(input.animation_kind, input.animation_index).map_err(Error::Format)?;
    // 4DAFC0 stores the animation argument, then reads the four fields.
    state.animation = found.clone();
    state.elapsed = F32(input.elapsed_bits);
    state.resumed_at = F32(input.resumed_at_bits);
    state.rate = F32(input.rate_bits);
    state.active = input.active_byte != 0;
    if found.is_none() {
        effects.push(OwnerEffect::NativeMessage {
            function: 0x404f60,
            line: 0x1e9,
            text: format!(
                "Animation '{}' of type '{}' not found for actor",
                input.animation_index, input.animation_kind
            ),
        });
        let default = animation(0, 0).map_err(Error::Format)?;
        // 4DB120: current (null after the Load) == new reads [new+8].
        let Some(default) = default else {
            return Err(Error::Format(
                "native null dereference at 0x4db12e (no kind-0 animation)".into(),
            ));
        };
        state.elapsed = F32(world_time_bits);
        state.animation = Some(default);
        state.active = false;
        state.rate = F32::ONE;
    }
    if let Some(variant) = input.variant {
        state.variant = variant;
    }
    let current = state.animation.clone().ok_or_else(|| {
        Error::Format("native null dereference at 0x563a77 (42A590 on a null animation)".into())
    })?;
    effects.push(OwnerEffect::PresentationFx {
        actor: owner,
        source: "42A590 Animation+38 action list (Character Load 563A77)",
        definition: current,
    });
    Ok(())
}
/// Original 567020 after the Updatable record: Ability records into
/// min(i, count-1), then the entry list (rebuilt from Ability+14 below
/// version 2, else the saved pairs).
fn apply_defender(
    state: &mut aa_actors::defender_state::DefenderState,
    input: &crate::saved_components::DefenderPayload,
    services: &PerformerServices<'_>,
) -> Result<()> {
    if let Some(records) = &input.abilities {
        for (index, record) in records.values.iter().enumerate() {
            let handle = state.load_target(index).ok_or_else(|| {
                Error::Format("saved Defender Ability record without any Ability (native reads before its vector)".into())
            })?;
            (services.apply_ability)(handle, &crate::commander_restore::saved_ability(record))
                .map_err(Error::Format)?;
        }
    }
    state
        .load_entries(
            input.pairs.as_ref().map(|p| p.values.as_slice()),
            |handle| (services.ability_word14)(handle),
        )
        .map_err(Error::Format)
}
/// Original4D8290: a saved component is always constructed and loaded. Body
/// and Experience are then deleted if the current definition lacks their bit.
/// For absent saved components, defaults are created only for a newly added
/// definition bit, with the explicit Properties exception.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComponentLoadAction {
    Skip,
    Default,
    /// The original reaches its 858A00 diagnostic before construction.
    DefinitionMismatch,
    Saved {
        remove_after_load: bool,
    },
}
pub fn component_load_action(
    slot: ComponentSlot,
    saved: bool,
    original_default_mask: u32,
    definition_mask: u32,
) -> ComponentLoadAction {
    let bit = 1 << slot.index();
    let defined = definition_mask & bit != 0;
    if saved {
        if !defined
            && !matches!(
                slot,
                ComponentSlot::Body | ComponentSlot::Mind | ComponentSlot::Experience
            )
        {
            ComponentLoadAction::DefinitionMismatch
        } else {
            ComponentLoadAction::Saved {
                remove_after_load: !defined
                    && matches!(slot, ComponentSlot::Body | ComponentSlot::Experience),
            }
        }
    } else if defined && (original_default_mask & bit == 0 || slot == ComponentSlot::Properties) {
        ComponentLoadAction::Default
    } else {
        ComponentLoadAction::Skip
    }
}
