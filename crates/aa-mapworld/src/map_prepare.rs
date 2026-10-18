//! Native `GWorld::ResolvePlaceholdersAndRandomThings` (`0x4FDA70`).
//!
//! This is only the ordered traversal/dispatch coordinator. The host owns the
//! live GTID table, definitions, Kingdoms, world RNG and replacement batch. All
//! replacement creation, property transfer, destruction and publication remain
//! required callbacks. Success here does not mean a map or mission is playable.
//!
//! Oracle: `research/skirmish-20261016-map-prepare/native-map-prepare.json`.
//! Original instructions are replayed with recorded host seams; each executed
//! instruction is checked byte-for-byte against the accepted v14 executable.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RandomKind {
    Object,
    Actor,
}
impl RandomKind {
    /// RandomObjectData +170 or RandomActorData +5C0, passed to `4B6FF0`.
    pub fn table_offset(self) -> u32 {
        match self {
            Self::Object => 0x170,
            Self::Actor => 0x5c0,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("map preparation host failed: {0}; earlier effects remain committed")]
    Host(String),
    #[error("native map preparation dereferences a missing owner on read {0}")]
    NullOwner(u8),
}
pub type Result<T> = std::result::Result<T, Error>;

/// Every read/call below occurs at its original point. A callback may change
/// the world: do not precompute owner answers, the live count or RNG logs.
pub trait World {
    type Object: Copy;
    type Definition: Copy;
    type Kingdom: Copy;
    type Nation: Copy;
    type Batch;

    /// Native local resolver construction (two initially eight-element-capacity
    /// vectors). Use growable storage; eight is allocation capacity, not a cap.
    /// Allocation-failure/C++ unwinding parity is outside this coordinator's proof.
    fn begin_replacements(&mut self) -> Result<Self::Batch>;
    /// Signed IDMGR +60004, reread after every processed object.
    fn live_count(&mut self) -> Result<i32>;
    /// `4EFBE0(&cursor)`, cursor initially zero; retain its native slot walk.
    fn next_object(&mut self, cursor: &mut u32) -> Result<Option<Self::Object>>;
    /// Object +4. The placeholder branch rereads this after its second owner call.
    fn definition(&mut self, object: Self::Object) -> Result<Self::Definition>;
    /// `[definition+18]+C`. This is the category, not runtime class +1C.
    fn category(&mut self, definition: Self::Definition) -> Result<u32>;
    /// Object virtual +108. Eligible placeholders call it THREE times.
    fn owner(&mut self, object: Self::Object) -> Result<Option<Self::Kingdom>>;
    fn kingdom_word_190(&mut self, kingdom: Self::Kingdom) -> Result<u32>;
    /// Kingdom +234, passed unchanged as `this` to the substitution lookup.
    fn nation(&mut self, kingdom: Self::Kingdom) -> Result<Option<Self::Nation>>;
    /// `473370(placeholder definition)` on that Nation; missing Nation behavior
    /// belongs to the real lookup, never to a guessed default nation.
    fn substitution(
        &mut self,
        nation: Option<Self::Nation>,
        definition: Self::Definition,
    ) -> Result<Option<Self::Definition>>;
    /// Replacement definition virtual +38; full EAX nonzero is true.
    fn replacement_is_actor(&mut self, definition: Self::Definition) -> Result<bool>;
    /// `404F60`, GWorld.cpp 0x138E, original placeholder ID from object +14.
    fn missing_substitution(&mut self, object: Self::Object) -> Result<()>;
    /// `77DC50` on B3655C; second argument is zero.
    fn random_scope(&mut self, kind: RandomKind) -> Result<()>;
    /// The current GWorld +D4 RNG word. Logging callbacks may change it.
    fn rng_word(&mut self) -> Result<u32>;
    /// `77DA70("(int)m_rng.GetSeed()", seed, 0)`, first B3655C then B36560.
    fn log_rng(&mut self, channel: u8, seed: u32) -> Result<()>;
    /// `4B6FF0` on the table with the actual mutable world RNG. Null results
    /// are passed to the native replacement request, not silently skipped.
    fn choose_random(
        &mut self,
        definition: Self::Definition,
        kind: RandomKind,
    ) -> Result<Option<Self::Definition>>;
    /// Object `4FD7F0` / actor `4FD8C0`; the host captures the real native
    /// replacement parameters/property transfer now, not at a later guessed time.
    fn queue_replacement(
        &mut self,
        batch: &mut Self::Batch,
        object: Self::Object,
        definition: Option<Self::Definition>,
        kind: RandomKind,
    ) -> Result<()>;
    /// `4FC4A0`: process both replacement lists in native order.
    fn process_replacements(&mut self, batch: &mut Self::Batch) -> Result<()>;
    /// Successful-path resolver destruction `4F6EB0`. On earlier errors the
    /// batch's Rust owner is dropped; native exception cleanup is not claimed.
    fn finish_replacements(&mut self, batch: Self::Batch) -> Result<()>;
}

/// `4FDA70`, normal successful-allocation path. Missing callbacks must fail.
pub fn resolve_placeholders_and_random_things(world: &mut impl World) -> Result<()> {
    let mut batch = world.begin_replacements()?;
    let mut cursor = 0;
    while world.live_count()? > 0 {
        let Some(object) = world.next_object(&mut cursor)? else {
            break;
        };
        let definition = world.definition(object)?;
        if world.category(definition)? == 0x1f {
            if world.owner(object)?.is_some() {
                let owner = world.owner(object)?.ok_or(Error::NullOwner(2))?;
                if world.kingdom_word_190(owner)? == 0 {
                    let current_definition = world.definition(object)?;
                    let owner = world.owner(object)?.ok_or(Error::NullOwner(3))?;
                    let nation = world.nation(owner)?;
                    if let Some(replacement) = world.substitution(nation, current_definition)? {
                        let kind = if world.replacement_is_actor(replacement)? {
                            RandomKind::Actor
                        } else {
                            RandomKind::Object
                        };
                        world.queue_replacement(&mut batch, object, Some(replacement), kind)?;
                    } else {
                        world.missing_substitution(object)?;
                    }
                }
            }
        } else {
            let kind = if world.category(definition)? == 0x22 {
                Some(RandomKind::Object)
            } else if world.category(definition)? == 0x23 {
                Some(RandomKind::Actor)
            } else {
                None
            };
            if let Some(kind) = kind {
                world.random_scope(kind)?;
                for channel in 0..2 {
                    let seed = world.rng_word()?;
                    world.log_rng(channel, seed)?;
                }
                let replacement = world.choose_random(definition, kind)?;
                world.queue_replacement(&mut batch, object, replacement, kind)?;
            }
        }
    }
    world.process_replacements(&mut batch)?;
    world.finish_replacements(batch)
}
