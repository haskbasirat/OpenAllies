//! Definition fields read by FRESH creation (`4F9C10` and the class
//! constructors / InitCreate / InitExisting below it), at their native
//! ObjectData / ActorData offsets.
//!
//! Object and DynamicDoodad definitions (`aa-mapworld` fresh_creation,
//! request C5 of research/phase8-conversion), all stored by ObjectData
//! LoadData `49DC40` (oracle research/phase8-actor-creation/
//! native-object-definition.json):
//! * +24 `Size.height` (read by `4EBE50` as the factor of object +38 =
//!   scale x +24: the scaled height, not a radius);
//! * +28 / +2C `Size.scale_min` / `scale_max`;
//! * +30 `description` (stored text);
//! * +34 `needs_space_for_settlement_zone` (the byte returned by definition
//!   virtual +24 `462E50` for ObjectData and ActorData);
//! * +38 shape A (`blocking_shape`), +50 shape B (`soft_shape`, keeping the
//!   blocking kind when empty), +68 target shape (soft when empty), +80
//!   explicit target byte: [`crate::objects::shape`] words;
//! * DynamicDoodad components +174 / +178: their +10 is `update_time`
//!   (loaders 4C8040 / 4C7CC0, research/phase7-definitions/
//!   native-doodads.json), the delay factor of `5904A0` / `58F960`.
//!
//! ActorData fields of actor creation (aa_actors::actor_creation::
//! ActorDefinition): +174 component mask, +1AC `name_list_ids` (a NameList
//! data-manager entry: its `Name` children's `name` texts), +1B0
//! `numbered`, +25C `TrackInfo` (track_type DISCRETE 0 / CONTINUOUS 1 as
//! registered by `4BDFF0`, `fade_sec` at +20), +268 `trailer_actor`, +270
//! `TrailerHitch.node_offset`, +2F0 `OrganizationComponent.layout_IDS`,
//! +4E0 `required_object_ids` (deferred reference A03480 with +10 = 1),
//! +5B4 `CityComponent.zone_radius`. ActorData LoadData `464BD0` stores
//! (464D13..464D84, 465DA4..465F74, 465F7D..4660B2, 467BA9..467C0E,
//! 4684BA).
use crate::{
    actors::ActorCatalog,
    native::{Error, Result, TypedValue},
};
use aa_numeric::Precision;
use serde::Serialize;

/// The object-definition fields of fresh creation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ObjectCreationInputs {
    pub id: String,
    /// `[+18]+1C`.
    pub runtime_class: u32,
    /// +24 Size.height.
    pub height_bits: u32,
    /// +28 / +2C.
    pub scale_min_bits: u32,
    pub scale_max_bits: u32,
    /// +34 needs_space_for_settlement_zone.
    pub settlement_space: bool,
    /// +38 / +50 / +68 shape words (kind, radius, min x, min y, max x,
    /// max y); +80.
    pub shape_a: [u32; 6],
    pub shape_b: [u32; 6],
    pub shape_target: [u32; 6],
    pub explicit_target: bool,
    /// DynamicDoodad +174 / +178 component +10 (`update_time`), class 9.
    pub doodad_update_time_bits: [Option<u32>; 2],
}

/// A TrackInfo entry (0x24 bytes): +0 track_type, +20 fade_sec.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct TrackInfoInputs {
    pub kind: u32,
    pub fade_sec_bits: u32,
}

/// A loaded NameList data-manager entry (+18 localized names, +1C count).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NameListInputs {
    pub ids: String,
    /// The 5B5620 localized `name` texts of the `Name` children, in order.
    pub names: Vec<String>,
}

/// The ActorData fields of actor creation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ActorCreationInputs {
    pub object: ObjectCreationInputs,
    /// +1C (`native_final_flags`).
    pub flags: u32,
    /// +108 / aspect weights / +128 (`native_creation_definition`).
    pub variations_enabled: bool,
    pub aspect_weights_bits: Vec<u32>,
    pub total_weight_bits: u32,
    /// +174 (`native_component_mask` with the caller's completed Performer
    /// ability count).
    pub component_mask: u32,
    /// +1AC.
    pub name_list: Option<NameListInputs>,
    /// +1B0.
    pub numbered: bool,
    /// +25C / +260.
    pub tracks: Vec<TrackInfoInputs>,
    /// +268 / +270.
    pub trailer_actor: Option<String>,
    pub node_offset_bits: [u32; 3],
    /// +2F0 (the layout IDS; the layout object is the layout owner's).
    pub layout_ids: Option<String>,
    /// +4E0 (non-null when the IDS is set).
    pub required_object_ids: Option<String>,
    /// +5B4.
    pub zone_radius_bits: u32,
}

fn string(v: Option<TypedValue>) -> Option<String> {
    match v {
        Some(TypedValue::String { value, .. })
            if !value.is_empty() && !value.eq_ignore_ascii_case("null") =>
        {
            Some(value)
        }
        _ => None,
    }
}

impl ActorCatalog {
    /// The object-definition fields of fresh creation (ObjectData,
    /// DynamicDoodadData and ActorData definitions).
    pub fn native_object_creation_inputs(
        &self,
        id: &str,
        pc: Precision,
    ) -> Result<ObjectCreationInputs> {
        let category = self.native_category(id)?;
        let object = self.native_object_definition(id, pc)?;
        let settlement_space = match self.native_field(id, "needs_space_for_settlement_zone", pc)? {
            Some(v) => v.boolean()?,
            None => {
                return Err(Error::Unsupported(
                    "needs_space_for_settlement_zone must be declared (5B5380 GetBoolean)".into(),
                ));
            }
        };
        let mut doodad = [None, None];
        if category.native_kind == 21 {
            let d = self.native_dynamic_doodad(id, pc)?;
            for (slot, c) in d.components.iter().enumerate() {
                if let Some(c) = c {
                    doodad[slot] = Some(c.fields["update_time"].fixed_bits()?);
                }
            }
        }
        Ok(ObjectCreationInputs {
            id: object.id.clone(),
            runtime_class: category.runtime_class,
            height_bits: object.height.bits,
            scale_min_bits: object.scale_min.bits,
            scale_max_bits: object.scale_max.bits,
            settlement_space,
            shape_a: object.blocking.words,
            shape_b: object.soft.words,
            shape_target: object.target.words,
            explicit_target: object.has_explicit_target,
            doodad_update_time_bits: doodad,
        })
    }

    /// A NameList entry (`name_list_ids`); None when no NameList has the IDS
    /// (the native `463FA0` null, logged "Unknown IDS '%s' for data manager
    /// '%s'" by the loader).
    pub fn native_name_list(&self, ids: &str, pc: Precision) -> Result<Option<NameListInputs>> {
        self.native_name_list_with_lookup(ids, pc, &mut |_| {
            Err(Error::Unsupported(
                "NameList requires a localization lookup (4C19D0 -> 5B5620 -> 4B9F40)".into(),
            ))
        })
    }

    /// Original 4C19D0 over Name children in 5B4220/42BE10 order. A `#key`
    /// calls the supplied 4B9F40 lookup; a missing key becomes the original
    /// empty default. Plain-string fields remain native type errors.
    pub fn native_name_list_with_lookup(
        &self,
        ids: &str,
        pc: Precision,
        lookup: &mut super::display_name::Lookup<'_>,
    ) -> Result<Option<NameListInputs>> {
        let Some(node) = self.native_named_definition("NameList", ids, pc)? else {
            return Ok(None);
        };
        let mut names = Vec::new();
        for child in node.native_name_list_children()? {
            let text = child.native_name_list_text()?;
            let value = if let Some(key) = text.strip_prefix('#') {
                String::from_utf16(
                    &lookup(&key.encode_utf16().collect::<Vec<_>>())?.unwrap_or_default(),
                )
                .map_err(|_| Error::Unsupported("NameList contains unpaired UTF-16 units".into()))?
            } else {
                text
            };
            names.push(value);
        }
        Ok(Some(NameListInputs {
            ids: ids.to_owned(),
            names,
        }))
    }

    /// The ActorData fields of actor creation. `performer_ability_count` is
    /// the completed native Ability load count (see `native_component_mask`).
    pub fn native_actor_creation_inputs(
        &self,
        id: &str,
        performer_ability_count: Option<u32>,
        pc: Precision,
    ) -> Result<ActorCreationInputs> {
        self.native_actor_creation_inputs_with_lookup(id, performer_ability_count, pc, &mut |_| {
            Err(Error::Unsupported(
                "actor NameList requires a localization lookup (4B9F40)".into(),
            ))
        })
    }

    /// Creation inputs with an explicit 4B9F40 localization owner for any
    /// referenced NameList. Does not fabricate untranslated runtime names.
    pub fn native_actor_creation_inputs_with_lookup(
        &self,
        id: &str,
        performer_ability_count: Option<u32>,
        pc: Precision,
        lookup: &mut super::display_name::Lookup<'_>,
    ) -> Result<ActorCreationInputs> {
        if self.native_category(id)?.native_kind != 2 {
            return Err(Error::Unsupported(
                "actor creation inputs require standard ActorData".into(),
            ));
        }
        let object = self.native_object_creation_inputs(id, pc)?;
        let creation = self.native_creation_definition(id, pc)?;
        let flags = self.native_final_flags(id, pc)?;
        let component_mask = self
            .native_component_mask_at(id, performer_ability_count, pc)?
            .bits;
        let numbered = match self.native_field(id, "numbered", pc)? {
            Some(v) => v.boolean()?,
            None => return Err(Error::Unsupported("numbered must be declared".into())),
        };
        let name_list = match string(self.native_field(id, "name_list_ids", pc)?) {
            Some(ids) => self.native_name_list_with_lookup(&ids, pc, lookup)?,
            None => None,
        };
        // TrackInfo, trailer_actor and TrailerHitch are MoverComponent fields
        // (464BD0 reads them on the Mover node).
        let node = self.resolved_actor(id)?;
        let mover = node.child("MoverComponent")?;
        let mut tracks = Vec::new();
        let track_nodes = match &mover {
            Some(m) => m.children("TrackInfo")?,
            None => Vec::new(),
        };
        for t in track_nodes {
            let kind = match t.field("track_type", pc)? {
                Some(TypedValue::String { value, .. }) => match value.to_ascii_uppercase().as_str()
                {
                    "DISCRETE" => 0,
                    "CONTINUOUS" => 1,
                    _ => {
                        return Err(Error::Unsupported(format!(
                            "Unknown track type '{value}' on object '{id}'"
                        )));
                    }
                },
                _ => {
                    return Err(Error::Unsupported(
                        "TrackInfo.track_type must be a string".into(),
                    ));
                }
            };
            let fade = match t.field("fade_sec", pc)? {
                Some(TypedValue::Scalar(s)) => s.bits,
                _ => {
                    return Err(Error::Unsupported(
                        "TrackInfo.fade_sec must be a scalar".into(),
                    ));
                }
            };
            tracks.push(TrackInfoInputs {
                kind,
                fade_sec_bits: fade,
            });
        }
        let trailer_actor = match &mover {
            Some(_) => string(self.native_field(id, "MoverComponent.trailer_actor", pc)?),
            None => None,
        };
        let hitch = match &mover {
            Some(m) => m.child("TrailerHitch")?,
            None => None,
        };
        let node_offset_bits = match hitch
            .as_ref()
            .map(|h| h.field("node_offset", pc))
            .transpose()?
            .flatten()
        {
            Some(TypedValue::Point { bits, .. }) if bits.len() == 3 => [bits[0], bits[1], bits[2]],
            None => [0; 3],
            _ => {
                return Err(Error::Unsupported(
                    "TrailerHitch.node_offset must be a point3".into(),
                ));
            }
        };
        if trailer_actor.is_some() && hitch.is_none() {
            return Err(Error::Unsupported(format!(
                "Trailer actor specified but no trailer hitch specified - actor '{id}'"
            )));
        }
        let layout_ids = if component_mask & (1 << 7) != 0 {
            string(self.native_field(id, "OrganizationComponent.layout_IDS", pc)?)
        } else {
            None
        };
        let required_object_ids = string(self.native_field(id, "required_object_ids", pc)?);
        // ActorData constructor 4624B0 zeroes +5B4; the City loader stores it.
        let zone_radius_bits = self.native_city_zone_radius(id, pc)?.unwrap_or(0);
        Ok(ActorCreationInputs {
            object,
            flags,
            variations_enabled: creation.variations_enabled,
            aspect_weights_bits: creation.variation_weights_bits,
            total_weight_bits: creation.total_variation_weight_bits,
            component_mask,
            name_list,
            numbered,
            tracks,
            trailer_actor,
            node_offset_bits,
            layout_ids,
            required_object_ids,
            zone_radius_bits,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        actors::{
            ActorCatalog,
            native_load::{NativeLoadReport, load_files},
        },
        sources::{Origin, ResolvedFile, SourceLimits, V14Sources, sha256},
        tgi::Limits,
    };
    use aa_numeric::Precision;
    use serde_json::Value;

    fn catalog(text: &str) -> ActorCatalog {
        ActorCatalog::from_files(
            vec![ResolvedFile {
                logical_path: "objects.tgi".into(),
                origin: Origin::V14Loose,
                sha256: sha256(text.as_bytes()),
                bytes: text.as_bytes().to_vec(),
            }],
            Limits::default(),
        )
    }
    fn n(v: &Value) -> u32 {
        v.as_u64().unwrap() as u32
    }
    fn lit(bits: &Value) -> String {
        format!("{}", f32::from_bits(n(bits)))
    }

    fn unhex(text: &str) -> Vec<u8> {
        text.as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    /// Original whole parser 5BD080, Name iterator 5B4220/42BE10,
    /// localized getter 5B5620 and NameList append 4C19D0/4C1910 all run
    /// natively. Only the text-table lookup is the declared fixture seam.
    #[test]
    fn name_lists_match_original_parser_and_loader() {
        let doc: Value = serde_json::from_str(include_str!(
            "../../../research/phase8-actor-creation/native-name-lists.json"
        ))
        .unwrap();
        assert_eq!(
            doc["original_sha256"],
            "5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26"
        );
        for row in doc["cases"].as_array().unwrap() {
            let label = row["label"].as_str().unwrap();
            let bytes = unhex(row["source_hex"].as_str().unwrap());
            assert_eq!(sha256(&bytes), row["source_sha256"], "{label} source");
            let file = ResolvedFile {
                logical_path: format!("{label}.tgi"),
                origin: Origin::V14Loose,
                sha256: sha256(&bytes),
                bytes,
            };
            let cat = load_files(
                vec![file],
                Limits::default(),
                &mut NativeLoadReport::default(),
            );
            let id = if label == "real_ukraine" {
                "namelist_ukraine"
            } else {
                "test"
            };
            let mut lookup = |key: &[u16]| {
                let key = String::from_utf16(key).unwrap();
                Ok(row["lookup"]
                    .get(&key)
                    .and_then(Value::as_str)
                    .map(|text| text.encode_utf16().collect()))
            };
            let result = cat.native_name_list_with_lookup(id, Precision::Double53, &mut lookup);
            if !row["result"]["error"].is_null() || !row["result"]["parse_error"].is_null() {
                assert!(result.is_err(), "{label}: native rejected this Name.name");
            } else {
                let result = result.unwrap_or_else(|e| panic!("{label}: {e}")).unwrap();
                assert_eq!(
                    serde_json::to_value(result.names).unwrap(),
                    row["result"]["names"],
                    "{label}"
                );
            }
            if label == "real_ukraine" || label == "localization" {
                assert!(
                    cat.native_name_list(id, Precision::Double53).is_err(),
                    "a missing localization owner must not return raw #keys as runtime names"
                );
            }
        }
    }

    /// research/phase8-actor-creation/native-object-definition.json: the
    /// original 49DC40 Size block on fixture nodes; the same field values
    /// through the TGI loader must give the same definition words.
    #[test]
    fn object_definition_fields_match_the_native_loader() {
        let doc: Value = serde_json::from_str(include_str!(
            "../../../research/phase8-actor-creation/native-object-definition.json"
        ))
        .unwrap();
        let mut checked = 0;
        for row in doc["cases"].as_array().unwrap() {
            let label = row["label"].as_str().unwrap();
            let case = &row["case"];
            let r = &row["result"];
            let pc = match case["pc"].as_str().unwrap() {
                "Single24" => Precision::Single24,
                "Extended64" => Precision::Extended64,
                _ => Precision::Double53,
            };
            let root = &case["root"];
            let needs = root.get("needs_space_for_settlement_zone").map_or(0, n);
            let mut text = format!(
                "[Thing]{{\nIDS=test\nstring category=OBJECT\nbool needs_space_for_settlement_zone={}\n",
                if needs != 0 { "true" } else { "false" }
            );
            let size = &case["size"];
            if size.is_null() {
                // No Size node: the native keeps the constructor fields; the
                // Rust loader requires Size (explicit error, recorded gap).
                text.push_str("}\n");
                assert!(
                    catalog(&text)
                        .native_object_creation_inputs("test", pc)
                        .is_err(),
                    "{label}"
                );
                continue;
            }
            text.push_str("[Size]{\n");
            for (k, v) in size.as_object().unwrap() {
                match v {
                    Value::String(s) => text.push_str(&format!(
                        "string {k}={}\n",
                        if s.is_empty() { "null" } else { s }
                    )),
                    v => text.push_str(&format!("fixed {k}={}\n", lit(v))),
                }
            }
            text.push_str("}\n}\n");
            let cat = catalog(&text);
            let out = cat.native_object_creation_inputs("test", pc);
            if !r["outcome"].is_null() {
                assert!(out.is_err(), "{label}: native {}", r["outcome"]);
                continue;
            }
            let out = out.unwrap_or_else(|e| panic!("{label}: {e}"));
            let w = |off: u32| n(&r["words"][format!("{off:#x}")]);
            assert_eq!(out.height_bits, w(0x24), "{label} +24");
            assert_eq!(out.scale_min_bits, w(0x28), "{label} +28");
            assert_eq!(out.scale_max_bits, w(0x2c), "{label} +2C");
            assert_eq!(
                u32::from(out.settlement_space),
                n(&r["byte_34"]),
                "{label} +34"
            );
            for i in 0..6u32 {
                assert_eq!(
                    out.shape_a[i as usize],
                    w(0x38 + 4 * i),
                    "{label} shape A {i}"
                );
                assert_eq!(
                    out.shape_b[i as usize],
                    w(0x50 + 4 * i),
                    "{label} shape B {i}"
                );
                assert_eq!(
                    out.shape_target[i as usize],
                    w(0x68 + 4 * i),
                    "{label} target {i}"
                );
            }
            assert_eq!(
                u32::from(out.explicit_target),
                n(&r["byte_80"]),
                "{label} +80"
            );
            checked += 1;
        }
        assert!(checked >= 96);
    }

    /// Read-only source projection: compare the preserved nine-actor input
    /// baseline, then exercise the three later additions including the
    /// 48-name Ukraine list. Localization deliberately echoes #keys here;
    /// this diagnostic is NOT a loaded/localized runtime definition.
    /// AA_EXPORT_ACTOR_CREATION_SOURCE may explicitly name a NEW output
    /// file. The preserved real-definitions.json is never overwritten.
    #[test]
    #[ignore = "requires original/v14 files via AA_GAME_ROOT"]
    fn real_actor_creation_source_projection_matches_preserved_baseline() {
        let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
        let mut sources = V14Sources::open(
            root.join("Data.rwd"),
            root.join("reconstruction/native-game/runtime-expanded-v14"),
            SourceLimits {
                max_member_bytes: 64 * 1024 * 1024,
            },
        )
        .unwrap();
        let catalog = ActorCatalog::load_v14(&mut sources).unwrap();
        let pc = Precision::Double53;
        let mut out = serde_json::Map::new();
        let ids = [
            "usa_infantry",
            "usa_infantry_mg",
            "ger_infantry",
            "ger_tank_medium",
            "ger_tank_light",
            "ger_tank_heavy",
            "regiment_usa_infantry",
            "regiment_ger_tank_medium",
            "regiment_ger_tank_light",
            "gbr_truck_infantry",
            "gbr_trailer_infantry",
            "ukraine_city01",
        ];
        for id in ids {
            // Every listed actor authors at least one Performer Ability; the
            // completed native count only matters through count == 0
            // (finalize_component_mask), so 1 stands for "loaded".
            let performer = catalog
                .native_performer_inputs(id, pc)
                .unwrap_or_else(|e| panic!("{id} performer: {e}"))
                .map(|_| 1);
            let mut source_keys = |key: &[u16]| {
                let mut out = vec![u16::from(b'#')];
                out.extend_from_slice(key);
                Ok(Some(out))
            };
            let inputs = catalog
                .native_actor_creation_inputs_with_lookup(id, performer, pc, &mut source_keys)
                .unwrap_or_else(|e| panic!("{id} creation inputs: {e}"));
            let mut v = serde_json::to_value(&inputs).unwrap();
            v["performer_ability_count_assumed"] = serde_json::json!(performer);
            out.insert(id.to_owned(), v);
        }
        let baseline: Value = serde_json::from_str(include_str!(
            "../../../research/phase8-actor-creation/real-definitions.json"
        ))
        .unwrap();
        assert_eq!(baseline.as_object().unwrap().len(), 9);
        for (id, expected) in baseline.as_object().unwrap() {
            assert_eq!(
                &out[id], expected,
                "{id}: preserved creation input baseline changed"
            );
        }
        let oracle: Value = serde_json::from_str(include_str!(
            "../../../research/phase8-actor-creation/native-name-lists.json"
        ))
        .unwrap();
        let ukraine = oracle["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["label"] == "real_ukraine")
            .unwrap();
        let source = sources.read("Namelists/namelist_ukraine.tgi").unwrap();
        assert_eq!(
            source.sha256, ukraine["source_sha256"],
            "Ukraine NameList source changed"
        );
        let names: Vec<String> = ukraine["result"]["lookups"]
            .as_array()
            .unwrap()
            .iter()
            .skip(1)
            .map(|key| format!("#{}", key.as_str().unwrap()))
            .collect();
        assert_eq!(
            out["ukraine_city01"]["name_list"]["names"],
            serde_json::json!(names)
        );
        assert_eq!(names.len(), 48);
        println!(
            "compared 9 preserved actors; projected 12 actors; Ukraine NameList: 48 source keys (localization unresolved)"
        );
        if let Some(path) = std::env::var_os("AA_EXPORT_ACTOR_CREATION_SOURCE") {
            use std::io::Write;
            let path = std::path::PathBuf::from(path);
            let doc = serde_json::json!({
                "scope": "authored source projection; NameList #keys are unresolved, not localized runtime names",
                "actors": out,
            });
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .expect("explicit source export requires a new output file; no fixture overwrite");
            file.write_all(serde_json::to_string_pretty(&doc).unwrap().as_bytes())
                .unwrap();
            println!("explicit source projection export: {}", path.display());
        }
    }
}
