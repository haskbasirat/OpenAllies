//! Source-bound HQ constructor input; no parallel HQ runtime storage.
//! Native4684E5..46851C: find HQComponent, GetInteger(max_attachments), store+530.
use crate::catalog::Catalog;
use aa_numeric::Precision;
pub fn slots_530(catalog: &Catalog<'_>, id: &str, pc: Precision) -> Result<i32, String> {
    catalog.require_untranslated_actor_identity(id)?;
    catalog
        .actors
        .resolved_actor(id)
        .map_err(|e| e.to_string())?
        .child("HQComponent")
        .map_err(|e| e.to_string())?
        .ok_or("HQ mask lacks actual HQComponent source")?
        .field("max_attachments", pc)
        .map_err(|e| e.to_string())?
        .ok_or("HQ max_attachments lacks actual source assignment")?
        .integer()
        .map_err(|e| e.to_string())
}
