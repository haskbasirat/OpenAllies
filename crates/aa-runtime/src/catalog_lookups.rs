//! Indexes over the ActorCatalog that one [`Catalog`](crate::catalog::Catalog)
//! borrows. They only make repeated lookups fast; every answer equals the
//! linear scan it replaces.
//!
//! Soundness: the indexes describe the Catalog's private `source_owner`
//! (`&'a ActorCatalog`, fixed at construction), which cannot change while the
//! Catalog exists; `ActorCatalog` has no interior mutability in the fields read
//! here (`sources`, `actors`). They are only lent as `&Lookups` borrowed from
//! the Catalog, so they never outlive that borrow. Each use checks the queried
//! catalog's address; any other catalog (the public `Catalog::actors` may be
//! pointed elsewhere) takes the original linear scan.
use aa_rules::{actors::ActorCatalog, resources::SourceIdentity};
use std::{
    cell::{OnceCell, RefCell},
    collections::HashMap,
    sync::Arc,
};

type Result<T> = std::result::Result<T, String>;

pub struct Lookups {
    /// Distinct for every Lookups value (never reused within the process).
    id: u64,
    /// Address of the ActorCatalog these indexes describe (never dereferenced).
    actors: usize,
    /// Lowercase source path -> first index in `ActorCatalog::sources`.
    source_positions: OnceCell<HashMap<String, usize>>,
    /// Lowercase actor IDS -> every index in `ActorCatalog::actors`, ascending.
    actor_positions: OnceCell<HashMap<String, Vec<usize>>>,
    /// Completed `fx_graph` Translate lookups (see [`TranslationKey`]).
    translations: RefCell<HashMap<TranslationKey, Result<String>>>,
    /// `combat_source_receipt::source_fingerprint` of the owner's sources.
    fingerprint: OnceCell<String>,
    /// Shared copy of the owner's `sources` (`ActorCatalog::sources_snapshot`,
    /// taken once: equal to the owner's sources for this Catalog's lifetime).
    universe: OnceCell<Arc<Vec<SourceIdentity>>>,
    /// Completed `bind_numeric` values per precision (see [`NumericMemo`]).
    numeric: RefCell<Vec<NumericMemo>>,
    /// Per field precision: every Translate root's inputs (see [`TranslateRow`]).
    translate_rows: RefCell<HashMap<u64, std::rc::Rc<Result<Vec<TranslateRow>>>>>,
    /// Per field precision: the ordered Bone roots (see [`BoneTable`]).
    bone_tables: RefCell<HashMap<u64, std::rc::Rc<Result<BoneTable>>>>,
}
/// fx_graph's Bone Find inputs at one precision: the Bone roots in their
/// stable source order with each IDS read (value or error), the first row
/// where that ordered scan stops with an error (an IDS read error or a
/// repeated IDS), and the first row of each lowercase IDS.
pub(crate) struct BoneTable {
    pub(crate) rows: Vec<BoneRow>,
    pub(crate) first_problem: Option<(usize, String)>,
    pub(crate) first_row: HashMap<String, usize>,
}
pub(crate) struct BoneRow {
    pub(crate) ids: Result<String>,
    pub(crate) sources: Vec<aa_rules::resources::Provenance>,
}
/// One Translate root as `fx_graph` reads it: its source position and span
/// start (or the error reading them gives), and its `node`/`ids_old`/
/// `ids_new` string fields (or their errors). Pure reads of the immutable
/// owner at one precision; a reader still consumes them in the original
/// order, so the same values and errors surface at the same points.
pub(crate) struct TranslateRow {
    pub(crate) source: Result<(usize, usize)>,
    pub(crate) kind: Result<String>,
    pub(crate) ids_old: Result<String>,
    pub(crate) ids_new: Result<String>,
}
/// One completed `bind_numeric`: its precision, the Settings rows it read
/// (NUMERIC_SETTINGS order) and the bound values. Replayed only while those
/// rows are unchanged; every other input belongs to the immutable owner.
struct NumericMemo {
    precision: aa_numeric::Precision,
    rows: Vec<Option<Vec<crate::settings::Declaration>>>,
    values: std::sync::Arc<std::collections::BTreeMap<String, crate::combat_value_source_context::NumericValue>>,
}
/// Everything a historical Translate lookup reads besides the immutable
/// catalog: field precision, the owner's source position and span start, the
/// manager tag and the queried IDS (exact spellings; the IDS is returned
/// unchanged when nothing matches).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct TranslationKey {
    pub(crate) precision: u64,
    pub(crate) owner: Option<(usize, usize)>,
    pub(crate) manager: String,
    pub(crate) ids: String,
}
impl Lookups {
    pub(crate) fn new(actors: &ActorCatalog) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Self {
            id: NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            actors: actors as *const ActorCatalog as usize,
            source_positions: OnceCell::new(),
            actor_positions: OnceCell::new(),
            translations: RefCell::new(HashMap::new()),
            fingerprint: OnceCell::new(),
            numeric: RefCell::new(Vec::new()),
            universe: OnceCell::new(),
            translate_rows: RefCell::new(HashMap::new()),
            bone_tables: RefCell::new(HashMap::new()),
        }
    }
    /// The Bone table for this precision, built once by `build` (None when
    /// `actors` is not the owner).
    pub(crate) fn bone_table(
        &self,
        actors: &ActorCatalog,
        precision: u64,
        build: impl FnOnce() -> Result<BoneTable>,
    ) -> Option<std::rc::Rc<Result<BoneTable>>> {
        if !self.is_for(actors) {
            return None;
        }
        if let Some(table) = self.bone_tables.borrow().get(&precision) {
            return Some(std::rc::Rc::clone(table));
        }
        let table = std::rc::Rc::new(build());
        self.bone_tables.borrow_mut().insert(precision, std::rc::Rc::clone(&table));
        Some(table)
    }
    /// The Translate rows for this precision, built once by `build` (None
    /// when `actors` is not the owner).
    pub(crate) fn translate_rows(
        &self,
        actors: &ActorCatalog,
        precision: u64,
        build: impl FnOnce() -> Result<Vec<TranslateRow>>,
    ) -> Option<std::rc::Rc<Result<Vec<TranslateRow>>>> {
        if !self.is_for(actors) {
            return None;
        }
        if let Some(rows) = self.translate_rows.borrow().get(&precision) {
            return Some(std::rc::Rc::clone(rows));
        }
        let rows = std::rc::Rc::new(build());
        self.translate_rows.borrow_mut().insert(precision, std::rc::Rc::clone(&rows));
        Some(rows)
    }
    /// The owner's shared source copy, when `actors` is the owner.
    pub(crate) fn universe(&self, actors: &ActorCatalog) -> Option<&Arc<Vec<SourceIdentity>>> {
        if !self.is_for(actors) {
            return None;
        }
        Some(self.universe.get_or_init(|| actors.sources_snapshot()))
    }
    /// `source_fingerprint(&actors.sources)` for the owner, computed once.
    pub(crate) fn source_fingerprint(&self, actors: &ActorCatalog) -> Option<&str> {
        if !self.is_for(actors) {
            return None;
        }
        Some(self.fingerprint.get_or_init(|| {
            crate::combat_source_receipt::source_fingerprint(&actors.sources)
        }))
    }
    /// A completed `bind_numeric` for this precision whose rows `unchanged`
    /// accepts, when `catalog` currently borrows the owner.
    pub(crate) fn numeric(
        &self,
        catalog: &crate::catalog::Catalog<'_>,
        precision: aa_numeric::Precision,
        unchanged: impl Fn(&[Option<Vec<crate::settings::Declaration>>]) -> bool,
    ) -> Option<std::sync::Arc<std::collections::BTreeMap<String, crate::combat_value_source_context::NumericValue>>> {
        if !self.is_for(catalog.actors) {
            return None;
        }
        self.numeric
            .borrow()
            .iter()
            .find(|memo| memo.precision == precision && unchanged(&memo.rows))
            .map(|memo| std::sync::Arc::clone(&memo.values))
    }
    pub(crate) fn store_numeric(
        &self,
        catalog: &crate::catalog::Catalog<'_>,
        precision: aa_numeric::Precision,
        rows: Vec<Option<Vec<crate::settings::Declaration>>>,
        values: std::sync::Arc<std::collections::BTreeMap<String, crate::combat_value_source_context::NumericValue>>,
    ) {
        if !self.is_for(catalog.actors) {
            return;
        }
        let mut numeric = self.numeric.borrow_mut();
        numeric.retain(|memo| memo.precision != precision);
        numeric.push(NumericMemo { precision, rows, values });
    }
    /// Identity of this value: equal ids mean the same Lookups (and so the
    /// same immutable source owner, still borrowed).
    pub(crate) fn id(&self) -> u64 {
        self.id
    }
    pub(crate) fn is_for(&self, actors: &ActorCatalog) -> bool {
        self.actors == actors as *const ActorCatalog as usize
    }
    /// `actors.sources.iter().position(|s| s.path.eq_ignore_ascii_case(path))`.
    pub(crate) fn source_position(&self, actors: &ActorCatalog, path: &str) -> Option<usize> {
        if !self.is_for(actors) {
            return actors.sources.iter().position(|s| s.path.eq_ignore_ascii_case(path));
        }
        self.source_positions
            .get_or_init(|| {
                let mut out = HashMap::new();
                for (index, source) in actors.sources.iter().enumerate() {
                    out.entry(source.path.to_ascii_lowercase()).or_insert(index);
                }
                out
            })
            .get(&path.to_ascii_lowercase())
            .copied()
    }
    /// `actors.actors.iter().position(|a| a.id.eq_ignore_ascii_case(id))`.
    pub(crate) fn actor_position(&self, actors: &ActorCatalog, id: &str) -> Option<usize> {
        if !self.is_for(actors) {
            return actors.actors.iter().position(|a| a.id.eq_ignore_ascii_case(id));
        }
        self.owner_actor_positions(actors, id).first().copied()
    }
    /// Every index `i` (ascending) with `actors.actors[i].id.eq_ignore_ascii_case(id)`.
    pub(crate) fn actor_positions(&self, actors: &ActorCatalog, id: &str) -> Vec<usize> {
        if !self.is_for(actors) {
            return actors
                .actors
                .iter()
                .enumerate()
                .filter(|(_, a)| a.id.eq_ignore_ascii_case(id))
                .map(|(index, _)| index)
                .collect();
        }
        self.owner_actor_positions(actors, id).to_vec()
    }
    fn owner_actor_positions(&self, actors: &ActorCatalog, id: &str) -> &[usize] {
        self.actor_positions
            .get_or_init(|| {
                let mut out: HashMap<String, Vec<usize>> = HashMap::new();
                for (index, actor) in actors.actors.iter().enumerate() {
                    out.entry(actor.id.to_ascii_lowercase()).or_default().push(index);
                }
                out
            })
            .get(&id.to_ascii_lowercase())
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
    /// A completed Translate lookup for this exact catalog, if any.
    pub(crate) fn translation(&self, actors: &ActorCatalog, key: &TranslationKey) -> Option<Result<String>> {
        if !self.is_for(actors) {
            return None;
        }
        self.translations.borrow().get(key).cloned()
    }
    pub(crate) fn store_translation(&self, actors: &ActorCatalog, key: TranslationKey, value: &Result<String>) {
        if self.is_for(actors) {
            self.translations.borrow_mut().insert(key, value.clone());
        }
    }
}

/// A copy of `catalog.actors.sources` (`.clone()`), shared with every other
/// copy taken from the same unchanged source list.
pub fn catalog_universe(catalog: &crate::catalog::Catalog<'_>) -> Arc<Vec<SourceIdentity>> {
    match catalog.lookups().universe(catalog.actors) {
        Some(universe) => Arc::clone(universe),
        None => Arc::new(catalog.actors.sources.clone()),
    }
}
/// `sources == catalog.actors.sources.as_slice()`. A slice that IS this
/// Catalog's shared copy is equal without comparing: that copy equals the
/// owner's sources, which cannot change while the Catalog borrows them.
pub fn same_sources(catalog: &crate::catalog::Catalog<'_>, sources: &[SourceIdentity]) -> bool {
    if let Some(universe) = catalog.lookups().universe(catalog.actors)
        && std::ptr::eq(universe.as_ptr(), sources.as_ptr())
        && universe.len() == sources.len()
    {
        return true;
    }
    sources == catalog.actors.sources.as_slice()
}
/// `a == b`; the same allocation (and length) is equal without comparing.
pub fn same_slices(a: &[SourceIdentity], b: &[SourceIdentity]) -> bool {
    (std::ptr::eq(a.as_ptr(), b.as_ptr()) && a.len() == b.len()) || a == b
}
#[cfg(test)]
#[path = "catalog_lookups_tests.rs"]
mod tests;
