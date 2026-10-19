//! Owned current-source restriction profiles loaded before505600's caller
//! proceeds to TEAMS. This is not a default ruleset or a complete World load.
use crate::catalog::Catalog;
use aa_economy::restrictions::{RestrictionProfile, RestrictionProfiles};
use aa_mapworld::{
    Chunk,
    restriction_groups::{LocalizedStorage, RestrictionGroups},
};
use aa_numeric::Precision;
use aa_rules::native::layout_data::{NativeLayoutData, PropertyRef};
use aa_rules::{
    native::{CATEGORIES, TypedValue},
    resources::Provenance,
};
use std::collections::BTreeMap;
use std::sync::Arc;

#[cfg(test)]
mod capture_tests {
    use crate as aa_runtime;
    include!("../tests/restriction_groups.rs");
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListKind {
    AllowedActors,
    AllowedProperties,
    RestrictedActors,
    RestrictedProperties,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalSource {
    pub id: String,
    pub sources: Vec<Provenance>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lookup {
    pub profile: usize,
    pub list: ListKind,
    pub authored: Vec<u16>,
    /// Native unknown/type-mismatched Find results are skips, not guessed IDs.
    pub resolved: Option<CanonicalSource>,
    pub duplicate_actor_skipped: bool,
}
#[derive(Debug)]
pub struct LoadedRestrictionGroups {
    input: RestrictionGroups,
    profiles: RestrictionProfiles,
    lookups: Vec<Lookup>,
    source_files: Vec<(String, String)>,
    source_actors: Vec<CanonicalSource>,
    property_owner: Arc<NativeLayoutData>,
    source_precision: Precision,
    /// Bookkeeping only: the catalog_lookups id whose immutable source owner
    /// last matched `source_files`/`source_actors` (both fixed at
    /// construction), so that comparison need not be repeated for it.
    source_checked: std::sync::atomic::AtomicU64,
}

fn key(units: &[u16]) -> Result<String, String> {
    // Native hash/compare stops at NUL. The existing restriction kernel's
    // eq_ignore_ascii_case is certified in the C-locale ASCII key domain.
    let units = &units[..units.iter().position(|u| *u == 0).unwrap_or(units.len())];
    if !units.iter().all(|u| *u <= 0x7f) {
        return Err("restriction lookup requires the proven ASCII C-locale key domain".into());
    }
    Ok(units.iter().map(|u| char::from(*u as u8)).collect())
}

fn hash(key: &str) -> u32 {
    key.bytes().fold(0u32, |h, c| {
        h.wrapping_mul(33)
            .wrapping_add(u32::from(c.to_ascii_uppercase()))
    })
}
///504450 /593570: rehash reverses old same-bucket chains. Keep the actual
/// final lookup precedence, including duplicate names across a rehash.
fn profile_order(names: &[String]) -> Result<Vec<usize>, String> {
    let mut buckets = vec![Vec::<usize>::new(); 13];
    for (index, name) in names.iter().enumerate() {
        if index
            >= buckets
                .len()
                .checked_mul(2)
                .ok_or("restriction bucket count overflow")?
        {
            let twice = buckets
                .len()
                .checked_mul(2)
                .ok_or("restriction bucket count overflow")?;
            let mut next = twice
                .checked_add(1)
                .ok_or("restriction bucket count overflow")?;
            if next % 3 == 0 || next % 5 == 0 {
                next = twice
                    .checked_add(3)
                    .ok_or("restriction bucket count overflow")?;
                if next % 3 == 0 || next % 5 == 0 {
                    next = twice
                        .checked_add(5)
                        .ok_or("restriction bucket count overflow")?;
                }
            }
            let mut grown = Vec::new();
            grown.try_reserve(next).map_err(|e| e.to_string())?;
            grown.resize_with(next, Vec::new);
            for chain in &buckets {
                for &old in chain {
                    let slot = hash(&names[old]) as usize % next;
                    grown[slot].insert(0, old);
                }
            }
            buckets = grown;
        }
        let slot = hash(name) as usize % buckets.len();
        buckets[slot].insert(0, index);
    }
    // RestrictionProfiles::find searches backward, whereas native maps search
    // head first. Equal ASCII keys always share their hash bucket.
    Ok(buckets
        .into_iter()
        .flat_map(|chain| chain.into_iter().rev())
        .collect())
}

impl LoadedRestrictionGroups {
    /// Original505600 fresh manager load, profile5049C0 versions2..4.
    /// Unknown typed references skip; duplicate actor entries log and skip;
    /// Property duplicates are retained; duplicate profile names preserve the
    /// native593570 rehash precedence. Allocation reentry, SEH and
    /// display-string localization side effects are outside this owned view.
    pub fn load(
        chunk: &Chunk,
        catalog: &Catalog<'_>,
        storage: LocalizedStorage,
        source_precision: Precision,
    ) -> Result<Self, String> {
        catalog.require_source_owner()?;
        if let Ok(state) = catalog
            .definition_bank
            .try_borrow()
            .map_err(|e| e.to_string())?
            .combat_source()
            && (state.source_precision() != source_precision
                || state.source_identities() != catalog.actors.sources.as_slice())
        {
            return Err(
                "restriction load source PC/identities differ from the installed Bank".into(),
            );
        }
        let input = RestrictionGroups::decode(chunk, storage).map_err(|e| e.to_string())?;
        let actors = catalog.actors;
        if actors.native_data_registry("NameList").is_none() {
            return Err("restriction groups require the native current source registry".into());
        }
        if actors
            .diagnostics
            .iter()
            .any(|d| !d.message.starts_with("native syntax notice "))
        {
            return Err(
                "restriction groups cannot accept unclassified source-load diagnostics".into(),
            );
        }
        let mut roots = Vec::new();
        for tag in aa_rules::actors::native_load::THING_TAGS {
            roots.extend(
                actors
                    .resolved_definitions(tag)
                    .map_err(|e| e.to_string())?,
            );
        }
        if roots.len() != actors.actors.len() {
            return Err(
                "restriction Thing registry differs from complete private source roots".into(),
            );
        }
        let mut canonical = BTreeMap::new();
        for node in roots {
            let sources = node.sources();
            let root = sources
                .first()
                .ok_or("restriction source root has no provenance")?;
            let mut public = actors.actors.iter().filter(|a| &a.source == root);
            let actor = public
                .next()
                .ok_or("restriction source root omitted from public registry")?;
            if public.next().is_some() || sources != actor.template_chain {
                return Err("restriction source registration/template chain mismatch".into());
            }
            // Typed string decoding (including native quote removal) performs
            // no FP arithmetic; its precision argument is not a source-PC claim.
            let Some(TypedValue::String { value: id, .. }) = node
                .field("IDS", Precision::Single24)
                .map_err(|e| e.to_string())?
            else {
                return Err("restriction source IDS must be an assigned string".into());
            };
            if id != actor.id {
                return Err("restriction source canonical IDS mismatch".into());
            }
            for source in &sources {
                if actors
                    .sources
                    .iter()
                    .filter(|s| s.path == source.path && s.sha256 == source.sha256)
                    .count()
                    != 1
                {
                    return Err("restriction source provenance omitted or duplicated".into());
                }
            }
            let Some(TypedValue::String {
                value: category, ..
            }) = node
                .field("category", Precision::Single24)
                .map_err(|e| e.to_string())?
            else {
                return Err("restriction source category must be an assigned string".into());
            };
            let kind = CATEGORIES
                .iter()
                .find(|c| c.name.eq_ignore_ascii_case(&category))
                .ok_or("restriction source category unknown")?
                .native_kind;
            canonical.insert(
                id.to_ascii_lowercase(),
                (kind, CanonicalSource { id, sources }),
            );
        }
        // Reconstruct the final current Thing Translate key set from private
        // resolved roots. This guards public actor reordering as well as
        // omissions/renames without translating a canonical identity twice.
        let mut translations = BTreeMap::new();
        for node in actors
            .resolved_definitions("Translate")
            .map_err(|e| e.to_string())?
        {
            let text = |field| -> Result<String, String> {
                match node
                    .field(field, Precision::Single24)
                    .map_err(|e| e.to_string())?
                {
                    Some(TypedValue::String { value, .. }) => Ok(value),
                    _ => Err(format!(
                        "restriction Translate requires assigned string {field}"
                    )),
                }
            };
            let manager = text("node")?;
            if aa_rules::actors::native_load::THING_TAGS
                .iter()
                .any(|tag| tag.eq_ignore_ascii_case(&manager))
            {
                let old = text("ids_old")?.to_ascii_lowercase();
                let new = text("ids_new")?.to_ascii_lowercase();
                if translations.insert(old, new).is_some() {
                    return Err("restriction source has duplicate Thing Translate key".into());
                }
            }
        }
        // Borrow the same cached Property arena as every restored actor. Its
        // Find applies current Translate once; use the canonical returned IDS
        // directly, without sending it through Translate a second time.
        let properties = catalog.layout_data(source_precision)?;
        let mut records = Vec::new();
        let mut lookups = Vec::new();
        for (ordinal, record) in input.profiles().iter().enumerate() {
            let name = key(&record.name)?;
            let mut profile = RestrictionProfile::default();
            let mut allowed_actors = Vec::<String>::new();
            let mut restricted_actors = Vec::<String>::new();
            for (list, values) in [(ListKind::AllowedActors, &record.allowed_actors),
                (ListKind::AllowedProperties, &record.allowed_properties),
                (ListKind::RestrictedActors, record.restricted_actors.as_ref().ok_or(
                    "restriction profile version0/1 needs the completed native category/property complement enumeration")?),
                (ListKind::RestrictedProperties, record.restricted_properties.as_ref().ok_or(
                    "restriction profile version0/1 needs the completed native category/property complement enumeration")?)] {
                for authored in values {
                    let ids = key(authored)?;
                    let actor_list = matches!(list, ListKind::AllowedActors | ListKind::RestrictedActors);
                    let resolved = if actor_list {
                        let k = ids.to_ascii_lowercase();
                        let expected = if ids.is_empty() { None } else {
                            canonical.get(translations.get(&k).unwrap_or(&k))
                        };
                        let found = if ids.is_empty() { None } else { actors.find(&ids) };
                        if found.map(|a| (&a.id, &a.template_chain)) != expected.map(|(_, s)| (&s.id, &s.sources)) {
                            return Err("restriction current Thing Find transport differs from private canonical/Translate owner".into());
                        }
                        expected.filter(|(kind, _)| matches!(kind, 2 | 19)).map(|(_, s)| s.clone())
                    } else {
                        properties.find_property(&ids, None).map(|reference| {
                            let PropertyRef::Registered(index) = reference else {
                                return Err("restriction Property Find returned an embedded identity".into());
                            };
                            let entry = properties.properties.get(index as usize)
                                .ok_or("restriction Property reference outside shared source owner")?;
                            Ok::<_, String>(CanonicalSource { id: entry.ids.clone(), sources: entry.sources.clone() })
                        }).transpose()?
                    };
                    let mut duplicate_actor_skipped = false;
                    if let Some(source) = &resolved {
                        if !source.id.is_ascii() || source.id.contains('\0') {
                            return Err("restriction canonical identity is outside the proven ASCII key domain".into());
                        }
                        let actor_seen = if list == ListKind::AllowedActors { &mut allowed_actors } else { &mut restricted_actors };
                        duplicate_actor_skipped = actor_list && actor_seen.iter().any(|id| id.eq_ignore_ascii_case(&source.id));
                        if !duplicate_actor_skipped {
                            if actor_list { actor_seen.push(source.id.clone()); }
                            match list {
                                ListKind::RestrictedActors => profile.actors.push((source.id.clone(), true)),
                                ListKind::RestrictedProperties => profile.technologies.push((source.id.clone(), true)),
                                _ => {}
                            }
                        }
                    }
                    lookups.push(Lookup { profile: ordinal, list, authored: authored.clone(), resolved, duplicate_actor_skipped });
                }
            }
            records.push(Some((name, profile)));
        }
        let names: Vec<_> = records
            .iter()
            .map(|p| p.as_ref().unwrap().0.clone())
            .collect();
        let mut profiles = RestrictionProfiles::default();
        for index in profile_order(&names)? {
            profiles.profiles.push(records[index].take().unwrap());
        }
        let source_files = actors
            .sources
            .iter()
            .map(|s| (s.path.clone(), s.sha256.clone()))
            .collect();
        let source_actors = actors
            .actors
            .iter()
            .map(|a| CanonicalSource {
                id: a.id.clone(),
                sources: a.template_chain.clone(),
            })
            .collect();
        Ok(Self {
            input,
            profiles,
            lookups,
            source_files,
            source_actors,
            property_owner: properties,
            source_precision,
            source_checked: std::sync::atomic::AtomicU64::new(0),
        })
    }
    pub fn profiles(&self) -> &RestrictionProfiles {
        &self.profiles
    }
    pub fn input(&self) -> &RestrictionGroups {
        &self.input
    }
    pub fn lookups(&self) -> &[Lookup] {
        &self.lookups
    }
    pub fn source_precision(&self) -> Precision {
        self.source_precision
    }
    /// The stored keys/identities may only be used with this source universe.
    pub fn require_catalog(&self, catalog: &Catalog<'_>) -> Result<(), String> {
        catalog.require_source_owner()?;
        if let Ok(state) = catalog
            .definition_bank
            .try_borrow()
            .map_err(|e| e.to_string())?
            .combat_source()
            && (state.source_precision() != self.source_precision
                || !crate::catalog_lookups::same_sources(catalog, state.source_identities()))
        {
            return Err(
                "restriction groups source PC/identities differ from the installed Bank".into(),
            );
        }
        let properties = catalog.layout_data(self.source_precision)?;
        if !Arc::ptr_eq(&properties, &self.property_owner) {
            return Err("restriction groups use another Property source arena".into());
        }
        //The rest compares only the immutable source owner with fields fixed at
        //construction: once it matched for this owner, it matches again.
        let lookups = catalog.lookups();
        let owner = lookups.is_for(catalog.actors);
        if owner
            && self.source_checked.load(std::sync::atomic::Ordering::Relaxed) == lookups.id()
        {
            return Ok(());
        }
        let sources = &catalog.actors.sources;
        if sources.len() != self.source_files.len()
            || sources
                .iter()
                .zip(&self.source_files)
                .any(|(s, (path, sha))| s.path != *path || s.sha256 != *sha)
        {
            return Err("restriction groups source universe changed".into());
        }
        if catalog.actors.actors.len() != self.source_actors.len()
            || catalog
                .actors
                .actors
                .iter()
                .zip(&self.source_actors)
                .any(|(a, s)| {
                    a.id != s.id
                        || a.template_chain != s.sources
                        || s.sources.first() != Some(&a.source)
                })
        {
            return Err("restriction groups canonical source registration changed".into());
        }
        if owner {
            self.source_checked
                .store(lookups.id(), std::sync::atomic::Ordering::Relaxed);
        }
        Ok(())
    }
}
