//!4FBB7D..4FBBAD argument capture for the replacement success diagnostic.
//!
//!The retained NativeMessage vector is the modern diagnostic transport also
//!used by restore/configuration/Organization code. Its formatting/emission
//!policy is not the original B38B70 threshold or native virtual sink ABI.
use crate::map_replacement_queue::DefinitionRef;
use aa_mapworld::restore::NativeMessage;

pub(super) fn current_ids(
    bank: &crate::definitions::Bank,
    definition: &DefinitionRef,
) -> Result<String, String> {
    let entry = bank.require_usable(definition.token)?;
    if entry.key.domain != "actordefinition"
        || !entry
            .key
            .name
            .eq_ignore_ascii_case(&definition.canonical_ids)
    {
        return Err("4FBB83 current replacement source/Bank identity differs".into());
    }
    //The Bank key is case folded for identity. The queued reference retains
    //the actual source Data+8 spelling copied by the replacement producer.
    Ok(definition.canonical_ids.clone())
}

struct Arguments {
    ids: String,
    x_double: u64,
    y_double: u64,
}
fn arguments(
    definition: Option<&DefinitionRef>,
    position: &[u32; 3],
    read_ids: impl FnOnce(&DefinitionRef) -> Result<String, String>,
) -> Result<Arguments, String> {
    //Native reads Y, then the current Params definition's IDS pointer, then
    //stores Y as double, reads X and stores X as double. These are current
    //post-Create Params, never the old placeholder or entry-time pose.
    let y = f32::from_bits(position[1]);
    let definition = definition.ok_or("4FBB83 current replacement Data pointer is null")?;
    let ids = read_ids(definition)?;
    let y = f64::from(y);
    let x = f64::from(f32::from_bits(position[0]));
    Ok(Arguments {
        ids,
        x_double: x.to_bits(),
        y_double: y.to_bits(),
    })
}
pub(super) fn message(
    definition: Option<&DefinitionRef>,
    position: &[u32; 3],
    read_ids: impl FnOnce(&DefinitionRef) -> Result<String, String>,
) -> Result<NativeMessage, String> {
    let Arguments {
        ids,
        x_double,
        y_double,
    } = arguments(definition, position, read_ids)?;
    let x = f64::from_bits(x_double);
    let y = f64::from_bits(y_double);
    Ok(NativeMessage {
        //Post-load messages use the existing non-record grouping convention.
        //This is diagnostic metadata, not a claimed DYNAMIC_THINGS ordinal.
        source_ordinal: 0,
        function: 0x404f90,
        line: 0x135e,
        //Rust's current logger formatting is intentional transport policy.
        //No native CRT printf parity is claimed for this displayed text.
        text: format!("created replacement {ids} at {x:.6},{y:.6}"),
    })
}

#[cfg(test)]
#[path = "replacement_created_diagnostic_tests.rs"]
mod tests;
