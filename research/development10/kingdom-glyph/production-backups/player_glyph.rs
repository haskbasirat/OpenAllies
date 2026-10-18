//! Native7666C0 player glyph selection and the current application variables.
//! Keep this process owner once. It supplies no mission/readiness capability.
//! Online account/PeerLink queries remain required real borrowed services.
use crate::{catalog::Catalog, definitions::Bank, load_view_variables::NativeOverrides};
use aa_rules::{
    native::TypedValue,
    native::index::native_enumerate,
    resources::{Provenance, SourceIdentity},
    sources::V14Sources,
    tgi::{Document, Item, Limits},
};

#[cfg(test)]
#[path = "player_glyph_tests.rs"]
mod tests;

type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Copy)]
enum Slot {
    Kingdom,
    Observer,
}
struct Variable {
    value: Vec<u16>,
    assignments: Vec<Provenance>,
}

/// Actual registered GlyphData+18 projection. No raw-index constructor exists.
pub struct Glyph {
    ids: String,
    source: Provenance,
    index: u16,
    universe: Vec<SourceIdentity>,
}
impl Glyph {
    pub fn ids(&self) -> &str {
        &self.ids
    }
    pub fn source(&self) -> &Provenance {
        &self.source
    }
    pub fn index(&self) -> u16 {
        self.index
    }

    ///4E7DC0/48D500 canonical lookup;493500 stores the low16 GetInt(index)
    ///word. This read needs the actual moved Bank's source precision.
    pub fn find(catalog: &Catalog<'_>, bank: &Bank, ids: &[u16]) -> Result<Option<Self>> {
        catalog.require_source_owner()?;
        let source = bank.combat_source()?;
        source.completed_means()?;
        if source.source_identities() != catalog.actors.sources {
            return Err("player glyph Bank/source owner differs".into());
        }
        let end = ids.iter().position(|&v| v == 0).unwrap_or(ids.len());
        if end == 0 {
            return Ok(None);
        }
        let name = String::from_utf16(&ids[..end]).map_err(|e| e.to_string())?;
        let registry = catalog
            .actors
            .native_data_registry("Glyph")
            .ok_or("Glyph DataManager+3BC is unbound")?;
        let entry = registry
            .find(&name)
            .ok_or_else(|| format!("Unknown IDS '{name}' for Glyph DataManager"))?;
        let node = catalog
            .actors
            .native_data_node(entry)
            .map_err(|e| e.to_string())?;
        let Some(TypedValue::Integer { value, .. }) = node
            .field("index", source.source_precision())
            .map_err(|e| e.to_string())?
        else {
            return Err("Glyph493500 index requires the actual typed integer getter value".into());
        };
        Ok(Some(Self {
            ids: entry.ids.clone(),
            source: entry.source.clone(),
            index: value as u16,
            universe: catalog.actors.sources.clone(),
        }))
    }
}

/// Actual player/Session/application siblings are read at each reached point.
/// In particular, absence of an online-manager producer is an error here,
/// never an assumed null singleton or a fabricated empty account glyph.
pub trait PlayerRead {
    type Player;
    fn ai_flag_0c(&mut self, player: &Self::Player) -> Result<bool>;
    fn online_manager_present(&mut self) -> Result<bool>;
    fn peer_present_04(&mut self, player: &Self::Player) -> Result<bool>;
    ///Actual792DC0 over the same PeerLink+7C and real account/level owners.
    fn online_peer_glyph(
        &mut self,
        player: &Self::Player,
        catalog: &Catalog<'_>,
        bank: &Bank,
    ) -> Result<Option<Glyph>>;
    fn session_state_e4(&mut self) -> Result<u32>;
    ///Only after the genuine765E10 writer; constructor-null is not that proof.
    fn kingdom_present_28(&mut self, player: &Self::Player) -> Result<bool>;
}

struct Cache<V> {
    flags: u32,
    kingdom: Option<V>,
    observer: Option<V>,
}
trait World {
    type Variable: Copy;
    type Glyph;
    fn lookup_string(&mut self, slot: Slot) -> Result<Self::Variable>;
    fn ai(&mut self) -> Result<bool>;
    fn online_manager(&mut self) -> Result<bool>;
    fn peer(&mut self) -> Result<bool>;
    fn online_glyph(&mut self) -> Result<Option<Self::Glyph>>;
    fn session(&mut self) -> Result<u32>;
    fn kingdom(&mut self) -> Result<bool>;
    fn variable_value(&mut self, variable: Self::Variable) -> Result<Vec<u16>>;
    fn find(&mut self, ids: &[u16]) -> Result<Option<Self::Glyph>>;
}
fn run<W: World>(cache: &mut Cache<W::Variable>, world: &mut W) -> Result<Option<W::Glyph>> {
    //7666E1/766717 set each bit before its required pointer lookup. Errors
    //describe that instruction prefix, not MSVC SEH cleanup/retry semantics.
    if cache.flags & 1 == 0 {
        cache.flags |= 1;
        cache.kingdom = Some(world.lookup_string(Slot::Kingdom)?);
    }
    if cache.flags & 2 == 0 {
        cache.flags |= 2;
        cache.observer = Some(world.lookup_string(Slot::Observer)?);
    }
    if !world.ai()? && world.online_manager()? && world.peer()? {
        if let Some(glyph) = world.online_glyph()? {
            return Ok(Some(glyph));
        }
    }
    if !matches!(world.session()?, 2 | 4 | 5) {
        return Ok(None);
    }
    //Native reads the current bound+28 first, then selects a retained pointer
    //and only then reads its current+10 string and calls4E7DC0.
    let variable = if world.kingdom()? {
        cache.kingdom
    } else {
        cache.observer
    }
    .ok_or("7666C0 reached an unbound cached String pointer")?;
    let ids = world.variable_value(variable)?;
    world.find(&ids)
}

/// One mutable process cache plus its two current source-created variables.
/// No Clone/Default/raw-value constructor is exposed. ControlledNone is an
/// explicit fresh-application scope, not evidence about another native client.
pub struct State {
    cache: Cache<Slot>,
    kingdom: Variable,
    observer: Variable,
    sources: Vec<SourceIdentity>,
    files: Vec<(String, String)>,
    stopped: Option<String>,
}
impl State {
    pub fn from_fresh_sources(
        catalog: &Catalog<'_>,
        files: &mut V14Sources,
        overrides: NativeOverrides,
    ) -> Result<Self> {
        catalog.require_source_owner()?;
        if !matches!(overrides, NativeOverrides::ControlledNone) {
            return Err(
                "player glyph current AVars require their actual profile/console override owner"
                    .into(),
            );
        }
        let paths: Vec<_> = files
            .catalog()
            .into_iter()
            .map(|e| e.logical_path)
            .collect();
        let mut kingdom: Option<Variable> = None;
        let mut observer: Option<Variable> = None;
        let mut inputs = Vec::new();
        for path in native_enumerate("AVars*.tgi", &paths) {
            let file = files.read(&path).map_err(|e| e.to_string())?;
            if aa_rules::sources::sha256(&file.bytes) != file.sha256 {
                return Err("Glyph AVar source hash differs".into());
            }
            inputs.push((file.logical_path.clone(), file.sha256.clone()));
            let doc =
                Document::parse_native(file.bytes, Limits::default()).map_err(|e| e.to_string())?;
            for item in &doc.items {
                let Item::Node(node) = item else { continue };
                if !doc
                    .header_name(&node.header)
                    .is_some_and(|v| v.eq_ignore_ascii_case("Vars"))
                {
                    continue;
                }
                for item in &node.items {
                    let Item::Property(p) = item else { continue };
                    let Some(name) = doc.property_name(p) else {
                        continue;
                    };
                    let target = if name.eq_ignore_ascii_case("KingdomGlyphInfo") {
                        &mut kingdom
                    } else if name.eq_ignore_ascii_case("ObserverGlyphInfo") {
                        &mut observer
                    } else {
                        continue;
                    };
                    if p.left.len() != 2
                        || !doc
                            .text(p.left[0].span)
                            .is_some_and(|v| v.eq_ignore_ascii_case("string"))
                    {
                        return Err(
                            "player glyph AVar assignment lacks its actual String declaration"
                                .into(),
                        );
                    }
                    let raw = doc
                        .property_value(p)
                        .ok_or("player glyph AVar assignment absent")?
                        .trim();
                    if !raw.is_ascii() || raw.contains(['"', '\\', '&', '\0']) {
                        return Err("player glyph AVar requires the proven plain ASCII string assignment scope".into());
                    }
                    let value = raw.encode_utf16().collect();
                    let provenance = Provenance {
                        path: file.logical_path.clone(),
                        sha256: file.sha256.clone(),
                        span: p.span,
                    };
                    if let Some(current) = target {
                        current.value = value;
                        current.assignments.push(provenance)
                    } else {
                        *target = Some(Variable {
                            value,
                            assignments: vec![provenance],
                        })
                    }
                }
            }
        }
        Ok(Self {
            cache: Cache {
                flags: 0,
                kingdom: None,
                observer: None,
            },
            kingdom: kingdom.ok_or("KingdomGlyphInfo actual AVar declaration absent")?,
            observer: observer.ok_or("ObserverGlyphInfo actual AVar declaration absent")?,
            sources: catalog.actors.sources.clone(),
            files: inputs,
            stopped: None,
        })
    }
    pub fn source_files(&self) -> &[(String, String)] {
        &self.files
    }
    pub fn stopped(&self) -> Option<&str> {
        self.stopped.as_deref()
    }
    pub fn glyph<P: PlayerRead + ?Sized>(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        players: &mut P,
        player: &P::Player,
    ) -> Result<Option<Glyph>> {
        catalog.require_source_owner()?;
        if self.sources != catalog.actors.sources {
            return Err("player glyph current variables belong to another source universe".into());
        }
        if let Some(error) = &self.stopped {
            return Err(format!("player glyph retained unsupported prefix: {error}"));
        }
        let Self {
            cache,
            kingdom,
            observer,
            stopped,
            ..
        } = self;
        let result = run(
            cache,
            &mut Context {
                kingdom,
                observer,
                catalog,
                bank,
                players,
                player,
            },
        );
        match result {
            Ok(Some(glyph)) if glyph.universe != catalog.actors.sources => {
                let error =
                    "online glyph belongs to a different registered source owner".to_owned();
                *stopped = Some(error.clone());
                Err(error)
            }
            Ok(glyph) => Ok(glyph),
            Err(error) => {
                *stopped = Some(error.clone());
                Err(error)
            }
        }
    }
}
struct Context<'a, 'source, P: PlayerRead + ?Sized> {
    kingdom: &'a Variable,
    observer: &'a Variable,
    catalog: &'a Catalog<'source>,
    bank: &'a Bank,
    players: &'a mut P,
    player: &'a P::Player,
}
impl<P: PlayerRead + ?Sized> World for Context<'_, '_, P> {
    type Variable = Slot;
    type Glyph = Glyph;
    fn lookup_string(&mut self, slot: Slot) -> Result<Slot> {
        Ok(slot)
    }
    fn ai(&mut self) -> Result<bool> {
        self.players.ai_flag_0c(self.player)
    }
    fn online_manager(&mut self) -> Result<bool> {
        self.players.online_manager_present()
    }
    fn peer(&mut self) -> Result<bool> {
        self.players.peer_present_04(self.player)
    }
    fn online_glyph(&mut self) -> Result<Option<Glyph>> {
        self.players
            .online_peer_glyph(self.player, self.catalog, self.bank)
    }
    fn session(&mut self) -> Result<u32> {
        self.players.session_state_e4()
    }
    fn kingdom(&mut self) -> Result<bool> {
        self.players.kingdom_present_28(self.player)
    }
    fn variable_value(&mut self, slot: Slot) -> Result<Vec<u16>> {
        Ok(match slot {
            Slot::Kingdom => &self.kingdom.value,
            Slot::Observer => &self.observer.value,
        }
        .clone())
    }
    fn find(&mut self, ids: &[u16]) -> Result<Option<Glyph>> {
        Glyph::find(self.catalog, self.bank, ids)
    }
}
