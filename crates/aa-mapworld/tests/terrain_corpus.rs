use aa_mapworld::{
    MapArchive, SourceIdentity,
    archive::sha256,
    header::MapHeader,
    terrain::{ResolvedVariation, TerrainChunk, TerrainGridLayout},
};
use serde_json::Value;
use std::path::PathBuf;

#[test]
#[ignore = "requires original RMP and TGI files; read-only"]
fn all_original_terrain_tiles_bind_to_raw_world_layout_and_source_definition_flags() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let data = root.join("../reconstruction/assets/original/Data");
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-spatial/native-real-map-terrain.json"
    ))
    .unwrap();
    let types_bytes =
        std::fs::read(root.join("research/engine-baseline/terrain-variation-metadata.json"))
            .unwrap();
    let metadata_bytes =
        std::fs::read(root.join("research/phase6-terrain/terrain-render-metadata.json")).unwrap();
    assert_eq!(sha256(&types_bytes), fixture["types_metadata_sha256"]);
    assert_eq!(
        sha256(&metadata_bytes),
        fixture["passability_metadata_sha256"]
    );
    let types: Value = serde_json::from_slice(&types_bytes).unwrap();
    let metadata: Value = serde_json::from_slice(&metadata_bytes).unwrap();
    for object in [
        &fixture["terrain_template"],
        &fixture["translation_source"],
        &fixture["resolution_source"],
    ] {
        assert_eq!(
            sha256(&std::fs::read(data.join(object["path"].as_str().unwrap())).unwrap()),
            object["sha256"]
        );
    }
    for (path, hash) in fixture["definition_sources"].as_object().unwrap() {
        assert_eq!(sha256(&std::fs::read(data.join(path)).unwrap()), *hash);
    }
    let resolve = |id: &str| -> Option<ResolvedVariation> {
        let r = metadata["variants"].get(id)?;
        let t = &types["variations"][r["canonical"].as_str().unwrap()];
        let source =
            SourceIdentity::of(&std::fs::read(data.join(r["path"].as_str().unwrap())).unwrap());
        assert_eq!(source.sha256, r["sha256"]);
        assert_eq!(source.sha256, t["source_sha256"]);
        Some(ResolvedVariation {
            id_utf16: id.encode_utf16().collect(),
            native_type: t["native_type"].as_u64().unwrap() as u32,
            passability_bits: u8::from(r["land"].as_bool().unwrap())
                | u8::from(r["water"].as_bool().unwrap()) << 1,
            source,
        })
    };
    let rows = fixture["maps"].as_object().unwrap();
    assert_eq!(rows.len(), 80);
    let mut total = 0;
    for (relative, row) in rows {
        let archive = MapArchive::from_bytes(std::fs::read(data.join(relative)).unwrap()).unwrap();
        assert_eq!(archive.source().sha256, row["map_sha256"]);
        let header_chunk = archive.decode_chunk("MAP_HEADER", u64::MAX).unwrap();
        assert_eq!(header_chunk.payload_source.sha256, row["header_sha256"]);
        let header = MapHeader::decode(&header_chunk).unwrap();
        // Evaluated original SVar is1 for this corpus; do not silently apply it
        // if a future source map explicitly overrides the construction input.
        for rules in [&header.rules, &header.creator.rules] {
            assert!(
                !rules
                    .rules
                    .iter()
                    .any(|r| r.key == "MapGTileResolutionBits".encode_utf16().collect::<Vec<_>>())
            );
        }
        let source = archive.decode_chunk("TERRAIN", u64::MAX).unwrap();
        assert_eq!(source.payload_source.sha256, row["terrain_sha256"]);
        let terrain = TerrainChunk::decode(&source).unwrap();
        let layout =
            TerrainGridLayout::from_world_extents(header.creator.world_extents_bits, 1).unwrap();
        assert_eq!(serde_json::to_value(layout).unwrap(), row["layout"]);
        let palette = terrain
            .palette_utf16
            .iter()
            .map(|s| resolve(&String::from_utf16(s).unwrap()))
            .collect::<Vec<_>>();
        let biome = String::from_utf16(&terrain.biome_utf16).unwrap();
        let initial_id = metadata["biome_defaults"][&biome]["variation_id"]
            .as_str()
            .unwrap();
        assert_eq!(initial_id, row["initial_variation_id"]);
        let bound = terrain
            .bind_with_native_fallback(layout, &palette, resolve(initial_id).unwrap())
            .unwrap();
        assert_eq!(
            bound.cells.len() as u64,
            row["tile_count"].as_u64().unwrap()
        );
        assert_eq!(
            terrain.tile_data_offset as u64,
            row["tile_data_offset"].as_u64().unwrap()
        );
        assert_eq!(
            terrain.suffix_offset as u64,
            row["suffix_offset"].as_u64().unwrap()
        );
        assert_eq!(
            terrain.pending_auxiliary_bytes.len() as u64,
            row["suffix_bytes"].as_u64().unwrap()
        );
        assert_eq!(
            bound.missing_variation_ordinals.len() as u64,
            row["fallback_count"].as_u64().unwrap()
        );
        assert!(bound.has_blocked_outer_ring());
        let flag_bytes = bound
            .cells
            .iter()
            .flat_map(|c| c.flags20.to_le_bytes())
            .collect::<Vec<_>>();
        assert_eq!(sha256(&flag_bytes), row["flags_sha256"]);
        let mut fields = Vec::new();
        for cell in &bound.cells {
            fields.extend(cell.variation_index.to_le_bytes());
            fields.extend(cell.flags20.to_le_bytes());
            fields.push(cell.byte_2c);
            fields.extend(cell.saved_explored_mask.to_le_bytes());
            fields.extend(
                cell.resolved_palette_index
                    .map(u32::from)
                    .unwrap_or(u32::MAX)
                    .to_le_bytes(),
            );
        }
        assert_eq!(sha256(&fields), row["cell_fields_sha256"]);
        let missing = bound
            .missing_variation_ordinals
            .iter()
            .flat_map(|&i| (i as u32).to_le_bytes())
            .collect::<Vec<_>>();
        assert_eq!(sha256(&missing), row["missing_ordinals_sha256"]);
        for (index, native) in row["sampled_source_indices"]
            .as_array()
            .unwrap()
            .iter()
            .zip(row["native_sampled_cells"].as_array().unwrap())
        {
            let cell = &bound.cells[index.as_u64().unwrap() as usize];
            assert_eq!(cell.flags20 as u64, native["flags20"].as_u64().unwrap());
            assert_eq!(
                serde_json::to_value(cell.resolved_palette_index).unwrap(),
                native["variation_index"]
            );
            assert_eq!(cell.byte_2c as u64, native["byte_2c"].as_u64().unwrap());
        }
        total += bound.cells.len();
    }
    assert_eq!(total, 2_960_384);
}
