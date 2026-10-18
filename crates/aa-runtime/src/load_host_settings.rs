//! Registered SVar read by fresh SetHost757185, before map-rule application.
//!5069C8..5069FE registers slot8/type2/flags0/default16 through401710/4B3470.
//! A missing authored assignment retains that genuine constructor value only
//! after checking the private actual source graph and current projection.
use crate::{
    catalog::Catalog, combat_source_receipt as receipt, combat_value_source_context as context,
};

pub fn admin_ticks_per_second(catalog: &Catalog<'_>) -> Result<u32, String> {
    catalog.require_source_owner()?;
    let bank = catalog.definition_bank.borrow();
    let source = bank.combat_source()?;
    source.completed_means()?;
    if receipt::source_fingerprint(&catalog.actors.sources)
        != receipt::source_fingerprint(source.source_identities())
    {
        return Err("local Host source Bank and Catalog identities differ".into());
    }
    // Recheck source FP/numeric context rather than accepting an arbitrary
    // explicit-PC diagnostic Bank or another public Settings projection.
    context::bind_numeric(catalog, source.context())?;
    let name = "AdminTicksPerSecond";
    let registry = catalog
        .actors
        .native_data_registry("WorldRule")
        .ok_or("local Host WorldRule registry is absent")?;
    if registry
        .entries
        .iter()
        .any(|entry| entry.ids.eq_ignore_ascii_case(name))
    {
        return Err(
            "local Host AdminTicksPerSecond WorldRule default application is unbound".into(),
        );
    }
    let settings = catalog
        .settings
        .as_ref()
        .ok_or("local Host Settings owner is absent")?;
    let pc = source.source_precision();
    let nodes = catalog
        .actors
        .resolved_definitions("SVars")
        .map_err(|e| e.to_string())?;
    let mut actual = Vec::new();
    for node in &nodes {
        if let Some(value) = node.field(name, pc).map_err(|e| e.to_string())? {
            actual.push(value);
        }
    }
    if actual.is_empty() {
        if settings.declarations.contains_key("admintickspersecond") {
            return Err("local Host AdminTicksPerSecond Settings value/span has no actual registered source assignment".into());
        }
        return Ok(constructor_default());
    }
    let (bits, provenance) = settings.unique_fixed(name, pc).map_err(|e| e.to_string())?;
    if !catalog.actors.sources.iter().any(|file| {
        file.path.eq_ignore_ascii_case(&provenance.path) && file.sha256 == provenance.sha256
    }) {
        return Err("local Host AdminTicksPerSecond Settings source identity differs".into());
    }
    let mut matching = 0usize;
    for field in actual {
        if let aa_rules::native::TypedValue::Scalar(value) = field
            && value.source == *provenance
            && value.field_type == aa_rules::native::ScalarType::Fixed
            && value.bits == bits
        {
            matching += 1;
        }
    }
    if matching != 1 {
        return Err("local Host AdminTicksPerSecond Settings value/span differs from actual registered source".into());
    }
    Ok(bits)
}

fn constructor_default() -> u32 {
    0x41800000
}

#[cfg(test)]
mod tests {
    #[test]
    fn actual_native_registration_retains_its_constructor_through_no_source_and_restricted_profile()
    {
        let proof: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261016-local-bootstrap/host/native-admin-default.json"
        ))
        .unwrap();
        assert_eq!(proof["passed"], true);
        assert_eq!(proof["cases"].as_array().unwrap().len(), 6);
        for row in proof["cases"].as_array().unwrap() {
            assert_eq!(row["name"], "AdminTicksPerSecond");
            assert_eq!(row["initial"]["bits"], super::constructor_default());
            assert_eq!(row["initial"]["kind"], 2);
            assert_eq!(row["initial"]["flags"], 0);
            assert_eq!(row["initial"]["index"], 8);
            assert_eq!(row["after_empty_source"], row["initial"]);
            assert_eq!(row["after_restricted_profile"], row["initial"]);
            assert_eq!(row["same_identity"], true);
        }
    }
}
