//! Memoized successful results of per-definition [`ActorCatalog`] queries.
//!
//! Every query here first selects the registered actor entry and checks its
//! `category` resolution exactly as before (those steps read public fields and
//! are never cached). What remains depends only on the selected entry's
//! `source` (which picks the resolver root) and on the private, immutable
//! resolver, so a successful result is a pure function of (root index, query
//! inputs). Only `Ok` values are stored: an error is always recomputed, so its
//! text (which may quote caller spelling) is produced exactly as before.
//!
//! The memo belongs to one catalog instance; a clone starts empty.
use crate::{
    native::{CategoryDescriptor, TypedValue},
    objects::ObjectDefinition,
    resources::Provenance,
};
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

#[derive(Debug, Default)]
pub(crate) struct QueryMemo {
    /// Exact root `source` -> every resolver root index with that source,
    /// ascending (the order of a linear scan over `Resolver::roots`).
    pub(crate) roots_by_source: OnceLock<HashMap<Provenance, Vec<usize>>>,
    /// (root, precision bits) -> field path -> `native_field` value.
    pub(crate) fields: Mutex<HashMap<(usize, u64), HashMap<String, Option<TypedValue>>>>,
    /// (root, encoded child path) -> `native_node_sources` value.
    pub(crate) node_sources: Mutex<HashMap<(usize, String), Vec<Provenance>>>,
    /// root -> `native_category` value.
    pub(crate) categories: Mutex<HashMap<usize, CategoryDescriptor>>,
    /// (root, precision bits) -> `native_final_flags` value.
    pub(crate) final_flags: Mutex<HashMap<(usize, u64), u32>>,
    /// (root, precision bits) -> `native_object_definition` value. Its `id`
    /// field is the caller's spelling and is replaced on every hit.
    pub(crate) object_definitions: Mutex<HashMap<(usize, u64), ObjectDefinition>>,
    /// (root, is_actor, precision bits) -> `native_base_flags` value.
    pub(crate) base_flags: Mutex<HashMap<(usize, bool, u64), (u32, Vec<TypedValue>)>>,
    /// root -> resolver paths of `resolved_actor`'s layers.
    pub(crate) actor_layers: Mutex<HashMap<usize, Vec<Vec<usize>>>>,
    /// Last shared copy of `ActorCatalog::sources` (see `sources_snapshot`).
    pub(crate) sources_snapshot: Mutex<Option<std::sync::Arc<Vec<crate::resources::SourceIdentity>>>>,
}
impl Clone for QueryMemo {
    fn clone(&self) -> Self {
        Self::default()
    }
}
/// The memo map, or None when it is unavailable (poisoned, or bypassed by a
/// unit test to compute the uncached result): the caller then computes.
pub(crate) fn lock<T>(memo: &Mutex<T>) -> Option<std::sync::MutexGuard<'_, T>> {
    #[cfg(test)]
    if BYPASS.with(std::cell::Cell::get) {
        return None;
    }
    memo.lock().ok()
}
#[cfg(test)]
thread_local! {
    /// Unit tests set this to obtain uncached results for comparison.
    pub(crate) static BYPASS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
/// Injective encoding of a child path (length-prefixed segments).
pub(crate) fn path_key(path: &[&str]) -> String {
    let mut out = String::new();
    for segment in path {
        out.push_str(&segment.len().to_string());
        out.push(':');
        out.push_str(segment);
    }
    out
}
