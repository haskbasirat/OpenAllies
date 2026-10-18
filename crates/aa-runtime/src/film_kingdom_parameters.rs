//!510EB0..511297 current Kingdom capture. The target is an independent native
//!value-copy destination, never a substitute for live Kingdom/World storage.
//!4E93C0 remains a required child; the later trigger-extra phase is separate.
use crate::{
    actor_world::{ActorStore, Source, SpecialSource},
    catalog::Catalog,
    creator_labels::LocalizedPairRef,
    definitions::Bank,
    load_kingdom_init_world::KingdomRegistryState,
    localization::Localization,
};
use aa_actors::{ActorId, loading::LoadKind};
use aa_mapworld::{
    bootstrap::DefinitionProof, restore::NamedDefinition, special_state::SpecialShell,
};
pub type Result<T> = std::result::Result<T, String>;

///Explicit-input kernel record. Its generic definition handles are native
///pointer value copies. An arbitrary record cannot be installed into State.
pub struct Captured<D> {
    pub id: Vec<u16>,
    pub key: Vec<u16>,
    pub text: Vec<u16>,
    pub glyph: Vec<u16>,
    pub minor: u8,
    pub team: Vec<u16>,
    pub nation: Option<D>,
    pub faction: Option<D>,
    pub color: Option<D>,
    pub handicap: Option<D>,
    pub eligible: u8,
    pub allow: u8,
    pub profile: Vec<u16>,
    pub byte30: u8,
    pub byte31: u8,
    pub byte32: u8,
    pub resource_bits: Vec<u32>,
    pub localized_entries: Vec<(Vec<u16>, Vec<u16>)>,
    pub definitions: Vec<D>,
    pub string58: Vec<u16>,
    pub properties: Vec<Vec<u16>>,
    pub byte68: u8,
}
impl<D> Captured<D> {
    fn constructor(resource_count: usize) -> Result<Self> {
        //50F4C0(1): all strings empty, native scalar/reference constructors,
        //then45F180 zeros exactly the actual Resource registry count.
        let mut resource_bits = Vec::new();
        resource_bits
            .try_reserve_exact(resource_count)
            .map_err(|e| e.to_string())?;
        resource_bits.resize(resource_count, 0);
        Ok(Self {
            id: Vec::new(),
            key: Vec::new(),
            text: Vec::new(),
            glyph: Vec::new(),
            minor: 1,
            team: Vec::new(),
            nation: None,
            faction: None,
            color: None,
            handicap: None,
            eligible: 1,
            allow: 1,
            profile: Vec::new(),
            byte30: 0,
            byte31: 0,
            byte32: 1,
            resource_bits,
            localized_entries: Vec::new(),
            definitions: Vec::new(),
            string58: Vec::new(),
            properties: Vec::new(),
            byte68: 0,
        })
    }
    pub fn display(&self) -> LocalizedPairRef<'_> {
        LocalizedPairRef::new(&self.key, &self.text)
    }
}
///Diagnostic explicit-input table. No public conversion/append/raw setter
///permits using it to populate the authoritative film_parameters::State.
pub struct Table<D> {
    rows: Vec<Captured<D>>,
}
impl<D> Table<D> {
    pub fn from_native_constructor() -> Self {
        Self { rows: Vec::new() }
    }
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &Captured<D>> {
        self.rows.iter()
    }
    pub fn len(&self) -> usize {
        self.rows.len()
    }
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    pub(crate) fn find_50dc00(&self, name: &[u16]) -> Option<usize> {
        let prefix = |s: &[u16]| s.iter().position(|&u| u == 0).unwrap_or(s.len());
        self.rows
            .iter()
            .position(|r| r.id[..prefix(&r.id)] == name[..prefix(name)])
    }
    pub(crate) fn assign_profile_2c(&mut self, index: usize, profile: Vec<u16>) -> Result<()> {
        self.rows
            .get_mut(index)
            .ok_or("5A17D0 captured Kingdom parameter is absent")?
            .profile = profile;
        Ok(())
    }
    pub(crate) fn append_510150(
        &mut self,
        index: usize,
        key: Vec<u16>,
        text: Vec<u16>,
    ) -> Result<()> {
        self.rows
            .get_mut(index)
            .ok_or("film objective extra destination is absent")?
            .localized_entries
            .push((key, text));
        Ok(())
    }
}
///Lazy native reads/children. first/next preserve the actual iterator owner
///and slot, including repeated actor values. Services may fail after earlier
///localized table effects; no destination transaction or rollback is used.
pub trait Fields {
    type Kingdom: Copy;
    type Team: Copy;
    type Definition;
    fn first(&mut self) -> Result<Option<Self::Kingdom>>;
    fn next(&mut self, current: Self::Kingdom) -> Result<Option<Self::Kingdom>>;
    fn resource_count(&mut self) -> Result<usize>;
    fn id_18(&mut self, k: Self::Kingdom) -> Result<Vec<u16>>;
    fn key_194(&mut self, k: Self::Kingdom) -> Result<Vec<u16>>;
    fn text_198(&mut self, k: Self::Kingdom) -> Result<Vec<u16>>;
    fn name_24(&mut self, k: Self::Kingdom) -> Result<Vec<u16>>;
    fn set_name_49ae00(&mut self, key: &[u16], text: &mut Vec<u16>, supplied: &[u16])
    -> Result<()>;
    fn glyph_4e93c0(&mut self, k: Self::Kingdom) -> Result<Vec<u16>>;
    fn word_190(&mut self, k: Self::Kingdom) -> Result<u32>;
    fn team_1ec(&mut self, k: Self::Kingdom) -> Result<Option<Self::Team>>;
    fn team_name_18(&mut self, team: Self::Team) -> Result<Vec<u16>>;
    fn nation_234(&mut self, k: Self::Kingdom) -> Result<Option<Self::Definition>>;
    fn faction_230_4(&mut self, k: Self::Kingdom) -> Result<Option<Self::Definition>>;
    fn color_1e8(&mut self, k: Self::Kingdom) -> Result<Option<Self::Definition>>;
    fn handicap_244(&mut self, k: Self::Kingdom) -> Result<Option<Self::Definition>>;
    fn byte_23b(&mut self, k: Self::Kingdom) -> Result<u8>;
    fn byte_23c(&mut self, k: Self::Kingdom) -> Result<u8>;
    fn word_19c(&mut self, k: Self::Kingdom) -> Result<u32>;
    fn string_240(&mut self, k: Self::Kingdom) -> Result<Vec<u16>>;
    fn byte_23d(&mut self, k: Self::Kingdom) -> Result<u8>;
}
pub fn capture_fields<S: Fields>(target: &mut Table<S::Definition>, source: &mut S) -> Result<()> {
    target.rows.clear(); //511031 before4F0680.
    let mut next = source.first()?;
    while let Some(k) = next {
        let mut row = Captured::constructor(source.resource_count()?)?;
        row.id = source.id_18(k)?;
        row.key = source.key_194(k)?;
        row.text = source.text_198(k)?;
        let current_name = source.name_24(k)?;
        source.set_name_49ae00(&row.key, &mut row.text, &current_name)?;
        row.glyph = source.glyph_4e93c0(k)?;
        row.minor = u8::from(source.word_190(k)? != 0);
        if let Some(team) = source.team_1ec(k)? {
            row.team = source.team_name_18(team)?;
        }
        row.nation = source.nation_234(k)?;
        row.faction = source.faction_230_4(k)?;
        row.color = source.color_1e8(k)?;
        row.handicap = source.handicap_244(k)?;
        row.allow = source.byte_23b(k)?;
        row.eligible = if source.byte_23c(k)? != 0 && source.word_19c(k)? != 2 {
            1
        } else {
            0
        };
        row.profile = source.string_240(k)?;
        row.localized_entries.clear(); //511237 resets+44; no earlier append exists.
        row.byte32 = source.byte_23d(k)?; //+32, not+31: verified stack base3C.
        let count = source.resource_count()?;
        row.resource_bits
            .try_reserve_exact(count.saturating_sub(row.resource_bits.len()))
            .map_err(|e| e.to_string())?;
        row.resource_bits.resize(count, 0);
        row.resource_bits.fill(0);
        target.rows.push(row); //4A4DC0 value copy; capacity8 is a growth hint.
        next = source.next(k)?;
    }
    Ok(())
}

///Immutable typed source identity captured from the actual reference. No
///public raw constructor; an IDS or registry ordinal alone cannot mint it.
pub struct Definition {
    domain: &'static str,
    id: Vec<u16>,
    source: aa_rules::resources::Provenance,
}
impl Definition {
    pub fn domain(&self) -> &'static str {
        self.domain
    }
    pub fn ids(&self) -> &[u16] {
        &self.id
    }
    pub fn source(&self) -> &aa_rules::resources::Provenance {
        &self.source
    }
}
///Concrete glyph providers must borrow the real Session/palette/current
///AVars. No default implementation or all-major-empty shortcut is supplied.
pub trait Glyph {
    fn glyph_4e93c0(&mut self, kingdom: ActorId) -> Result<Vec<u16>>;
}
struct Live<'a, 'source, G> {
    catalog: &'a Catalog<'source>,
    bank: &'a Bank,
    actors: &'a ActorStore,
    registry: &'a KingdomRegistryState,
    text: &'a mut Localization,
    glyph: &'a mut G,
}
fn same(a: &DefinitionProof, b: &DefinitionProof) -> bool {
    a.id == b.id
        && a.source == b.source
        && a.template_chain == b.template_chain
        && a.category == b.category
        && a.base_flag_inputs == b.base_flag_inputs
}
impl<G> Live<'_, '_, G> {
    fn source_precision(&self) -> Result<aa_numeric::Precision> {
        self.catalog.require_source_owner()?;
        let source = self.bank.combat_source()?;
        source.completed_means()?;
        if source.source_identities() != self.catalog.actors.sources {
            return Err("film Kingdom Bank/source Catalog differ".into());
        }
        Ok(source.source_precision())
    }
    fn special(
        &self,
        id: ActorId,
        kingdom: bool,
    ) -> Result<(&crate::actor_world::RuntimeObject, &SpecialShell)> {
        let pc = self.source_precision()?;
        let key = self
            .actors
            .key(id)
            .ok_or("film Kingdom/Team generation is absent")?;
        let entity = self.actors.entity(key).map_err(|e| e.to_string())?;
        let Source::Special(s) = &entity.source else {
            return Err("film Kingdom/Team source is not actual special storage".into());
        };
        let correct = match &s.source {
            SpecialSource::Kingdom(saved) => kingdom && saved.serialized_id == id.raw(),
            SpecialSource::Team(saved) => !kingdom && saved.serialized_id == id.raw(),
        };
        if !correct || !s.derived_payload_loaded {
            return Err("film Kingdom/Team derived allocation is incomplete or wrong class".into());
        }
        let r = self.actors.object(key).map_err(|e| e.to_string())?;
        let proof = &r.thing.definition;
        let resolved = aa_mapworld::bootstrap::resolve_definition(
            self.catalog.actors,
            &proof.id.encode_utf16().collect::<Vec<_>>(),
            pc,
        )
        .map_err(|e| e.to_string())?;
        let class = if kingdom { 4 } else { 5 };
        let kind = if kingdom { 2 } else { 0 };
        if r.kind
            != if kingdom {
                LoadKind::Actor
            } else {
                LoadKind::Thing
            }
            || r.definition.native_class != class
            || resolved.native_class != class
            || proof.category.native_kind != kind
            || r.actor.is_some() != kingdom
            || r.object.is_some() != kingdom
            || !same(proof, &r.definition.proof)
            || !same(proof, &resolved.definition)
        {
            return Err("film Kingdom/Team constructor/source certificate differs".into());
        }
        Ok((
            r,
            s.derived
                .as_ref()
                .ok_or("film Kingdom/Team current derived storage is absent")?,
        ))
    }
    fn shell(&self, k: (usize, ActorId)) -> Result<&SpecialShell> {
        Ok(self.special(k.1, true)?.1)
    }
    fn word(&self, k: (usize, ActorId), o: u32) -> Result<u32> {
        self.shell(k)?
            .words
            .get(&o)
            .copied()
            .ok_or_else(|| format!("film current Kingdom+{o:X} is unbound"))
    }
    fn byte(&self, k: (usize, ActorId), o: u32) -> Result<u8> {
        let s = self.shell(k)?;
        s.bytes
            .get(&o)
            .copied()
            .or_else(|| {
                s.words
                    .get(&(o & !3))
                    .map(|w| ((w >> ((o & 3) * 8)) & 255) as u8)
            })
            .ok_or_else(|| format!("film current Kingdom byte+{o:X} is unbound"))
    }
    fn string(&self, k: (usize, ActorId), o: u32) -> Result<Vec<u16>> {
        self.shell(k)?
            .strings_utf16
            .get(&o)
            .cloned()
            .ok_or_else(|| format!("film current Kingdom string+{o:X} is unbound"))
    }
    fn definition(&self, domain: &'static str, named: &NamedDefinition) -> Result<Definition> {
        self.source_precision()?;
        let registry = self
            .catalog
            .actors
            .native_data_registry(domain)
            .ok_or("film captured source registry is absent")?;
        let mut matches = registry
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.ids.eq_ignore_ascii_case(&named.id));
        let (index, entry) = matches
            .next()
            .ok_or("film captured source reference is foreign")?;
        if matches.next().is_some()
            || !entry.reloads.is_empty()
            || named.index.is_some_and(|n| n as usize != index)
        {
            return Err(
                "film captured source identity is ambiguous/reloaded or has a foreign ordinal"
                    .into(),
            );
        }
        Ok(Definition {
            domain,
            id: entry.ids.encode_utf16().collect(),
            source: entry.source.clone(),
        })
    }
    fn reference(
        &self,
        k: (usize, ActorId),
        o: u32,
        domain: &'static str,
    ) -> Result<Option<Definition>> {
        let s = self.shell(k)?;
        match s.references.get(&o) {
            Some(Some(named)) => Ok(Some(self.definition(domain, named)?)),
            Some(None) => Ok(None),
            None if s.words.get(&o) == Some(&0) => Ok(None),
            _ => Err(format!("film current Kingdom reference+{o:X} is unbound")),
        }
    }
    fn slot_from(&self, from: usize) -> Option<(usize, ActorId)> {
        self.registry
            .all()
            .iter()
            .enumerate()
            .skip(from)
            .find_map(|(i, id)| id.map(|id| (i, id)))
    }
}
impl<G: Glyph> Fields for Live<'_, '_, G> {
    type Kingdom = (usize, ActorId);
    type Team = ActorId;
    type Definition = Definition;
    fn first(&mut self) -> Result<Option<Self::Kingdom>> {
        Ok(self.slot_from(0))
    }
    fn next(&mut self, k: Self::Kingdom) -> Result<Option<Self::Kingdom>> {
        Ok(self.slot_from(k.0.checked_add(1).ok_or("Kingdom iterator overflow")?))
    }
    fn resource_count(&mut self) -> Result<usize> {
        Ok(self
            .catalog
            .actors
            .native_resource_registry(self.source_precision()?)
            .map_err(|e| e.to_string())?
            .ids
            .len())
    }
    fn id_18(&mut self, k: Self::Kingdom) -> Result<Vec<u16>> {
        Ok(self.special(k.1, true)?.0.thing.name_utf16.clone())
    }
    fn key_194(&mut self, k: Self::Kingdom) -> Result<Vec<u16>> {
        self.string(k, 0x194)
    }
    fn text_198(&mut self, k: Self::Kingdom) -> Result<Vec<u16>> {
        self.string(k, 0x198)
    }
    fn name_24(&mut self, k: Self::Kingdom) -> Result<Vec<u16>> {
        self.string(k, 0x198)
    } //actual4E5340.
    fn set_name_49ae00(
        &mut self,
        key: &[u16],
        text: &mut Vec<u16>,
        supplied: &[u16],
    ) -> Result<()> {
        crate::live_map_strings::set_existing_pair(self.text, key, text, supplied)
    }
    fn glyph_4e93c0(&mut self, k: Self::Kingdom) -> Result<Vec<u16>> {
        self.glyph.glyph_4e93c0(k.1)
    }
    fn word_190(&mut self, k: Self::Kingdom) -> Result<u32> {
        self.word(k, 0x190)
    }
    fn team_1ec(&mut self, k: Self::Kingdom) -> Result<Option<ActorId>> {
        let s = self.shell(k)?;
        match s.identity_references.get(&0x1ec) {
            Some(v) => Ok(*v),
            None if s.words.get(&0x1ec) == Some(&0) => Ok(None),
            _ => Err("film current Kingdom Team reference is unbound".into()),
        }
    }
    fn team_name_18(&mut self, t: ActorId) -> Result<Vec<u16>> {
        Ok(self.special(t, false)?.0.thing.name_utf16.clone())
    }
    fn nation_234(&mut self, k: Self::Kingdom) -> Result<Option<Definition>> {
        self.reference(k, 0x234, "Nation")
    }
    fn faction_230_4(&mut self, k: Self::Kingdom) -> Result<Option<Definition>> {
        let s = self.shell(k)?;
        let Some(commander) = &s.commander else {
            return if s.words.get(&0x230) == Some(&0) || !s.words.contains_key(&0x230) {
                Ok(None)
            } else {
                Err("film Commander owner is unbound".into())
            };
        };
        let entry = self.bank.entry(commander.definition)?;
        if !entry.key.domain.eq_ignore_ascii_case("Faction") {
            return Err("film Commander definition is not actual Faction".into());
        }
        Ok(Some(self.definition(
            "Faction",
            &NamedDefinition {
                id: entry.key.name.clone(),
                index: None,
            },
        )?))
    }
    fn color_1e8(&mut self, k: Self::Kingdom) -> Result<Option<Definition>> {
        self.reference(k, 0x1e8, "KingdomColor")
    }
    fn handicap_244(&mut self, k: Self::Kingdom) -> Result<Option<Definition>> {
        self.reference(k, 0x244, "Handicap")
    }
    fn byte_23b(&mut self, k: Self::Kingdom) -> Result<u8> {
        self.byte(k, 0x23b)
    }
    fn byte_23c(&mut self, k: Self::Kingdom) -> Result<u8> {
        self.byte(k, 0x23c)
    }
    fn word_19c(&mut self, k: Self::Kingdom) -> Result<u32> {
        self.word(k, 0x19c)
    }
    fn string_240(&mut self, k: Self::Kingdom) -> Result<Vec<u16>> {
        self.string(k, 0x240)
    }
    fn byte_23d(&mut self, k: Self::Kingdom) -> Result<u8> {
        self.byte(k, 0x23d)
    }
}
pub(crate) fn capture_live<G: Glyph>(
    target: &mut Table<Definition>,
    catalog: &Catalog<'_>,
    bank: &Bank,
    actors: &ActorStore,
    registry: &KingdomRegistryState,
    text: &mut Localization,
    glyph: &mut G,
) -> Result<()> {
    capture_fields(
        target,
        &mut Live {
            catalog,
            bank,
            actors,
            registry,
            text,
            glyph,
        },
    )
}
///Complete concrete4E93C0 child over the persistent application Formatter,
///current glyph-variable owner and real Session roster loan. No fresh cache
///is constructed inside the per-header capture.
#[allow(clippy::too_many_arguments)]
pub(crate) fn capture_live_current<P: crate::load_kingdom_glyph::Players + ?Sized>(
    target: &mut Table<Definition>,
    catalog: &Catalog<'_>,
    bank: &Bank,
    actors: &ActorStore,
    registry: &KingdomRegistryState,
    text: &mut Localization,
    formatter: &mut crate::load_kingdom_glyph::Formatter,
    variables: &mut crate::player_glyph::State,
    players: &mut P,
) -> Result<()> {
    struct Current<'a, 'source, P: ?Sized> {
        catalog: &'a Catalog<'source>,
        bank: &'a Bank,
        actors: &'a ActorStore,
        formatter: &'a mut crate::load_kingdom_glyph::Formatter,
        variables: &'a mut crate::player_glyph::State,
        players: &'a mut P,
    }
    impl<P: crate::load_kingdom_glyph::Players + ?Sized> Glyph for Current<'_, '_, P> {
        fn glyph_4e93c0(&mut self, id: ActorId) -> Result<Vec<u16>> {
            self.formatter.display(
                self.catalog,
                self.bank,
                self.actors,
                id,
                self.variables,
                self.players,
            )
        }
    }
    capture_live(
        target,
        catalog,
        bank,
        actors,
        registry,
        text,
        &mut Current {
            catalog,
            bank,
            actors,
            formatter,
            variables,
            players,
        },
    )
}
