//!Native76D820 binding to sealed registration shape and the actual current
//!SVar owner. The inactive-only API remains for bounded diagnostic callers.
//!The sealed reference supplies registration/type validity, not current
//!values. The positive API reads current fields from the same published
//!World State; the inactive-only projection never certifies that owner.
use crate::{catalog::Catalog, combat_source_receipt};
use aa_orders::net::sync::Synchronizer;
use aa_rules::{resources::SourceIdentity, sources::sha256};
use serde_json::Value;
#[path="current_svar_sync.rs"]
mod current;
const REFERENCE: &[u8] = include_bytes!(
    "../../../research/development10/world-ready-7757f0/svar-registration-reference.json"
);
const REFERENCE_SHA: &str = "835544ff092751412e6df08cee9ce3a56197677e382430820bda207a505b3800";
const CAPTURE_SHA: &str = "ca5037d596281761ae3c4b479c6746ffb771b20751f3e9543364bed80202edf8";
struct Registration {
    name: String,
    kind: u32,
}
///Immutable registered-shape evidence for the captured native reference.
///No Clone/Default/Deserialize/raw constructor or current-value access exists.
pub struct Inventory {
    sources: Vec<SourceIdentity>,
    registrations: Vec<Registration>,
}
fn reference() -> Result<Vec<Registration>, String> {
    if sha256(REFERENCE) != REFERENCE_SHA {
        return Err("sealed SVar registration reference differs".into());
    }
    let value: Value = serde_json::from_slice(REFERENCE).map_err(|e| e.to_string())?;
    if value["schema"] != 1
        || value["capture_sha256"] != CAPTURE_SHA
        || value["native_exe_sha256"] != combat_source_receipt::EXE_SHA256
        || value["source_identities_sha256"] != combat_source_receipt::SOURCES_SHA256
        || value["source_control"] != 0x27f
        || value["manager"] != 0xb39aa8
        || value["count"] != 231
    {
        return Err("SVar registry reference is outside the captured source target".into());
    }
    let rows = value["entries"]
        .as_array()
        .ok_or("SVar registered rows absent")?;
    if rows.len() != 231 {
        return Err("SVar registration count differs".into());
    }
    let mut counts = [0; 6];
    let mut registrations = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        let name = row["name"].as_str().ok_or("SVar name absent")?;
        let kind = row["kind"].as_u64().ok_or("SVar type absent")? as usize;
        if row["ordinal"] != index
            || row["word20"] != index
            || !(1..=5).contains(&kind)
            || name.contains('\0')
        {
            return Err("invalid typed SVar registry shape".into());
        }
        counts[kind] += 1;
        registrations.push(Registration {
            name: name.into(),
            kind: kind as u32,
        });
    }
    if counts != [0, 21, 169, 36, 1, 4]
        || registrations
            .iter()
            .filter(|r| r.kind == 4)
            .map(|r| r.name.as_str())
            .collect::<Vec<_>>()
            != ["MapDefaultBiome"]
    {
        return Err("SVar native registered type census differs".into());
    }
    Ok(registrations)
}
fn require_inactive(sync: &Synchronizer) -> Result<(), String> {
    if sync.enabled && sync.suspend < 1 {
        return Err("76D820 enabled/unsuspended logging requires the actual complete current SVar manager; a later false thread gate is not this capability".into());
    }
    Ok(())
}
impl Inventory {
    pub(crate) fn from_captured_source(catalog: &Catalog<'_>) -> Result<Self, String> {
        catalog.require_source_owner()?;
        if combat_source_receipt::source_fingerprint(&catalog.actors.sources)
            != combat_source_receipt::SOURCES_SHA256
        {
            return Err("SVar inventory belongs to another source registration universe".into());
        }
        Ok(Self {
            sources: catalog.actors.sources.clone(),
            registrations: reference()?,
        })
    }
    pub fn registration_count(&self) -> usize {
        self.registrations.len()
    }
    ///Read the SAME current registered SVar fields, with the actual source
    ///Bank and WorldRules-publication witness. Logging gates remain lazy at
    ///each native leaf; disabled logging does not suppress current reads.
    pub fn synchronize_current(
        &self,
        catalog:&Catalog<'_>,
        bank:&crate::definitions::Bank,
        values:&crate::load_view_variables::State,
        binding:&mut crate::live_clocked::SyncBinding<'_>,
    )->Result<(),String>{
        catalog.require_source_owner()?;
        if self.sources!=catalog.actors.sources{return Err("SVar registry source owner changed".into())}
        values.validate_in_bank(catalog,bank)?;
        values.require_published()?;
        let mut count=0;
        for (index,actual)in values.registrations().enumerate(){
            let expected=self.registrations.get(index).ok_or("current SVar registration count exceeds native source")?;
            if actual.ordinal()!=index as u32||actual.name()!=expected.name||actual.kind()!=expected.kind {
                return Err(format!("current SVar registration{index} differs from sealed native source"));
            }
            count+=1;
        }
        if count!=self.registrations.len(){return Err("current SVar registration count differs from native source".into())}
        current::synchronize(values,&mut crate::launch_settings_dump::SyncSink{binding})
    }
    ///Native77DC50/77DA70/77D930/77D9D0/77DB10 return BEFORE OS/thread gates
    ///when disabled or positively suspended. No log/hash/gameplay/RNG writes
    ///are omitted on this proved typed domain. The positive branch is required.
    pub fn synchronize_inactive(
        &self,
        catalog: &Catalog<'_>,
        same_sync_a: &Synchronizer,
    ) -> Result<(), String> {
        catalog.require_source_owner()?;
        if self.sources != catalog.actors.sources {
            return Err("SVar registry source owner changed".into());
        }
        require_inactive(same_sync_a)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn captured_registered_shapes_and_inactive_native_projection() {
        assert_eq!(reference().unwrap().len(), 231);
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../research/development10/world-ready-7757f0/native-svar-inactive.json"
        ))
        .unwrap();
        assert_eq!(fixture["passed"], true);
        assert_eq!(fixture["capture_sha256"], CAPTURE_SHA);
        assert_eq!(fixture["cases"].as_array().unwrap().len(), 120);
        for row in fixture["cases"].as_array().unwrap() {
            let mut sync = Synchronizer::new(2);
            sync.enabled = row["enabled"].as_u64().unwrap() != 0;
            sync.suspend = row["suspend"].as_i64().unwrap() as i32;
            let before = serde_json::to_value(&sync).unwrap();
            require_inactive(&sync).unwrap();
            assert_eq!(serde_json::to_value(&sync).unwrap(), before);
            assert_eq!(row["unchanged"], true);
            assert_eq!(row["registrations"], 231);
            for name in [
                "executed_native_string_copy",
                "executed_native_text_wrapper",
                "executed_native_text_logger",
            ] {
                assert_eq!(row[name], true);
            }
        }
        let mut sync = Synchronizer::new(2);
        sync.enabled = true;
        assert!(require_inactive(&sync).is_err());
        sync.suspend = -1;
        assert!(require_inactive(&sync).is_err());
    }
    #[test]
    #[ignore = "requires actual AA_GAME_ROOT archive and v14 definitions"]
    fn actual_source_inventory_and_no_transplanted_source_universe() {
        use aa_rules::{
            actors::ActorCatalog,
            sources::{SourceLimits, V14Sources},
        };
        let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
        let mut files = V14Sources::open(
            root.join("Data.rwd"),
            root.join("reconstruction/native-game/runtime-expanded-v14"),
            SourceLimits {
                max_member_bytes: 64 * 1024 * 1024,
            },
        )
        .unwrap();
        let mut actors = ActorCatalog::load_v14(&mut files).unwrap();
        let inventory = Inventory::from_captured_source(&Catalog::new(&actors)).unwrap();
        inventory
            .synchronize_inactive(&Catalog::new(&actors), &Synchronizer::new(2))
            .unwrap();
        actors.sources[0].sha256 = "changed logical source".into();
        assert!(
            inventory
                .synchronize_inactive(&Catalog::new(&actors), &Synchronizer::new(2))
                .is_err()
        );
        assert!(Inventory::from_captured_source(&Catalog::new(&actors)).is_err());
    }
}
