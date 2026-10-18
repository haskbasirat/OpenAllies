//!Selected-map slice of the same application's generic resource namespace.
//!5AD430 cached lookup increments actual+14 before type/open checks.59F570
//!then opens through the retained depot and closes/destroys that probe stream.
//!The V14 archive backend replaces OS packed-file leaves; unknown resources,
//!class collisions and unported general resource workers are not success.
use crate::{catalog::Catalog, map_resource_streams::Owner, resource_creator::SourceResource};
use aa_rules::sources::{Origin, V14Sources};
use std::{collections::BTreeMap, sync::Arc};
type Result<T> = std::result::Result<T, String>;

///859E50 replaces backslash by slash;405030/5AC160 use C-string prefix and
///ASCII case-insensitive identity. Non-ASCII case/locale lookup is unbound.
pub(crate) fn normalized_key(ids: &[u16]) -> Result<String> {
    let end = ids.iter().position(|&u| u == 0).unwrap_or(ids.len());
    if ids[..end].iter().any(|&u| u > 127) {
        return Err("resource name outside proved ASCII native lookup domain".into());
    }
    let text = String::from_utf16(&ids[..end]).map_err(|e| e.to_string())?;
    Ok(text.replace('\\', "/").to_ascii_lowercase())
}
enum Metadata {
    Pending,
    Loaded {
        resource: Arc<SourceResource>,
        streams: Owner,
        origin: Origin,
        file_sha: String,
    },
    Failed(String),
}
struct Entry {
    ids: Vec<u16>,
    backing_path: String,
    references_14: u32,
    metadata: Metadata,
}
///No Clone/Default/raw insert. Application owns one and checks generic map/
///film IDS collisions before delegation; these are not type-separated native
///hashmaps. MapSource strong archive ownership survives every opened view.
pub struct State {
    entries: BTreeMap<String, Entry>,
    source_fingerprint: Option<String>,
}
impl State {
    ///5B0479..5B04EA constructs both actual generic name hashes empty. Their
    ///initial bucket counts are implementation details, not resource limits.
    pub(crate) fn constructed() -> Self {
        Self {
            entries: BTreeMap::new(),
            source_fingerprint: None,
        }
    }
    pub(crate) fn contains_key(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }
    pub(crate) fn keys(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }
    fn validate(&mut self, catalog: &Catalog<'_>) -> Result<()> {
        catalog.require_source_owner()?;
        let actual = crate::combat_source_receipt::source_fingerprint(&catalog.actors.sources);
        if let Some(expected) = &self.source_fingerprint {
            if expected != &actual {
                return Err(
                    "map resource depot belongs to another definition source target".into(),
                );
            }
        } else {
            self.source_fingerprint = Some(actual)
        }
        Ok(())
    }
    ///Concrete selected-file preload: resolve the actual archive/loose overlay,
    ///construct the MapResource base(+14=0), materialize its Header/Creator via
    ///the existing verified loaders, then retain the same source allocation.
    ///This is not a general ResourceManager worker/init capability.
    pub(crate) fn select_map(
        &mut self,
        catalog: &Catalog<'_>,
        files: &mut V14Sources,
        ids: &[u16],
    ) -> Result<Arc<SourceResource>> {
        self.select_map_internal(catalog, files, ids, None)
    }
    ///Production source allocation also executes5A27A0->5A23F0->5A14C0.
    ///The same application's B3D8CC owner is incremented before any depot
    ///read/header error. The diagnostic value-only wrapper above omits that
    ///constructor receipt and must not certify a fresh native application.
    pub(crate) fn select_map_with_process(
        &mut self,catalog:&Catalog<'_>,files:&mut V14Sources,ids:&[u16],
        process:&mut crate::film_process_metadata::State,
    )->Result<Arc<SourceResource>> {
        self.select_map_internal(catalog,files,ids,Some(process))
    }
    fn select_map_internal(
        &mut self,catalog:&Catalog<'_>,files:&mut V14Sources,ids:&[u16],
        process:Option<&mut crate::film_process_metadata::State>,
    )->Result<Arc<SourceResource>> {
        self.validate(catalog)?;
        let key = normalized_key(ids)?;
        if self.entries.contains_key(&key) {
            return self.find_scenario(catalog, files, ids);
        }
        let candidates = files
            .catalog()
            .into_iter()
            .filter(|row| {
                normalized_key(&row.logical_path.encode_utf16().collect::<Vec<_>>()).as_ref()
                    == Ok(&key)
            })
            .collect::<Vec<_>>();
        let [entry] = candidates.as_slice() else {
            return Err("selected resource absent or ambiguous in actual accepted depot".into());
        };
        if !entry.logical_path.to_ascii_lowercase().ends_with(".rmp") {
            return Err("selected resource has an unbound non-MapResource class".into());
        }
        let canonical = entry.logical_path.clone();
        //Resource constructor receives the normalized query spelling, not a
        //catalog display-name substitution. Depot spelling stays separate.
        let resource_ids = ids
            .iter()
            .copied()
            .take_while(|&u| u != 0)
            .map(|u| {
                if u == u16::from(b'\\') {
                    u16::from(b'/')
                } else {
                    u
                }
            })
            .collect::<Vec<_>>();
        self.entries.insert(
            key.clone(),
            Entry {
                ids: resource_ids.clone(),
                backing_path: canonical.clone(),
                references_14: 0,
                metadata: Metadata::Pending,
            },
        );
        if let Some(process)=process {process.header_reset_5a14c0();}
        let loaded = (|| {
            let input = files.read(&canonical).map_err(|e| e.to_string())?;
            let origin = input.origin;
            let file_sha = input.sha256;
            let map =
                aa_mapworld::MapArchive::from_bytes(input.bytes).map_err(|e| e.to_string())?;
            let streams = Owner::open(map);
            let resource = SourceResource::load(catalog, &streams, &resource_ids)?;
            Ok::<_, String>((resource, streams, origin, file_sha))
        })();
        let entry = self
            .entries
            .get_mut(&key)
            .expect("owned reserved resource remains present");
        match loaded {
            Ok((resource, streams, origin, file_sha)) => {
                entry.metadata = Metadata::Loaded {
                    resource: Arc::clone(&resource),
                    streams,
                    origin,
                    file_sha,
                };
                Ok(resource)
            }
            Err(error) => {
                entry.metadata = Metadata::Failed(error.clone());
                Err(error)
            }
        }
    }
    ///59F570 cached selected-map path. Missing general resource creation is
    ///explicit, rather than looking in another type's map or returning null.
    pub(crate) fn find_scenario(
        &mut self,
        catalog: &Catalog<'_>,
        files: &mut V14Sources,
        ids: &[u16],
    ) -> Result<Arc<SourceResource>> {
        self.validate(catalog)?;
        let key = normalized_key(ids)?;
        let entry = self.entries.get_mut(&key).ok_or(
            "59F570 requires actual selected resource preload; general fallback5AEA90 is unbound",
        )?;
        entry.references_14 = entry.references_14.wrapping_add(1);
        let Metadata::Loaded {
            resource,
            streams,
            origin,
            file_sha,
        } = &entry.metadata
        else {
            return Err(match &entry.metadata {
                Metadata::Failed(e) => format!("current MapResource metadata remained failed: {e}"),
                _ => "current MapResource metadata has not completed".into(),
            });
        };
        let bank = catalog
            .definition_bank
            .try_borrow()
            .map_err(|_| "map resource Bank already borrowed")?;
        resource.parsed_creator().validate(catalog, &bank)?;
        //5AA340 selects the same file's actual depot. Source.read owns the
        //real original RWD/accepted loose overlay read; it never manufactures
        //an open callback. The probe cursor is consumed by its close/drop.
        let probe = files.read(&entry.backing_path).map_err(|e| e.to_string())?;
        if &probe.origin != origin
            || &probe.sha256 != file_sha
            || probe.sha256 != streams.archive()?.source().sha256
        {
            return Err("59F570 current backing depot bytes differ from retained resource".into());
        }
        let stream = std::io::Cursor::new(probe.bytes);
        drop(stream); //native close(+28), then destroy(+10), one owned probe
        Ok(Arc::clone(resource))
    }
    ///Cached59F570 does not allocate or reset another Header. The process
    ///owner is lent to maintain the same API as the allocating selected path;
    ///no increment occurs on either a cached hit or this bounded missing error.
    pub(crate) fn find_scenario_with_process(
        &mut self,catalog:&Catalog<'_>,files:&mut V14Sources,ids:&[u16],
        _process:&mut crate::film_process_metadata::State,
    )->Result<Arc<SourceResource>> {self.find_scenario(catalog,files,ids)}
    pub(crate) fn open_map_streams(&self, source: &Arc<SourceResource>) -> Result<Owner> {
        let key = normalized_key(source.ids())?;
        let entry = self
            .entries
            .get(&key)
            .ok_or("map source is not owned by this application depot")?;
        let Metadata::Loaded {
            resource, streams, ..
        } = &entry.metadata
        else {
            return Err("owned map metadata is incomplete".into());
        };
        if !Arc::ptr_eq(resource, source) {
            return Err("equal map bytes/IDS are not the same source allocation".into());
        }
        streams.reopen_shared()
    }
    pub(crate) fn references(&self, ids: &[u16]) -> Result<u32> {
        self.entries
            .get(&normalized_key(ids)?)
            .map(|e| e.references_14)
            .ok_or("resource is not in current map cache".into())
    }
    ///A different typed Find still hits the same generic resource first.
    ///5AD430 increments+14 BEFORE59F450/5A26E0 class-chain rejection.
    pub(crate) fn reject_wrong_type_lookup(&mut self, ids: &[u16]) -> Result<()> {
        let entry = self
            .entries
            .get_mut(&normalized_key(ids)?)
            .ok_or("wrong-class lookup has no actual cached resource")?;
        entry.references_14 = entry.references_14.wrapping_add(1);
        Err("resource exists but has MapResource class, not the requested native class".into())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_resource_constructor_and_cached_lookup_replay() {
        let doc: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261017-resource-map-cache/native-cache.json"
        ))
        .unwrap();
        assert_eq!(doc["passed"], true);
        for row in doc["constructors"].as_array().unwrap() {
            assert!(State::constructed().entries.is_empty());
            assert_eq!(row["refcount"], 0);
        }
        for row in doc["cached_lookups"].as_array().unwrap() {
            let key = row["key"]
                .as_str()
                .unwrap()
                .encode_utf16()
                .collect::<Vec<_>>();
            assert_eq!(normalized_key(&key).unwrap(), "maps/actual.rmp");
            assert_eq!(
                (row["initial"].as_u64().unwrap() as u32).wrapping_add(1),
                row["count"].as_u64().unwrap() as u32
            );
            assert_eq!(
                row["protocol"],
                serde_json::json!(["open_as_file", "close", "destroy"])
            );
        }
        for row in doc["wrong_class"].as_array().unwrap() {
            let mut owner = State::constructed();
            let ids = "Maps/Actual.RMP".encode_utf16().collect::<Vec<_>>();
            owner.entries.insert(
                normalized_key(&ids).unwrap(),
                Entry {
                    ids: ids.clone(),
                    backing_path: "Maps/Actual.RMP".into(),
                    references_14: 0,
                    metadata: Metadata::Pending,
                },
            );
            assert!(owner.reject_wrong_type_lookup(&ids).is_err());
            assert_eq!(
                owner.references(&ids).unwrap(),
                row["count_after"].as_u64().unwrap() as u32
            );
        }
    }
}
