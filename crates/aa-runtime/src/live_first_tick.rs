//! Native4F96D0 first-tick audio selection over the actual local-player owner.
//! The required747630 platform/audio callback is synchronous at its original
//! boundary. Its backend is not certified by this selection port.
use crate::{
    actor_world::{ActorStore, Source as ActorSource, SpecialSource},
    catalog::Catalog,
    definitions::Bank,
    load_tail_world::SessionRead,
    settings::{Declaration, Settings},
};
use aa_actors::ActorId;
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog, native::TypedValue, resources::Provenance, sources::V14Sources,
};
use serde::Serialize;
use std::collections::VecDeque;

#[cfg(test)]
#[path = "live_first_tick_tests.rs"]
mod tests;

type Result<T> = std::result::Result<T, String>;
trait World {
    type Player;
    type Kingdom;
    type Nation;
    type Audio;
    fn local_player(&mut self) -> Result<Option<Self::Player>>;
    /// Includes765E00's current-GWorld gate. In this live adapter the actual
    /// retained GWorld exists; a null player is consequently a native fault.
    fn player_kingdom(&mut self, player: Option<&Self::Player>) -> Result<Option<Self::Kingdom>>;
    fn nation(&mut self, kingdom: &Self::Kingdom) -> Result<Option<Self::Nation>>;
    fn nation_music(&mut self, nation: &Self::Nation) -> Result<Option<Self::Audio>>;
    fn observer_music(&mut self) -> Result<Vec<u16>>;
    fn find_audio(&mut self, id: &[u16]) -> Result<Option<Self::Audio>>;
    fn unknown_audio(&mut self, id: &[u16]) -> String;
    fn play(&mut self, audio: Self::Audio) -> Result<()>;
}
fn first_tick(w: &mut impl World) -> Result<()> {
    let player = w.local_player()?;
    let audio = if let Some(kingdom) = w.player_kingdom(player.as_ref())? {
        let Some(nation) = w.nation(&kingdom)? else {
            return Ok(());
        };
        w.nation_music(&nation)?
    } else {
        let text = w.observer_music()?;
        let id = &text[..text.iter().position(|&v| v == 0).unwrap_or(text.len())];
        if id.is_empty() {
            None
        } else {
            Some(match w.find_audio(id)? {
                Some(audio) => audio,
                None => return Err(w.unknown_audio(id)),
            })
        }
    };
    if let Some(audio) = audio {
        w.play(audio)?;
    }
    Ok(())
}

/// Raw authored clip field plus its literal comma-separated path members.
/// This records source input; it does not select/shuffle a native playlist.
#[derive(Clone, Debug, Serialize)]
pub struct ClipInput {
    pub input: TypedValue,
    pub paths: Vec<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct AudioDefinition {
    id: String,
    source: Provenance,
    sources: Vec<Provenance>,
    category: TypedValue,
    files: ClipInput,
    leadin: ClipInput,
    fallout: ClipInput,
}
impl AudioDefinition {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn source(&self) -> &Provenance {
        &self.source
    }
    pub fn sources(&self) -> &[Provenance] {
        &self.sources
    }
    pub fn category(&self) -> &TypedValue {
        &self.category
    }
    pub fn files(&self) -> &ClipInput {
        &self.files
    }
    pub fn leadin(&self) -> &ClipInput {
        &self.leadin
    }
    pub fn fallout(&self) -> &ClipInput {
        &self.fallout
    }
}
#[derive(Clone, Debug, Serialize)]
pub enum Origin {
    Nation {
        kingdom: ActorId,
        id: String,
        source: Provenance,
    },
    Observer {
        source: Provenance,
    },
}
/// The native caller ignores747630's result. These distinguish real platform
/// acknowledgement from tested audible playback; neither invents a sound ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Playback {
    Accepted,
    Declined,
}
#[derive(Clone, Debug, Serialize)]
pub enum Delivery {
    Pending,
    Finished(Playback),
    Failed(String),
}
#[derive(Clone, Debug, Serialize)]
pub struct MusicRequest {
    pub audio: AudioDefinition,
    pub origin: Origin,
    pub delivery: Delivery,
}
/// Required actual audio service. Implementations must use the exact canonical
/// IDS and report real output; no guessed nation/default playlist is requested.
pub trait AudioOutput {
    fn play_747630(&mut self, request: &MusicRequest) -> Result<Playback>;
}
pub struct UnboundAudioOutput;
impl AudioOutput for UnboundAudioOutput {
    fn play_747630(&mut self, _: &MusicRequest) -> Result<Playback> {
        Err("required native B36548 SoundManager747630 platform output is unbound".into())
    }
}

/// Application-owned request/acknowledgement storage, not a claim that these
/// are the original SoundManager's unported internal queue/voice structures.
/// A callback runs at the native call point; enqueue alone cannot finish it.
pub struct MusicRequests {
    entries: VecDeque<MusicRequest>,
}
impl MusicRequests {
    pub fn new() -> Self {
        Self {
            entries: VecDeque::new(),
        }
    }
    pub fn entries(&self) -> &VecDeque<MusicRequest> {
        &self.entries
    }
    pub fn pop_finished(&mut self) -> Option<MusicRequest> {
        self.entries
            .front()
            .is_some_and(|v| !matches!(v.delivery, Delivery::Pending))
            .then(|| self.entries.pop_front().unwrap())
    }
    fn play(
        &mut self,
        audio: AudioDefinition,
        origin: Origin,
        output: &mut (impl AudioOutput + ?Sized),
    ) -> Result<()> {
        self.entries.try_reserve(1).map_err(|e| e.to_string())?;
        self.entries.push_back(MusicRequest {
            audio,
            origin,
            delivery: Delivery::Pending,
        });
        let request = self
            .entries
            .back_mut()
            .ok_or("music request allocation vanished")?;
        match output.play_747630(request) {
            Ok(result) => {
                request.delivery = Delivery::Finished(result);
                Ok(())
            }
            Err(error) => {
                request.delivery = Delivery::Failed(error.clone());
                Err(error)
            }
        }
    }
}

fn same_row(a: &Declaration, b: &Declaration) -> bool {
    a.field_type == b.field_type && a.raw == b.raw && a.source == b.source
}
fn observer_row(settings: &Settings) -> Result<&Declaration> {
    let rows = settings
        .app_variables
        .get("observergamemusic")
        .ok_or("ObserverGameMusic has no current owner")?;
    let [row] = rows.as_slice() else {
        return Err("ObserverGameMusic override policy is not bound".into());
    };
    if !row
        .field_type
        .as_deref()
        .is_some_and(|v| v.eq_ignore_ascii_case("string"))
    {
        return Err("ObserverGameMusic is not a declared string".into());
    }
    Ok(row)
}
fn string(input: &TypedValue) -> Result<&str> {
    if let TypedValue::String { value, .. } = input {
        Ok(value)
    } else {
        Err("first-tick source field is not a string".into())
    }
}
fn clips(input: TypedValue) -> Result<ClipInput> {
    let value = string(&input)?;
    let paths = if value.is_empty() {
        vec![]
    } else {
        let mut out = vec![];
        for path in value.split(',') {
            // Current music source uses literal resource paths. Do not infer
            // formatting expressions, path fallbacks or undocumented quoting.
            if path.is_empty() || path.trim() != path || path.chars().any(char::is_control) {
                return Err(
                    "first-tick music clip-list syntax requires native parser binding".into(),
                );
            }
            out.push(path.to_owned());
        }
        out
    };
    Ok(ClipInput { input, paths })
}

/// Validated immutable source inputs for the controlled fresh Settings path.
/// No active profile is read. No arbitrary current value can impersonate the
/// private source row. Later appvar setters need their actual owner binding.
enum CatalogOwner<'a> {
    Borrowed(&'a ActorCatalog),
    Shared(std::sync::Arc<ActorCatalog>),
}
impl std::ops::Deref for CatalogOwner<'_> {
    type Target=ActorCatalog;
    fn deref(&self)->&Self::Target {
        match self {Self::Borrowed(v)=>v,Self::Shared(v)=>v}
    }
}
pub struct Source<'a> {
    catalog: CatalogOwner<'a>,
    precision: Precision,
    source_state: usize,
    observer: Declaration,
}
impl<'a> Source<'a> {
    pub fn bind(
        catalog: &Catalog<'a>,
        bank: &Bank,
        settings: &Settings,
        files: &mut V14Sources,
    ) -> Result<Self> {
        catalog.require_source_owner()?;
        let state = bank.combat_source()?;
        state.completed_means()?;
        if state.source_identities() != catalog.actors.sources.as_slice() {
            return Err("first-tick source Bank/Catalog identities differ".into());
        }
        for manager in ["Audio", "Nation"] {
            catalog
                .actors
                .native_data_registry(manager)
                .ok_or_else(|| format!("first-tick actual {manager} manager is unavailable"))?;
        }
        // Reparse actual logical files to reject forged public declaration
        // values, hashes or spans. Only this AVar is projected, not arbitrary
        // profile/console override semantics.
        let original = Settings::load_v14(files, aa_rules::tgi::Limits::default())
            .map_err(|e| e.to_string())?;
        let row = observer_row(settings)?;
        if !same_row(row, observer_row(&original)?) {
            return Err("ObserverGameMusic value/span differs from actual source".into());
        }
        Ok(Self {
            catalog: CatalogOwner::Borrowed(catalog.actors),
            precision: state.source_precision(),
            source_state: std::ptr::from_ref(state).addr(),
            observer: row.clone(),
        })
    }
    ///Retain the SAME immutable source allocation across a worker/UI handoff.
    ///A deep copy of equal source data is rejected; no Bank is rebuilt.
    pub fn into_shared(self,owner:std::sync::Arc<ActorCatalog>)
        ->std::result::Result<Source<'static>,(Self,String)> {
        if !std::ptr::eq(&*self.catalog,owner.as_ref()) {
            return Err((self,"first-tick shared source is not its actual bound allocation".into()));
        }
        Ok(Source {catalog:CatalogOwner::Shared(owner),precision:self.precision,
            source_state:self.source_state,observer:self.observer})
    }
    fn validate_bank(&self, bank: &Bank) -> Result<()> {
        let state = bank.combat_source()?;
        state.completed_means()?;
        if std::ptr::from_ref(state).addr() != self.source_state
            || state.source_identities() != self.catalog.sources.as_slice()
            || state.source_precision() != self.precision
        {
            return Err("first-tick source belongs to another moved Bank owner".into());
        }
        Ok(())
    }
    fn audio(&self, id: &str) -> Result<Option<AudioDefinition>> {
        let Some(entry) = self.catalog.native_data_find("Audio", id) else {
            return Ok(None);
        };
        let node = self
            .catalog
            .native_data_node(entry)
            .map_err(|e| e.to_string())?;
        let required = |name| {
            node.field(name, self.precision)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Audio {} has no required {name} field", entry.ids))
        };
        let ids = required("IDS")?;
        if !string(&ids)?.eq_ignore_ascii_case(&entry.ids) {
            return Err("Audio registry/typed IDS differs".into());
        }
        let category = required("category")?;
        let _ = string(&category)?;
        Ok(Some(AudioDefinition {
            id: entry.ids.clone(),
            source: entry.source.clone(),
            sources: node.sources(),
            category,
            files: clips(required("files")?)?,
            leadin: clips(required("leadin")?)?,
            fallout: clips(required("fallout")?)?,
        }))
    }
}

/// Source and output are real retained owners. Construct/use this only while
/// the same loaded GWorld is current; live_clocked supplies its actual actors.
pub struct Context<'a, 's, S: SessionRead + ?Sized, O: AudioOutput + ?Sized> {
    pub session: &'a S,
    pub source: &'a Source<'s>,
    pub bank: &'a Bank,
    pub settings: &'a Settings,
    pub requests: &'a mut MusicRequests,
    pub output: &'a mut O,
}
impl<S: SessionRead + ?Sized, O: AudioOutput + ?Sized> crate::live_clocked::FirstTick
    for Context<'_, '_, S, O>
{
    fn run(&mut self, actors: &ActorStore) -> Result<()> {
        self.source.validate_bank(self.bank)?;
        first_tick(&mut Borrowed {
            context: self,
            actors,
            origin: None,
        })
    }
}
struct Borrowed<'a, 'b, 's, S: SessionRead + ?Sized, O: AudioOutput + ?Sized> {
    context: &'a mut Context<'b, 's, S, O>,
    actors: &'a ActorStore,
    origin: Option<Origin>,
}
impl<S: SessionRead + ?Sized, O: AudioOutput + ?Sized> World for Borrowed<'_, '_, '_, S, O> {
    type Player = S::Player;
    type Kingdom = ActorId;
    type Nation = (ActorId, String);
    type Audio = AudioDefinition;
    fn local_player(&mut self) -> Result<Option<Self::Player>> {
        self.context.session.local_player()
    }
    fn player_kingdom(&mut self, player: Option<&Self::Player>) -> Result<Option<Self::Kingdom>> {
        self.context
            .session
            .player_kingdom_28(player.ok_or("native765E0C null local Player dereference")?)
    }
    fn nation(&mut self, kingdom: &Self::Kingdom) -> Result<Option<Self::Nation>> {
        let key = self
            .actors
            .key(*kingdom)
            .ok_or("first-tick local Kingdom GTID is stale")?;
        let entity = self.actors.entity(key).map_err(|e| e.to_string())?;
        let ActorSource::Special(s) = &entity.source else {
            return Err("first-tick local Kingdom has another allocation class".into());
        };
        if !matches!(s.source, SpecialSource::Kingdom(_)) {
            return Err("first-tick local owner is not a Kingdom".into());
        }
        let definition = &entity
            .runtime
            .as_ref()
            .ok_or("first-tick Kingdom runtime is missing")?
            .definition;
        let canonical = self
            .context
            .source
            .catalog
            .find(&definition.proof.id)
            .ok_or("first-tick Kingdom source is absent")?;
        let token = self
            .context
            .bank
            .token("ActorDefinition", &definition.proof.id)
            .ok_or("first-tick Kingdom source token is absent")?;
        let state = self.context.bank.combat_source()?;
        state.completed_values(token)?;
        if definition.native_class != 4
            || definition.proof.source != canonical.source
            || definition.proof.template_chain != canonical.template_chain
            || state.definition_source(token)? != &canonical.source
            || !state
                .definition_id(token)?
                .eq_ignore_ascii_case(&definition.proof.id)
            || definition.proof.category.runtime_class != 4
            || self
                .context
                .source
                .catalog
                .native_category(&definition.proof.id)
                .map_err(|e| e.to_string())?
                != definition.proof.category
        {
            return Err("first-tick Kingdom canonical definition/source differs".into());
        }
        let shell = s
            .derived
            .as_ref()
            .ok_or("first-tick Kingdom derived state is absent")?;
        let nation = match shell.references.get(&0x234) {
            Some(v) => v.as_ref(),
            None if shell.words.get(&0x234) == Some(&0) => None,
            None => return Err("first-tick actual Kingdom+234 Nation owner is unbound".into()),
        };
        Ok(nation.map(|n| (*kingdom, n.id.clone())))
    }
    fn nation_music(&mut self, nation: &Self::Nation) -> Result<Option<Self::Audio>> {
        let entry = self
            .context
            .source
            .catalog
            .native_data_find("Nation", &nation.1)
            .ok_or("first-tick Nation definition is absent")?;
        let node = self
            .context
            .source
            .catalog
            .native_data_node(entry)
            .map_err(|e| e.to_string())?;
        let input = node
            .field("game_music", self.context.source.precision)
            .map_err(|e| e.to_string())?
            .ok_or("Nation game_music has no required source field")?;
        let id = string(&input)?;
        self.origin = Some(Origin::Nation {
            kingdom: nation.0,
            id: entry.ids.clone(),
            source: entry.source.clone(),
        });
        if id.is_empty() {
            Ok(None)
        } else {
            Ok(Some(
                self.context
                    .source
                    .audio(id)?
                    .ok_or("Nation+38 Audio source lookup is unresolved")?,
            ))
        }
    }
    fn observer_music(&mut self) -> Result<Vec<u16>> {
        let row = observer_row(self.context.settings)?;
        if !same_row(row, &self.context.source.observer) {
            return Err("ObserverGameMusic current setter/override owner is unbound".into());
        }
        // The admitted current source has one quoted string or one plain IDS.
        // Bare null/empty syntax remains refused until its AVar setter is proved.
        let text = if let Some(value) = row.raw.strip_prefix('"') {
            value
                .split_once('"')
                .ok_or("unterminated ObserverGameMusic source string")?
                .0
        } else {
            if row.raw.is_empty()
                || row.raw.eq_ignore_ascii_case("null")
                || !row
                    .raw
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                return Err("ObserverGameMusic source literal syntax is unproved".into());
            }
            row.raw.as_str()
        };
        self.origin = Some(Origin::Observer {
            source: row.source.clone(),
        });
        Ok(text.encode_utf16().collect())
    }
    fn find_audio(&mut self, id: &[u16]) -> Result<Option<Self::Audio>> {
        self.context
            .source
            .audio(&String::from_utf16(id).map_err(|_| "Audio IDS has unpaired UTF16")?)
    }
    fn unknown_audio(&mut self, id: &[u16]) -> String {
        format!(
            "Unknown IDS '{}' for data manager 'Audio'",
            String::from_utf16_lossy(id)
        )
    }
    fn play(&mut self, audio: Self::Audio) -> Result<()> {
        self.context.requests.play(
            audio,
            self.origin
                .clone()
                .ok_or("first-tick audio origin was not read")?,
            self.context.output,
        )
    }
}
