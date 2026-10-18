//! The shared authoritative component payload for newly created and loaded actors.
//! Generic Mind/Performer values are real runtime handles or owned concrete states;
//! `Infallible` explicitly excludes them while saved support is still incomplete.
use crate::{ActorId, components::ComponentSlot, ground_states::*};
use aa_body::{F32, credit::Object, promotion::ExperienceState, properties::PropertiesState};
use aa_numeric::Precision;
use std::convert::Infallible;
#[derive(Clone, Debug)]
pub enum Component<D = String, M = Infallible, P = Infallible> {
    Body(Box<BodyState>),
    Character(Box<CharacterState<D>>),
    Mover(Box<MoverState>),
    Mind(M),
    Performer(P),
    Properties(Box<PropertiesState>),
    Organization(Box<OrganizationState<D>>),
    Element(Box<ElementState<D>>),
    Economy(Box<EconomyState>),
    Construction(Box<crate::construction_state::ConstructionState>),
    Denizen(Box<crate::denizen_state::DenizenState>),
    Structure(Box<crate::structure_state::StructureState>),
    Supply(Box<crate::supply_state::SupplyState>),
    Recruit(Box<crate::recruit_state::RecruitState>),
    Defender(Box<crate::defender_state::DefenderState>),
    Research(Box<crate::research_state::ResearchState>),
    Lair(Box<crate::lair_city_state::LairState>),
    City(Box<crate::lair_city_state::CityState>),
    MobileDeployment(Box<crate::mobile_deployment_state::MobileDeploymentState>),
    Experience(Box<ExperienceState>),
    Detection(Box<DetectionState>),
    Transport(Box<crate::transport_state::TransportState<D>>),
    Builder(Box<crate::builder_state::BuilderState>),
}
impl<D, M, P> Component<D, M, P> {
    pub fn slot(&self) -> ComponentSlot {
        match self {
            Self::Body(_) => ComponentSlot::Body,
            Self::Character(_) => ComponentSlot::Character,
            Self::Mover(_) => ComponentSlot::Mover,
            Self::Mind(_) => ComponentSlot::Mind,
            Self::Performer(_) => ComponentSlot::Performer,
            Self::Properties(_) => ComponentSlot::Properties,
            Self::Organization(_) => ComponentSlot::Organization,
            Self::Element(_) => ComponentSlot::Element,
            Self::Economy(_) => ComponentSlot::Economy,
            Self::Construction(_) => ComponentSlot::Construction,
            Self::Denizen(_) => ComponentSlot::Denizen,
            Self::Structure(_) => ComponentSlot::Structure,
            Self::Supply(_) => ComponentSlot::Supply,
            Self::Recruit(_) => ComponentSlot::Recruit,
            Self::Defender(_) => ComponentSlot::Defender,
            Self::Research(_) => ComponentSlot::Research,
            Self::Lair(_) => ComponentSlot::Lair,
            Self::City(_) => ComponentSlot::City,
            Self::MobileDeployment(_) => ComponentSlot::MobileDeployment,
            Self::Experience(_) => ComponentSlot::Experience,
            Self::Detection(_) => ComponentSlot::Detection,
            Self::Transport(_) => ComponentSlot::Transport,
            Self::Builder(_) => ComponentSlot::Builder,
        }
    }
    /// Moves all existing ground boxes without cloning or rerunning constructors.
    /// Only specialized arena handles/states are transformed by the caller.
    pub fn map_specialized<N, Q>(
        self,
        mind: impl FnOnce(M) -> N,
        performer: impl FnOnce(P) -> Q,
    ) -> Component<D, N, Q> {
        match self {
            Self::Body(v) => Component::Body(v),
            Self::Character(v) => Component::Character(v),
            Self::Mover(v) => Component::Mover(v),
            Self::Mind(v) => Component::Mind(mind(v)),
            Self::Performer(v) => Component::Performer(performer(v)),
            Self::Properties(v) => Component::Properties(v),
            Self::Organization(v) => Component::Organization(v),
            Self::Element(v) => Component::Element(v),
            Self::Economy(v) => Component::Economy(v),
            Self::Construction(v) => Component::Construction(v),
            Self::Denizen(v) => Component::Denizen(v),
            Self::Structure(v) => Component::Structure(v),
            Self::Supply(v) => Component::Supply(v),
            Self::Recruit(v) => Component::Recruit(v),
            Self::Defender(v) => Component::Defender(v),
            Self::Research(v) => Component::Research(v),
            Self::Lair(v) => Component::Lair(v),
            Self::City(v) => Component::City(v),
            Self::MobileDeployment(v) => Component::MobileDeployment(v),
            Self::Experience(v) => Component::Experience(v),
            Self::Detection(v) => Component::Detection(v),
            Self::Transport(v) => Component::Transport(v),
            Self::Builder(v) => Component::Builder(v),
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct BodyDefinition {
    pub maximum_bits: u32,
    pub defense_bits: u32,
}
#[derive(Clone, Debug)]
pub struct OrganizationDefinition<D> {
    pub layout: LayoutDefinition<D>,
    pub morale_breakpoint_bits: u32,
}
/// These values come from the completed native definition getters/registries.
/// `None` means unavailable, not an empty/default runtime definition.
#[derive(Clone, Debug)]
pub struct Definitions<D = String> {
    pub body: Option<BodyDefinition>,
    pub mover: Option<MoverDefinition>,
    pub character_animations: Option<Vec<Option<D>>>,
    pub organization: Option<OrganizationDefinition<D>>,
    pub properties_domain_count: Option<u32>,
    pub resource_domain_count: Option<usize>,
    pub detection_radius_bits: Option<u32>,
    pub construction: Option<crate::construction_state::Definition>,
    /// 56B720 Inventory slots, head first (ActorCatalog::native_denizen_inventory).
    pub denizen: Option<Vec<crate::denizen_state::InventorySlotDefinition>>,
    /// 45D730 (null modifier) ActorData+284 health bits of every Inventory
    /// slot target, read by the Denizen saved Load's occupant values.
    pub denizen_target_health_bits: Option<std::collections::BTreeMap<String, u32>>,
    pub supply: Option<crate::supply_state::Definition>,
    /// ActorData+558 ResearchItems as canonical Property IDS, list order.
    pub research_items: Option<Vec<String>>,
    /// ActorData+5B4 CityComponent `zone_radius`.
    pub city_zone_radius_bits: Option<u32>,
}
impl<D> Default for Definitions<D> {
    fn default() -> Self {
        Self {
            body: None,
            mover: None,
            character_animations: None,
            organization: None,
            properties_domain_count: None,
            resource_domain_count: None,
            detection_radius_bits: None,
            construction: None,
            denizen: None,
            denizen_target_health_bits: None,
            supply: None,
            research_items: None,
            city_zone_radius_bits: None,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Context {
    pub owner: ActorId,
    pub world_time: F32,
    pub position: [F32; 3],
    pub precision: Precision,
}
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Ground(#[from] crate::ground_states::Error),
    #[error(transparent)]
    Body(#[from] aa_body::Error),
    #[error("component {slot:?} requires completed source input {input}")]
    Missing {
        slot: ComponentSlot,
        input: &'static str,
    },
    #[error("component {0:?} requires a specialized concrete factory")]
    Specialized(ComponentSlot),
}
pub type Result<T> = std::result::Result<T, Error>;
fn required<T>(slot: ComponentSlot, value: Option<T>, input: &'static str) -> Result<T> {
    value.ok_or(Error::Missing { slot, input })
}
/// Actually allocates one ground component with its proved constructor. It does
/// not run InitCreate, allocate GTIDs, publish spatial state or draw world RNG.
/// The caller inserts this value in the central shell's component slot before
/// advancing the next native constructor or saved Load stage.
pub fn construct<D: Clone, M, P>(
    slot: ComponentSlot,
    definition: &Definitions<D>,
    context: Context,
) -> Result<Component<D, M, P>> {
    let c = context;
    Ok(match slot {
        ComponentSlot::Body => {
            let d = required(slot, definition.body, "null-context health and defense")?;
            Component::Body(Box::new(BodyState::new(
                c.owner,
                F32(d.maximum_bits),
                F32(d.defense_bits),
            )?))
        }
        ComponentSlot::Mover => Component::Mover(Box::new(MoverState::new(
            c.owner,
            required(slot, definition.mover, "null-context Mover getters")?,
            c.precision,
        )?)),
        ComponentSlot::Character => Component::Character(Box::new(CharacterState::new(
            c.owner,
            required(
                slot,
                definition.character_animations.as_deref(),
                "resolved Character view animations",
            )?,
        ))),
        ComponentSlot::Properties => Component::Properties(Box::new(PropertiesState::new(
            Object(c.owner.raw()),
            required(
                slot,
                definition.properties_domain_count,
                "completed AbilityType domain count",
            )?,
        )?)),
        ComponentSlot::Organization => {
            let d = required(
                slot,
                definition.organization.as_ref(),
                "resolved Organization layout and morale breakpoint",
            )?;
            Component::Organization(Box::new(OrganizationState::new(
                c.owner,
                d.layout.clone(),
                c.world_time,
                F32(d.morale_breakpoint_bits),
                [c.position[0], c.position[1]],
            )?))
        }
        ComponentSlot::Element => Component::Element(Box::new(ElementState::new(c.owner)?)),
        ComponentSlot::Economy => Component::Economy(Box::new(EconomyState::new(
            c.owner,
            required(
                slot,
                definition.resource_domain_count,
                "completed Resource domain count",
            )?,
        )?)),
        ComponentSlot::Experience => {
            Component::Experience(Box::new(ExperienceState::new(Object(c.owner.raw()))))
        }
        ComponentSlot::Construction => {
            Component::Construction(Box::new(crate::construction_state::ConstructionState::new(
                c.owner,
                required(
                    slot,
                    definition.construction.as_ref(),
                    "resolved null-modifier costs and completed predecessor-chain proof",
                )?,
            )?))
        }
        ComponentSlot::Denizen => {
            Component::Denizen(Box::new(crate::denizen_state::DenizenState::with_inventory(
                c.owner,
                required(
                    slot,
                    definition.denizen.as_deref(),
                    "resolved Denizen Inventory",
                )?,
            )))
        }
        ComponentSlot::Detection => Component::Detection(Box::new(DetectionState::new(
            c.owner,
            F32(required(
                slot,
                definition.detection_radius_bits,
                "finalized null-context Detection radius",
            )?),
        ))),
        ComponentSlot::Supply => Component::Supply(Box::new(
            crate::supply_state::SupplyState::new(
                c.owner,
                required(
                    slot,
                    definition.supply.as_ref(),
                    "ActorData SupplyComponent fields",
                )?,
            ),
        )),
        ComponentSlot::Lair => Component::Lair(Box::new(
            crate::lair_city_state::LairState::new(c.owner),
        )),
        ComponentSlot::City => Component::City(Box::new(
            crate::lair_city_state::CityState::new(c.owner),
        )),
        ComponentSlot::Research => Component::Research(Box::new(
            crate::research_state::ResearchState::new(c.owner),
        )),
        ComponentSlot::MobileDeployment => Component::MobileDeployment(Box::new(
            crate::mobile_deployment_state::MobileDeploymentState::new(c.owner),
        )),
        ComponentSlot::Recruit => Component::Recruit(Box::new(
            crate::recruit_state::RecruitState::new(c.owner),
        )),
        ComponentSlot::Structure => Component::Structure(Box::new(
            crate::structure_state::StructureState::new(c.owner),
        )),
        // 562F10: the component base 4DB0C0 only.
        ComponentSlot::Builder => {
            Component::Builder(Box::new(crate::builder_state::BuilderState::new(c.owner)))
        }
        // 58B9A0: cost/upkeep vectors sized by the registered Resource count
        // [B3A660]+28 (the value EconomyState::new uses).
        ComponentSlot::Transport => Component::Transport(Box::new(
            crate::transport_state::TransportState::new(
                c.owner,
                required(
                    slot,
                    definition.resource_domain_count,
                    "completed Resource domain count",
                )?,
            ),
        )),
        _ => return Err(Error::Specialized(slot)),
    })
}
