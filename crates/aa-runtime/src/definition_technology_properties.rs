//! Fresh ActorData+43C head-first TechnologyProperty references.
//!
//!46797F ->4C4CF0 prepends canonical Property references;45ECA0 exposes the
//!same list to4E6180. These are distinct from ResearchComponent+558 items.
//!Returned Bank identities authorize identity only, never property effects.
use crate::{catalog::Catalog, definition_fx::Source};
use aa_mapworld::restore::RuntimeDefinition;

/// Bind the actual immutable source definition under its completed Bank PC.
///A repeated/shared ActorData load is refused by Source::bind rather than
///being falsely reconstructed as an initially empty native linked list.
pub fn bind(catalog: &Catalog<'_>, definition: &RuntimeDefinition) -> Result<Vec<u32>, String> {
    if !matches!(definition.native_class, 3 | 4) {
        return Err("45ECA0 requires actual ActorData allocation".into());
    }
    let source = Source::bind(catalog, definition)?;
    let ids = catalog
        .actors
        .native_statistics_technology_properties(source.canonical_id(), source.source_precision())
        .map_err(|e| e.to_string())?;
    // Identity allocation does not change native traversal: the source
    //producer has already returned the actual head-first list order.
    let mut out = Vec::new();
    out.try_reserve_exact(ids.len())
        .map_err(|e| e.to_string())?;
    if ids.is_empty() {
        return Ok(out);
    }
    let mut bank = catalog
        .definition_bank
        .try_borrow_mut()
        .map_err(|_| "45ECA0 nested canonical Property Bank ownership")?;
    for id in ids {
        out.push(bank.intern("Property", &id)?);
    }
    Ok(out)
}

#[cfg(test)]
#[path = "definition_technology_properties_tests.rs"]
mod tests;
