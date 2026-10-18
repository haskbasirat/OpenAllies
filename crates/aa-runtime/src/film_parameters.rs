//!Current header parameter storage, distinct from the live World and Creator.
//!50F890 constructs empty active counts;510EB0..511020 captures Teams in the
//!actual World+16C slot order, preserving both localized CString words.
use crate::{
    actor_world::{ActorStore, Source, SpecialSource},
    catalog::Catalog,
    creator_labels::LocalizedPairRef,
    definitions::Bank,
    live_registration,
};
use aa_actors::{ActorId, loading::LoadKind};
use aa_mapworld::bootstrap::DefinitionProof;
pub type Result<T> = std::result::Result<T, String>;
struct Team {
    id: Vec<u16>,
    key: Vec<u16>,
    text: Vec<u16>,
}
pub struct TeamRef<'a> {
    team: &'a Team,
}
impl<'a> TeamRef<'a> {
    pub fn id(&self) -> &'a [u16] {
        &self.team.id
    }
    pub fn display(&self) -> LocalizedPairRef<'a> {
        LocalizedPairRef::new(&self.team.key, &self.team.text)
    }
    pub fn display_key(&self) -> &'a [u16] {
        &self.team.key
    }
    pub fn display_text(&self) -> &'a [u16] {
        &self.team.text
    }
}
///No Clone/Default/raw public setter. Move this SAME owner into the fresh
///header. Native capacity8 is a storage hint, not a new gameplay limit.
pub struct State {
    teams: Table,
    kingdoms: crate::film_kingdom_parameters::Table<crate::film_kingdom_parameters::Definition>,
}
impl State {
    ///50F890 active Team/Kingdom counts are both zero. This constructor does
    ///not certify that later510EB0 current Kingdom capture has completed.
    pub fn from_native_constructor() -> Self {
        Self {
            teams: Table::from_native_constructor(),
            kingdoms: crate::film_kingdom_parameters::Table::from_native_constructor(),
        }
    }
    pub fn teams(&self) -> impl ExactSizeIterator<Item = TeamRef<'_>> {
        self.teams.teams()
    }
    pub fn kingdom_count(&self) -> usize {
        self.kingdoms.len()
    }
    pub fn kingdoms(
        &self,
    ) -> impl ExactSizeIterator<
        Item = &crate::film_kingdom_parameters::Captured<
            crate::film_kingdom_parameters::Definition,
        >,
    > {
        self.kingdoms.iter()
    }
    ///Native511020..511297; later trigger extras are a separate required
    ///phase. The same current localization table retains49AE00 writes on error.
    pub fn capture_kingdoms<G: crate::film_kingdom_parameters::Glyph>(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        actors: &ActorStore,
        registry: &crate::load_kingdom_init_world::KingdomRegistryState,
        text: &mut crate::localization::Localization,
        glyph: &mut G,
    ) -> Result<()> {
        crate::film_kingdom_parameters::capture_live(
            &mut self.kingdoms,
            catalog,
            bank,
            actors,
            registry,
            text,
            glyph,
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub fn capture_kingdoms_current<P: crate::load_kingdom_glyph::Players + ?Sized>(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        actors: &ActorStore,
        registry: &crate::load_kingdom_init_world::KingdomRegistryState,
        text: &mut crate::localization::Localization,
        formatter: &mut crate::load_kingdom_glyph::Formatter,
        variables: &mut crate::player_glyph::State,
        players: &mut P,
    ) -> Result<()> {
        crate::film_kingdom_parameters::capture_live_current(
            &mut self.kingdoms,
            catalog,
            bank,
            actors,
            registry,
            text,
            formatter,
            variables,
            players,
        )
    }
    pub fn capture_teams(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        actors: &ActorStore,
        registration: &live_registration::State,
    ) -> Result<()> {
        capture_team_fields(
            &mut self.teams,
            &LiveTeams {
                catalog,
                bank,
                actors,
                registration,
            },
        )
    }
}
impl crate::film_header_entries::ParameterTarget for State {
    fn find_50dc00(&self, name: &[u16]) -> Option<usize> {
        self.kingdoms.find_50dc00(name)
    }
    fn assign_profile_2c(&mut self, index: usize, profile: Vec<u16>) -> Result<()> {
        self.kingdoms.assign_profile_2c(index, profile)
    }
}
impl crate::film_parameter_extras::Target for State {
    fn find_50dc00(&self, name: &[u16]) -> Option<usize> {
        self.kingdoms.find_50dc00(name)
    }
    fn append_510150(&mut self, index: usize, key: Vec<u16>, text: Vec<u16>) -> Result<()> {
        self.kingdoms.append_510150(index, key, text)
    }
}
///Explicit-input native kernel table. It cannot be converted or installed
///into an authoritative State; only State's concrete reader can mutate that
///owner. There is no second table or final copyback in the live path.
pub struct Table {
    teams: Vec<Team>,
}
impl Table {
    pub fn from_native_constructor() -> Self {
        Self { teams: Vec::new() }
    }
    pub fn teams(&self) -> impl ExactSizeIterator<Item = TeamRef<'_>> {
        self.teams.iter().map(|team| TeamRef { team })
    }
}
///Closed readonly source loans: the native Team graph has no outward World
///mutation callback. Captured values are native CString value copies, not a
///retained snapshot used as live Actor authority.
pub trait TeamFields {
    type Team: Copy;
    fn slots(&self) -> Result<&[Option<Self::Team>]>;
    fn name_18(&self, team: Self::Team) -> Result<Vec<u16>>;
    fn key_3c(&self, team: Self::Team) -> Result<Vec<u16>>;
    fn text_40(&self, team: Self::Team) -> Result<Vec<u16>>;
}
pub fn capture_team_fields<S: TeamFields>(target: &mut Table, source: &S) -> Result<()> {
    target.teams.clear(); //510EE8, before50D6F0 first slot access.
    for &team in source.slots()?.iter().flatten() {
        let id = source.name_18(team)?;
        let key = source.key_3c(team)?;
        let text = source.text_40(team)?;
        target.teams.push(Team { id, key, text }); //Real510250 append, no sorting/dedup.
    }
    Ok(())
}
struct LiveTeams<'a, 'source> {
    catalog: &'a Catalog<'source>,
    bank: &'a Bank,
    actors: &'a ActorStore,
    registration: &'a live_registration::State,
}
fn same(a: &DefinitionProof, b: &DefinitionProof) -> bool {
    a.id == b.id
        && a.source == b.source
        && a.template_chain == b.template_chain
        && a.category == b.category
        && a.base_flag_inputs == b.base_flag_inputs
}
impl LiveTeams<'_, '_> {
    fn team(
        &self,
        id: ActorId,
    ) -> Result<(
        &crate::actor_world::RuntimeObject,
        &aa_mapworld::special_state::SpecialShell,
    )> {
        self.catalog.require_source_owner()?;
        let source = self.bank.combat_source()?;
        source.completed_means()?;
        if source.source_identities() != self.catalog.actors.sources {
            return Err("film Team source Bank/catalog differ".into());
        }
        let key = self
            .actors
            .key(id)
            .ok_or("film Team generation is absent")?;
        let entity = self.actors.entity(key).map_err(|e| e.to_string())?;
        let Source::Special(special) = &entity.source else {
            return Err("film Team is not a native special allocation".into());
        };
        if !matches!(&special.source, SpecialSource::Team(saved) if saved.serialized_id==id.raw())
            || !special.derived_payload_loaded
        {
            return Err("film Team derived load is incomplete or wrong class".into());
        }
        let runtime = self.actors.object(key).map_err(|e| e.to_string())?;
        let proof = &runtime.thing.definition;
        let resolved = aa_mapworld::bootstrap::resolve_definition(
            self.catalog.actors,
            &proof.id.encode_utf16().collect::<Vec<_>>(),
            source.source_precision(),
        )
        .map_err(|e| e.to_string())?;
        if runtime.kind != LoadKind::Thing
            || runtime.object.is_some()
            || runtime.actor.is_some()
            || runtime.definition.native_class != 5
            || resolved.native_class != 5
            || proof.category.native_kind != 0
            || !same(proof, &runtime.definition.proof)
            || !same(proof, &resolved.definition)
        {
            return Err("film Team actual constructor/source certificate differs".into());
        }
        Ok((
            runtime,
            special
                .derived
                .as_ref()
                .ok_or("film Team derived storage is absent")?,
        ))
    }
}
impl TeamFields for LiveTeams<'_, '_> {
    type Team = ActorId;
    fn slots(&self) -> Result<&[Option<ActorId>]> {
        Ok(self.registration.teams())
    }
    fn name_18(&self, id: ActorId) -> Result<Vec<u16>> {
        Ok(self.team(id)?.0.thing.name_utf16.clone())
    }
    fn key_3c(&self, id: ActorId) -> Result<Vec<u16>> {
        self.team(id)?
            .1
            .strings_utf16
            .get(&0x3c)
            .cloned()
            .ok_or("film Team current localized key3C is absent".into())
    }
    fn text_40(&self, id: ActorId) -> Result<Vec<u16>> {
        self.team(id)?
            .1
            .strings_utf16
            .get(&0x40)
            .cloned()
            .ok_or("film Team current localized text40 is absent".into())
    }
}
