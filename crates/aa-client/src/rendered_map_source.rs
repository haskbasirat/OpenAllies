//! Immutable correspondence between the selected raw RMP, extracted chunk
//! manifest, and the converted map's terrain samples/palette/placement prefix.
//! This is not a model/material conversion or complete rendering-parity proof.
use aa_assets::{AssetStore, PreparedScene};
use aa_mapworld::{
    MapArchive, SourceIdentity,
    header::MapHeader,
    inventory::Inventory,
    terrain::{TerrainChunk, TerrainGridLayout},
};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug)]
pub(crate) struct RenderedMapSource {
    id: String,
    identity: SourceIdentity,
}
#[derive(Deserialize)]
struct Manifest {
    source: String,
    sha256: String,
    chunks: BTreeMap<String, Chunk>,
}
#[derive(Deserialize)]
struct Chunk {
    size: u64,
    sha256: String,
}

impl RenderedMapSource {
    pub(crate) fn load(store: &AssetStore, scene: &PreparedScene) -> Result<Self> {
        Ok(Self::inputs(store, scene, true)?.0)
    }

    /// The old inspection converter rounded native samples to five decimal
    /// places. Install the verified raw samples and rebuild the actual terrain
    /// vertices before returning a receipt; no recovered file is rewritten.
    pub(crate) fn bind_exact(store: &AssetStore, scene: &mut PreparedScene) -> Result<Self> {
        let (receipt, [heights, mountain_heights]) = Self::inputs(store, scene, false)?;
        let mut map = scene.map.terrain.clone();
        map.heights = heights;
        map.mountain_heights = mountain_heights;
        let terrain = map.prepare(scene.terrain.height_mode)?;
        // Publish together only after every input and the rebuilt vertices pass.
        scene.map.terrain = map;
        scene.terrain = terrain;
        Ok(receipt)
    }

    fn inputs(
        store: &AssetStore,
        scene: &PreparedScene,
        require_exact_samples: bool,
    ) -> Result<(Self, [Vec<f32>; 2])> {
        let map = &scene.map.terrain;
        let placements = &scene.map.placements;
        let info = store
            .maps()
            .iter()
            .find(|i| i.id == map.id)
            .context("rendered map has no canonical AssetStore selection")?;
        ensure!(
            map.id == placements.id && map.source == info.source,
            "rendered terrain/placement selection identity differs"
        );
        let expected = format!("assets/map-chunks/{}/DYNAMIC_THINGS.bin", map.id);
        ensure!(
            placements.source.replace('\\', "/") == expected,
            "converted placement data names another extraction source"
        );
        let source = store.resolve_url(&format!("/assets/original/Data/{}", info.source))?;
        let archive = MapArchive::from_bytes(std::fs::read(source)?)?;
        let manifest_path =
            store.resolve_url(&format!("/assets/map-chunks/{}/manifest.json", map.id))?;
        let manifest: Manifest = serde_json::from_slice(&std::fs::read(manifest_path)?)?;
        ensure!(
            manifest.sha256 == archive.source().sha256,
            "current raw RMP differs from the converted map extraction manifest"
        );
        let suffix = format!("assets/original/Data/{}", info.source).to_ascii_lowercase();
        ensure!(
            manifest
                .source
                .replace('\\', "/")
                .to_ascii_lowercase()
                .ends_with(&suffix),
            "map extraction manifest names another raw source"
        );
        let names = [
            "MAP_HEADER",
            "TERRAIN",
            "HEIGHT_MAP",
            "MOUNTAIN_HEIGHT_MAP",
            "DYNAMIC_THINGS",
            "DYNAMIC_THINGS_INDEX",
        ];
        for name in names {
            let raw = archive.decode_chunk(name, u64::MAX)?;
            let receipt = manifest
                .chunks
                .get(name)
                .context("extracted map chunk receipt is absent")?;
            ensure!(
                raw.payload_source.sha256 == receipt.sha256
                    && raw.bytes.len() as u64 == receipt.size,
                "raw {name} differs from the extraction manifest"
            );
            let path = store.resolve_url(&format!("/assets/map-chunks/{}/{name}.bin", map.id))?;
            ensure!(
                SourceIdentity::of(&std::fs::read(path)?) == raw.payload_source,
                "extracted {name} bytes differ from the selected raw RMP"
            );
        }
        let header = MapHeader::decode(&archive.decode_chunk("MAP_HEADER", u64::MAX)?)?;
        let terrain = TerrainChunk::decode(&archive.decode_chunk("TERRAIN", u64::MAX)?)?;
        let layout = TerrainGridLayout::from_world_extents(
            header.creator.world_extents_bits,
            terrain.resolution_bits,
        )?;
        ensure!(
            map.width as i32 == layout.width
                && map.height as i32 == layout.height
                && map.cell_size.to_bits() == layout.cell_size_bits
                && map.tiles.len() == terrain.tiles.len(),
            "converted terrain grid differs from its raw map"
        );
        for (&converted, raw) in map.tiles.iter().zip(&terrain.tiles) {
            ensure!(
                converted == u32::from(raw.variation_index()),
                "converted terrain tile indices are stale"
            );
            let name = String::from_utf16(
                terrain
                    .palette_utf16
                    .get(converted as usize)
                    .context("raw terrain palette index is invalid")?,
            )?;
            let entry = map
                .palette
                .get(&converted)
                .context("converted terrain palette entry is absent")?;
            ensure!(
                entry.id.eq_ignore_ascii_case(&name),
                "converted terrain variation identity is stale"
            );
        }
        let mut samples = [Vec::new(), Vec::new()];
        for (ordinal, (name, values)) in [
            ("HEIGHT_MAP", &map.heights),
            ("MOUNTAIN_HEIGHT_MAP", &map.mountain_heights),
        ]
        .into_iter()
        .enumerate()
        {
            let chunk = archive.decode_chunk(name, u64::MAX)?;
            let resolution = u32::from_le_bytes(
                chunk
                    .bytes
                    .get(16..20)
                    .context("raw height resolution is absent")?
                    .try_into()?,
            );
            let layout = aa_heightfield::binding::GridLayout::from_world_extents(
                header.creator.world_extents_bits,
                resolution,
            )?;
            let field = aa_heightfield::binding::LoadedHeightfield::decode(layout, &chunk.bytes)?
                .state
                .field;
            ensure!(
                field.width == map.width as i32 + 1
                    && field.height == map.height as i32 + 1
                    && field.cell_size_bits == map.cell_size.to_bits()
                    && field.height_bits.len() == values.len(),
                "converted {name} layout differs from the selected raw map"
            );
            ensure!(
                !require_exact_samples
                    || field
                        .height_bits
                        .iter()
                        .copied()
                        .eq(values.iter().map(|v| v.to_bits())),
                "converted {name} samples differ from the selected raw map"
            );
            samples[ordinal] = field.height_bits.into_iter().map(f32::from_bits).collect();
        }
        let inventory = Inventory::decode(&archive, u64::MAX)?;
        ensure!(
            inventory.unsupported.is_empty()
                && placements.errors.is_empty()
                && placements.expected_records == inventory.placements.len()
                && placements.actors.len() == inventory.placements.len(),
            "converted placements do not cover the raw indexed records"
        );
        for (raw, converted) in inventory.placements.iter().zip(&placements.actors) {
            ensure!(
                raw.record_ordinal < placements.actors.len()
                    && raw.serialized_id == converted.instance_id
                    && raw.class_code == converted.class_code
                    && String::from_utf16(&raw.thing_id_utf16)? == converted.thing_id
                    && raw.source_span.offset as u64 == converted.record_offset
                    && raw.source_span.length as u64 == converted.record_length
                    && raw.thing.version == converted.thing_version
                    && raw.thing.word_1c == converted.base_field
                    && raw.thing.position_bits == converted.native_position().map(f32::to_bits),
                "converted placement identity/position prefix differs from raw indexed record"
            );
            ensure!(
                raw.thing
                    .facing_bits
                    .into_iter()
                    .zip(converted.orientation)
                    .all(|(raw, converted)| converted
                        .map_or(!f32::from_bits(raw).is_finite(), |v| v.to_bits() == raw)),
                "converted placement facing differs from the raw indexed record"
            );
        }
        Ok((
            Self {
                id: map.id.clone(),
                identity: archive.source().clone(),
            },
            samples,
        ))
    }
    pub(crate) fn id(&self) -> &str {
        &self.id
    }
    pub(crate) fn require_identity(&self, identity: &SourceIdentity) -> Result<()> {
        ensure!(
            &self.identity == identity,
            "rendered map source hash differs from the native world"
        );
        Ok(())
    }
}
