//! Live-World leaves of the UIWorldState command masks 0x437EB0
//! ([`super::command_masks`]): the Thing/Actor/Organization/Research reads,
//! actor vt+0x154 (0x4CE680) and 0x5845A0, over the SAME ActorStore, Bank,
//! spatial owner and Globals the order paths use: between frames the
//! read-only `World::presentation_parts` (the UI thread's 0x43A1C0 / list
//! callers), inside a frame the `GameplayOrderLoans` (0x50C8E0 from the
//! simulation's 0x4CFCC0).
//!
//! Bindings (native -> live owner; proofs in
//! `research/phase8-orders-input/command-masks/` unless noted):
//! * Thing vt+0xB4 by runtime class: 3 GActor 0xA273F0, 4 GKingdom
//!   0xA299D8 and 8 GGroup 0xA29490 -> 0x6A71D0 (true); the primary
//!   vtables of classes 2/5/6/7/9/10 (0xA2A2A0, 0xA29780, 0xA28A00,
//!   0xA28B88, 0xA28E90, 0xA28D18) -> 0x6A71F0 (false);
//! * Thing vt+0x20: classes 3/4 -> 0x4CF120 = `eligible_with_bank`
//!   (research/skirmish-20261016-player-orders); classes 2/5/6/7/9/10 ->
//!   0x969B10 (false);
//! * Thing vt+0x134: class 3 -> 0x4D1E90 (Actor+0xE8 non-NULL and equal),
//!   class 4 -> 0x4E5240 (this == kingdom);
//! * actor vt+0x154 (all three vtables) -> [`virtual_154_4ce680`] with
//!   Actor+0xE8 = `actor_word_with_bank(0xE8)`, vt+0x20 = `eligible_with_bank`
//!   and vt+0x34 = `allows_with_bank` (0x4D1F90 F0/F4 bit);
//! * Actor+0x7C / Organization+0x28,+0x2C / Actor+0x100 -> the
//!   `player_order_world::guess::Live` reads (Organization slot, member
//!   array, `Components.state.low`); Organization+0x8C -> its saved/constructed
//!   byte; `[[B36530]+0x298][i]+0x3C` -> `guess::Live::definition` (the
//!   predefined command plus the actor_command_ui controls in the same Bank);
//! * Actor+0xB4 -> the Research slot; 0x5845A0 -> [`research_available_5845a0`]
//!   over the owner's ActorData+0x558 list (the bound definition's research
//!   items) and the component's +0x10 list.
//!
//! Explicit errors (no default): GGroup (class 8) reads, because the shared
//! order helpers bind only the ordinary Actor/Kingdom layouts; 0x5844A0 for
//! a non-empty research list (the research admission owners are not bound on
//! this path); a class without a Thing vtable; any failing owner read.
use super::command_masks::{self as kernel, MaskLeaves, PermissionLeaves, ResearchLeaves, virtual_154_4ce680};
use crate::{
    actor_world::{RuntimeComponent, RuntimeObject, Spatial},
    catalog::Catalog,
    live_world_frame::{GameplayOrderLoans, PresentationParts},
    player_order_world::{self as orders, ActorRead, Banks, DefinitionBankLoan, Globals, ReadBank, guess::{self, World as _}},
};
use aa_actors::{ActorId, components::ComponentSlot, ground_states::OrganizationState};
use std::fmt::Debug;

pub type Result<T> = std::result::Result<T, String>;
fn text(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// The Thing classes whose primary vtable is bound here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    /// 3 GActor 0xA273F0 / 4 GKingdom 0xA299D8.
    Ordinary(u32),
    /// 8 GGroup 0xA29490.
    Group,
    /// 2, 5, 6, 7, 9, 10: vt+0xB4 0x6A71F0, vt+0x20 0x969B10.
    Other(u32),
}

/// [`MaskLeaves`] over the live World. `thing_14` is the view directory's
/// VThing+0x14 (the HUD/client owner of the views supplies it).
pub struct Live<'a, 'src, V, A: ?Sized, P, G, F> {
    view: std::marker::PhantomData<fn(V)>,
    catalog: &'a Catalog<'src>,
    banks: &'a dyn Banks,
    actors: &'a A,
    spatial: &'a P,
    globals: &'a mut G,
    local_kingdom_5c0e30: Result<Option<ActorId>>,
    thing_14: F,
}
impl<'a, 'src, V, A: ?Sized, P, G, F> Live<'a, 'src, V, A, P, G, F> {
    /// In a frame, over the moved Bank loan. `local_kingdom_5c0e30` is the
    /// session read 0x5C0E30 returns (`[B36550]` NULL -> None, else
    /// 0x76AC70); an Err is returned only when 0x437EB0 reaches the call.
    pub fn new(
        catalog: &'a Catalog<'src>,
        bank: &'a DefinitionBankLoan<'_>,
        actors: &'a A,
        spatial: &'a P,
        globals: &'a mut G,
        local_kingdom_5c0e30: Result<Option<ActorId>>,
        thing_14: F,
    ) -> Self {
        Self::with_banks(catalog, bank, actors, spatial, globals, local_kingdom_5c0e30, thing_14)
    }
    /// Between frames, over the World's Bank read-only.
    pub fn new_read(
        catalog: &'a Catalog<'src>,
        bank: &'a ReadBank<'_>,
        actors: &'a A,
        spatial: &'a P,
        globals: &'a mut G,
        local_kingdom_5c0e30: Result<Option<ActorId>>,
        thing_14: F,
    ) -> Self {
        Self::with_banks(catalog, bank, actors, spatial, globals, local_kingdom_5c0e30, thing_14)
    }
    pub(crate) fn with_banks(
        catalog: &'a Catalog<'src>,
        banks: &'a dyn Banks,
        actors: &'a A,
        spatial: &'a P,
        globals: &'a mut G,
        local_kingdom_5c0e30: Result<Option<ActorId>>,
        thing_14: F,
    ) -> Self {
        Self { view: std::marker::PhantomData, catalog, banks, actors, spatial, globals, local_kingdom_5c0e30, thing_14 }
    }
}
impl<'a, 'src, V, A: ActorRead + ?Sized, P: Spatial, G: Globals, F> Live<'a, 'src, V, A, P, G, F> {
    fn guess(&mut self) -> guess::Live<'_, 'src, A, G> {
        guess::Live { catalog: self.catalog, banks: self.banks, actors: self.actors, globals: &mut *self.globals }
    }
    fn class(&self, thing: ActorId) -> Result<Class> {
        let key = self.actors.key(thing).ok_or("0x437EB0 view+0x14 Thing: current generation")?;
        let d = &self.actors.object(key).map_err(text)?.definition;
        if d.native_class != d.proof.category.runtime_class {
            return Err("0x437EB0 Thing class differs from its source category".into());
        }
        match d.native_class {
            3 | 4 => Ok(Class::Ordinary(d.native_class)),
            8 => Ok(Class::Group),
            2 | 5 | 6 | 7 | 9 | 10 => Ok(Class::Other(d.native_class)),
            c => Err(format!("0x437EB0 Thing class {c} has no 4F9C10 primary vtable")),
        }
    }
    fn ordinary(&self, actor: ActorId, leaf: &str) -> Result<&'a RuntimeObject> {
        match self.class(actor)? {
            Class::Ordinary(_) => orders::object_with_bank(self.catalog, self.banks, self.actors, actor).map_err(text),
            Class::Group => Err(format!("0x437EB0 {leaf} on a GGroup (A29490): the GROUP layout is not bound by the order helpers")),
            Class::Other(c) => Err(format!("0x437EB0 {leaf} on a class {c} Thing (no GActor fields)")),
        }
    }
    fn organization(&self, organization: ActorId) -> Result<&'a OrganizationState<String>> {
        match self.ordinary(organization, "Organization+0x8C")?.components().map_err(text)?.get(ComponentSlot::Organization) {
            Some(RuntimeComponent::Organization(o)) if o.header.owner == organization => Ok(&**o),
            _ => Err("0x437F05 actual Organization slot payload/owner".into()),
        }
    }
    fn owner_e8(&self, actor: ActorId) -> Result<Option<ActorId>> {
        let raw = orders::actor_word_with_bank(self.catalog, self.banks, self.actors, actor, 0xe8).map_err(text)?;
        Ok((raw != 0).then_some(ActorId::from_raw(raw)))
    }
}

/// vt+0x154 = 0x4CE680 leaves on an ordinary Actor/Kingdom.
struct Permission<'l, 'a, 'src, V, A: ?Sized, P, G, F>(&'l mut Live<'a, 'src, V, A, P, G, F>);
impl<V, A: ActorRead + ?Sized, P: Spatial, G: Globals, F> PermissionLeaves for Permission<'_, '_, '_, V, A, P, G, F> {
    type Actor = ActorId;
    type Kingdom = ActorId;
    fn owner_e8(&mut self, actor: ActorId) -> Result<Option<ActorId>> {
        self.0.owner_e8(actor)
    }
    fn virtual_20(&mut self, actor: ActorId, argument: u8) -> Result<bool> {
        let l = &mut *self.0;
        orders::eligible_with_bank(l.catalog, l.banks, l.actors, l.spatial, actor, argument, &mut *l.globals).map_err(text)
    }
    fn virtual_34(&mut self, actor: ActorId, index: u32) -> Result<bool> {
        let l = &*self.0;
        orders::allows_with_bank(l.catalog, l.banks, l.actors, actor, index).map_err(text)
    }
}

/// 0x5845A0 leaves on a Research component (identified by its owner).
struct Research<'l, 'a, 'src, V, A: ?Sized, P, G, F>(&'l Live<'a, 'src, V, A, P, G, F>);
impl<V, A: ActorRead + ?Sized, P: Spatial, G: Globals, F> Research<'_, '_, '_, V, A, P, G, F> {
    fn items(&self, owner: ActorId) -> Result<&[String]> {
        // [[this+4]+4]+0x558: the owner's ActorData ResearchItem list, bound
        // with the definition (catalog native_research_property_ids).
        self.0.ordinary(owner, "ActorData+0x558")?.definition.ground.research_items.as_deref()
            .ok_or_else(|| "0x5845AB ActorData+0x558 research list of a Research component owner is unbound".into())
    }
}
impl<V, A: ActorRead + ?Sized, P: Spatial, G: Globals, F> ResearchLeaves for Research<'_, '_, '_, V, A, P, G, F> {
    type Research = ActorId;
    type Item = usize;
    fn items_558(&mut self, owner: ActorId) -> Result<Vec<usize>> {
        Ok((0..self.items(owner)?.len()).collect())
    }
    fn blocked_5844a0(&mut self, _: ActorId, _: usize, _: u32) -> Result<u32> {
        Err("0x5844A0 research admission (aa_production::research::research_blocked) has no live owner on the UI mask path: \
             actor vt+34(0x23), vt+108 kingdom, Structure+94 queue 588A40, Kingdom+324 4E2390, 4CED40/581280/4E0FB0 \
             properties and 5842F0 requirements"
            .into())
    }
    fn listed_10(&mut self, owner: ActorId) -> Result<Vec<usize>> {
        // +0x10 holds ActorData records; ResearchState keeps their canonical
        // Property IDS (5849E0 resolves each to the first matching record).
        let items = self.items(owner)?;
        let state = match self.0.ordinary(owner, "Research+0x10")?.components().map_err(text)?.get(ComponentSlot::Research) {
            Some(RuntimeComponent::Research(r)) if r.header.owner == owner => r,
            _ => return Err("0x5845C5 actual Research slot payload/owner".into()),
        };
        state.items.iter().map(|id| {
            items.iter().position(|i| i.eq_ignore_ascii_case(id))
                .ok_or_else(|| format!("Research+0x10 item {id:?} is not in the owner's ActorData+0x558 list"))
        }).collect()
    }
}

impl<V, A, P, G, F> MaskLeaves for Live<'_, '_, V, A, P, G, F>
where
    V: Copy + Eq + Debug,
    A: ActorRead + ?Sized,
    P: Spatial,
    G: Globals,
    F: FnMut(V) -> Result<Option<ActorId>>,
{
    type View = V;
    type Object = ActorId;
    type Organization = ActorId;
    type Kingdom = ActorId;
    type Research = ActorId;
    fn thing_14(&mut self, view: V) -> Result<Option<ActorId>> {
        (self.thing_14)(view)
    }
    fn virtual_b4(&mut self, thing: ActorId) -> Result<bool> {
        match self.class(thing)? {
            Class::Ordinary(_) => self.ordinary(thing, "vt+0xB4").map(|_| true),
            Class::Group => Ok(true),
            Class::Other(_) => Ok(false),
        }
    }
    fn organization_7c(&mut self, actor: ActorId) -> Result<Option<ActorId>> {
        self.ordinary(actor, "Actor+0x7C")?;
        self.guess().organization_7c(actor).map_err(text)
    }
    fn state_100(&mut self, actor: ActorId) -> Result<u32> {
        self.ordinary(actor, "Actor+0x100")?;
        self.guess().state_100(actor).map_err(text)
    }
    fn organization_8c(&mut self, organization: ActorId) -> Result<bool> {
        let byte = self.organization(organization)?.bytes.get(&0x8c).copied()
            .ok_or("0x437F05 actual Organization+0x8C byte")?;
        Ok(byte != 0)
    }
    fn non_group_3c(&mut self, index: u32) -> Result<Option<bool>> {
        Ok(Some(self.guess().definition(index).map_err(text)?.non_group_3c))
    }
    fn local_kingdom_5c0e30(&mut self) -> Result<Option<ActorId>> {
        self.local_kingdom_5c0e30.clone()
    }
    fn virtual_154(&mut self, actor: ActorId, kingdom: Option<ActorId>, index: u32, selected: u8) -> Result<bool> {
        self.ordinary(actor, "vt+0x154 (0x4CE680)")?;
        virtual_154_4ce680(&mut Permission(self), actor, kingdom, index, selected)
    }
    fn research_b4(&mut self, actor: ActorId) -> Result<Option<ActorId>> {
        match self.ordinary(actor, "Actor+0xB4")?.components().map_err(text)?.get(ComponentSlot::Research) {
            None => Ok(None),
            Some(RuntimeComponent::Research(r)) if r.header.owner == actor => Ok(Some(actor)),
            Some(_) => Err("0x438032 actual Research slot payload/owner".into()),
        }
    }
    fn research_available_5845a0(&mut self, research: ActorId) -> Result<bool> {
        kernel::research_available_5845a0(&mut Research(self), research)
    }
    fn members_28(&mut self, organization: ActorId) -> Result<Vec<Option<ActorId>>> {
        self.ordinary(organization, "Organization+0x28")?;
        self.guess().members_28(organization).map_err(text)
    }
    fn virtual_20(&mut self, thing: ActorId, argument: u8) -> Result<bool> {
        match self.class(thing)? {
            Class::Ordinary(_) => Permission(self).virtual_20(thing, argument),
            Class::Group => Err("0x43810B vt+0x20 0x4CF120 on a GGroup (A29490): the GROUP layout is not bound".into()),
            Class::Other(_) => Ok(false),
        }
    }
    fn virtual_134(&mut self, thing: ActorId, kingdom: ActorId) -> Result<bool> {
        match self.class(thing)? {
            Class::Ordinary(3) => {
                self.ordinary(thing, "vt+0x134 (0x4D1E90)")?;
                Ok(self.owner_e8(thing)? == Some(kingdom))
            }
            Class::Ordinary(_) => {
                self.ordinary(thing, "vt+0x134 (0x4E5240)")?;
                Ok(thing == kingdom)
            }
            Class::Group => Err("0x438128 vt+0x134 0x4D1E90 on a GGroup (A29490): the GROUP layout is not bound".into()),
            Class::Other(c) => Err(format!("0x438128 vt+0x134 on a class {c} Thing (its vt+0x20 0x969B10 is false)")),
        }
    }
}

/// 0x437EB0 between frames over `world.presentation_parts()` (read-only
/// store, spatial owner and Bank): the UI thread's callers. `selected` is
/// UIWorldState+0x08 head first, `thing_14` its views' +0x14 Things,
/// `kingdom_4c` the +0x4C word, `group_40_byte_18` the +0x40 group's +0x18
/// byte (None for NULL). Returns +0x20, +0x24, +0x28, +0x2C.
#[allow(clippy::too_many_arguments)]
pub fn command_masks_between_frames<V, F>(
    catalog: &Catalog<'_>,
    parts: &PresentationParts<'_>,
    globals: &mut impl Globals,
    local_kingdom_5c0e30: Result<Option<ActorId>>,
    selected: &[V],
    thing_14: F,
    kingdom_4c: Option<ActorId>,
    group_40_byte_18: Option<bool>,
) -> Result<[u32; 4]>
where
    V: Copy + Eq + Debug,
    F: FnMut(V) -> Result<Option<ActorId>>,
{
    let bank = ReadBank::new(parts.definitions);
    let mut live = Live::new_read(catalog, &bank, parts.actors, parts.spatial, globals, local_kingdom_5c0e30, thing_14);
    kernel::command_masks_437eb0(&mut live, selected, kingdom_4c, group_40_byte_18)
}

/// The same 0x437EB0 inside a frame (`world.borrow_frame()
/// .with_gameplay_order(|parts| ...)`), e.g. the simulation notification
/// 0x50C8E0.
#[allow(clippy::too_many_arguments)]
pub fn command_masks_in_frame<V, F>(
    catalog: &Catalog<'_>,
    parts: GameplayOrderLoans<'_>,
    globals: &mut impl Globals,
    local_kingdom_5c0e30: Result<Option<ActorId>>,
    selected: &[V],
    thing_14: F,
    kingdom_4c: Option<ActorId>,
    group_40_byte_18: Option<bool>,
) -> Result<[u32; 4]>
where
    V: Copy + Eq + Debug,
    F: FnMut(V) -> Result<Option<ActorId>>,
{
    let bank = DefinitionBankLoan::new(parts.definitions);
    let (_, actors) = parts.actors.split_minds();
    let mut live = Live::new(catalog, &bank, &actors, &*parts.spatial, globals, local_kingdom_5c0e30, thing_14);
    kernel::command_masks_437eb0(&mut live, selected, kingdom_4c, group_40_byte_18)
}

/// Actor vt+0x154(kingdom, index, selected) = 0x4CE680 on one live actor
/// between frames (for callers other than 0x437EB0).
#[allow(clippy::too_many_arguments)]
pub fn virtual_154_between_frames(
    catalog: &Catalog<'_>,
    parts: &PresentationParts<'_>,
    globals: &mut impl Globals,
    actor: ActorId,
    kingdom: Option<ActorId>,
    index: u32,
    selected: u8,
) -> Result<bool> {
    let bank = ReadBank::new(parts.definitions);
    let mut live = Live::new_read(catalog, &bank, parts.actors, parts.spatial, globals, Ok(None),
        |_: ()| Err::<Option<ActorId>, String>("no view on the vt+0x154 path".into()));
    MaskLeaves::virtual_154(&mut live, actor, kingdom, index, selected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player_order_world::Result as OrderResult;
    use serde_json::Value;

    struct NoVariables;
    impl Globals for NoVariables {
        fn can_process_40eaa0(&mut self) -> OrderResult<bool> {
            Err(orders::Error::Required("40EAA0".into()))
        }
        fn session_phase_76ac90(&mut self) -> OrderResult<u32> {
            Err(orders::Error::Required("76AC90".into()))
        }
    }
    struct NoSpatial;
    impl Spatial for NoSpatial {
        fn placement(&self, _: ActorId) -> crate::actor_world::Result<crate::actor_world::PlacementState> {
            Err(crate::actor_world::Error::Missing("test spatial"))
        }
        fn publish(&mut self, _: ActorId, _: crate::actor_world::PlacementState) -> crate::actor_world::Result<()> {
            Err(crate::actor_world::Error::Missing("test spatial"))
        }
    }

    /// The live definitions read the actual def+0x3C (non_group_command) of
    /// all 39 commands; a selected view naming no live Thing is refused at
    /// its first read; an empty selection reads nothing.
    #[test]
    fn live_command_mask_leaves_bind_actual_definitions() {
        use crate::{combat_source_completion as completion, combat_source_receipt::VerifiedStartupReceipt, settings::Settings};
        use aa_rules::{actors::ActorCatalog, sources::{SourceLimits, V14Sources}, tgi::Limits};
        let Some(game) = std::env::var_os("AA_GAME_ROOT").map(std::path::PathBuf::from) else {
            eprintln!("AA_GAME_ROOT unset: live command-mask binding not exercised");
            return;
        };
        let mut sources = V14Sources::open(
            game.join("Data.rwd"),
            game.join("reconstruction/native-game/runtime-expanded-v14"),
            SourceLimits { max_member_bytes: 64 * 1024 * 1024 },
        )
        .unwrap();
        let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
        let actors = ActorCatalog::load_v14(&mut sources).unwrap();
        let mut catalog = Catalog::new(&actors);
        catalog.settings = Some(settings);
        let receipt = VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut sources).unwrap();
        completion::install(&catalog, &mut sources, receipt.context(), receipt.csv(), receipt.globals()).unwrap();
        completion::compute(&catalog).unwrap();
        let fixture: Value = serde_json::from_slice(
            &std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../research/phase8-orders-input/command-masks/native-command-masks.json"))
            .unwrap(),
        )
        .unwrap();
        let mut store = crate::actor_world::ActorStore::new();
        let mut globals = NoVariables;
        {
            let mut live = Live::with_banks(&catalog, &catalog.definition_bank, &store, &NoSpatial, &mut globals,
                Err("5C0E30 unbound".into()), |v: u32| Ok(Some(ActorId::from_raw(v))));
            for (i, d) in fixture["definitions_actual"].as_array().unwrap().iter().enumerate() {
                let native = d["b3c"].as_u64().unwrap() != 0;
                assert_eq!(live.non_group_3c(i as u32).unwrap(), Some(native), "{}", d["ids"]);
            }
            assert_eq!(kernel::command_masks_437eb0(&mut live, &[], None, None).unwrap(), [0; 4]);
            let error = kernel::command_masks_437eb0(&mut live, &[0x0001_0005], None, None).unwrap_err();
            assert!(error.contains("current generation"), "{error}");
        }
        let (_, access) = store.split_minds();
        let mut live = Live::with_banks(&catalog, &catalog.definition_bank, &access, &NoSpatial, &mut globals,
            Ok(None), |_: ()| Ok(None));
        let error = MaskLeaves::virtual_154(&mut live, ActorId::from_raw(0x0001_0005), None, 5, 1).unwrap_err();
        assert!(error.contains("current generation"), "{error}");
    }
}
