use anyhow::{Result,Context,ensure};
use std::collections::BTreeMap;
use aa_assets::AssetStore;
fn use_effective_material_grid(map:&mut aa_assets::TerrainMap,terrain:&aa_runtime::battle_terrain::BattleTerrain)->Result<()>{
    let grid=&terrain.bound.layout;
    ensure!(map.width as i32==grid.width&&map.height as i32==grid.height
        &&map.cell_size.to_bits()==grid.cell_size_bits,
        "terrain rendering grid differs from current World");
    ensure!(map.tiles.len()==terrain.terrain.tiles.len(),"terrain rendering tile count differs");
    let width=map.width as usize;
    ensure!(width>0,"terrain rendering grid is empty");
    material_grid(map,terrain.terrain.tiles.iter().enumerate().map(|(i,tile)|{
        let variation=terrain.variation_definition_at((i%width)as i32,(i/width)as i32)
            .map_err(anyhow::Error::msg)?;
        Ok((u32::from(tile.variation_index()),variation.id.as_str()))
    }))
}

fn material_grid<'a>(map:&mut aa_assets::TerrainMap,
    effective:impl IntoIterator<Item=Result<(u32,&'a str)>>)->Result<()>{
    let mut directory=BTreeMap::<&str,u32>::new();
    let mut palette=BTreeMap::new();
    let mut tiles=Vec::new();
    tiles.try_reserve_exact(map.tiles.len())?;
    for entry in effective{
        let (saved,canonical)=entry?;
        ensure!(map.tiles.get(tiles.len())==Some(&saved),"terrain serialized tile index differs");
        let original=map.palette.get(&saved).context("terrain rendering palette slot absent")?;
        let id=match directory.get(canonical){
            Some(&id)=>id,
            None=>{
                let id=u32::try_from(palette.len()).context("terrain rendering material index overflow")?;
                let mut material=original.clone();
                material.id=canonical.to_owned();
                directory.insert(canonical,id);
                palette.insert(id,material);
                id
            }
        };
        tiles.push(id);
    }
    ensure!(tiles.len()==map.tiles.len(),"terrain rendering tile count differs");
    //This local packet's IDs are only material indices. Canonical metadata and
    //its current source hash are still required before any draw is prepared.
    map.tiles=tiles;
    map.palette=palette;
    Ok(())
}


#[cfg(test)]mod tests{use super::*;
    #[test]
    fn effective_material_grid_preserves_carry_forward_and_canonical_aliases(){
        let entry=|id:&str|aa_assets::TerrainPaletteEntry{id:id.into(),texture:None,
            color:[1.0;3],width:None,height:None};
        let mut map=aa_assets::TerrainMap{id:"carry".into(),name:"carry".into(),description:String::new(),
            source:String::new(),width:6,height:1,cell_size:2.0,biomes:vec![],
            tiles:vec![165,116,165,136,165,7],heights:vec![],mountain_heights:vec![],
            palette:BTreeMap::from([(165,entry("unresolved")),(116,entry("grass")),
                (136,entry("jungle")),(7,entry("grass_alias")),(99,entry("unused"))]),limitations:vec![]};
        let raw=map.clone();
        //The first unresolved tile carries the Biome's initial grass; later
        //ones carry the actual preceding choice. A true alias joins grass.
        material_grid(&mut map,[(165,"grass"),(116,"grass"),(165,"grass"),
            (136,"jungle"),(165,"jungle"),(7,"grass")].into_iter().map(Ok)).unwrap();
        assert_eq!(map.tiles,[0,0,0,1,1,0]);
        assert_eq!(map.palette.len(),2);
        assert_eq!(map.palette[&0].id,"grass");
        assert_eq!(map.palette[&1].id,"jungle");
        assert_eq!(raw.tiles,[165,116,165,136,165,7]);
        assert_eq!(raw.palette[&165].id,"unresolved");
        let unchanged=map.clone();
        assert!(material_grid(&mut map,[Ok((99,"grass"))]).is_err());
        assert_eq!(map,unchanged);
        assert!(material_grid(&mut map,[Ok((0,"grass"))]).is_err());
        assert_eq!(map,unchanged);
    }

    #[test]
    #[ignore="requires AA_GAME_ROOT original/v14 source and converted map"]
    fn firefight_effective_material_grid_preserves_actual_native_tile_choices(){
        use aa_rules::{actors::ActorCatalog,sources::{SourceLimits,V14Sources},tgi::Limits};
        use aa_runtime::{battle_terrain::BattleTerrain,settings::Settings};
        let root=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
        let mut sources=V14Sources::open(root.join("Data.rwd"),
            root.join("reconstruction/native-game/runtime-expanded-v14"),
            SourceLimits{max_member_bytes:u64::MAX}).unwrap();
        let settings=Settings::load_v14(&mut sources,Limits::default()).unwrap();
        let actors=ActorCatalog::load_v14(&mut sources).unwrap();
        let file=sources.read("Maps/DeveloperMaps/(3v3)Firefight.RMP").unwrap();
        let archive=aa_mapworld::MapArchive::from_bytes(file.bytes).unwrap();
        assert_eq!(archive.source().sha256,"c15fd62a581cd36bb8e6131a4ffacba903a62259e0b3c2ae86afee611251e74b");
        let terrain=BattleTerrain::load_with_sources(&archive,&mut sources,&actors,&settings,
            aa_numeric::Precision::Double53).unwrap();
        assert_eq!(terrain.terrain.map_source,*archive.source());
        assert_eq!(terrain.terrain.source.sha256,"084b1f164ec36d6fd6b14c2a4679de623defcfe09ddca68fedf9f572eddc8981");
        assert_eq!(terrain.bound.cells[15].resolved_palette_index,Some(116));
        assert_eq!(terrain.bound.cells[44].resolved_palette_index,Some(136));
        let store=AssetStore::open(root.join("reconstruction")).unwrap();
        let original=store.load_map("DeveloperMaps/(3v3)Firefight").unwrap().terrain;
        let raw_tiles=terrain.terrain.tiles.clone();
        let raw_bytes=terrain.terrain.serialized_tile_bytes.clone();
        let bound_cells=terrain.bound.cells.clone();
        let mut map=original.clone();
        use_effective_material_grid(&mut map,&terrain).unwrap();
        assert_eq!(map.tiles.len(),16384);
        assert_eq!(original.tiles[15],165);assert_eq!(original.tiles[44],165);
        assert_ne!(map.tiles[15],map.tiles[44]);
        assert_eq!(map.palette[&map.tiles[15]].id,"tropical_grass01");
        assert_eq!(map.palette[&map.tiles[44]].id,"tropical_jungle01");
        for (i,&id) in map.tiles.iter().enumerate(){
            let actual=terrain.variation_definition_at((i%128)as i32,(i/128)as i32).unwrap();
            assert_eq!(map.palette[&id].id,actual.id,"tile {i}");
            assert_eq!(u32::from(terrain.terrain.tiles[i].variation_index()),original.tiles[i]);
        }
        assert_eq!(terrain.terrain.tiles,raw_tiles);
        assert_eq!(terrain.terrain.serialized_tile_bytes,raw_bytes);
        assert_eq!(terrain.bound.cells,bound_cells);
        assert_eq!(store.load_map("DeveloperMaps/(3v3)Firefight").unwrap().terrain,original);
        let metadata:serde_json::Value=serde_json::from_str(include_str!(
            "E:/Games/Axis and Alies/rust-rebuild/research/phase6-terrain/terrain-render-metadata.json")).unwrap();
        let mut priorities=BTreeMap::new();
        for (&id,material) in &map.palette{
            let v=&metadata["variants"][&material.id];
            assert_eq!(v["canonical"].as_str(),Some(material.id.as_str()));
            let actual=actors.native_data_find("Variation",&material.id).unwrap();
            assert!(actual.source.path.eq_ignore_ascii_case(v["path"].as_str().unwrap()));
            assert_eq!(actual.source.sha256,v["sha256"].as_str().unwrap());
            priorities.insert(id,v["priority"].as_i64().unwrap() as i32);
        }
        //The new IDs remain suitable for the unchanged native mask planner:
        //every interior neighborhood resolves its actual canonical materials.
        let planner=aa_assets::TerrainLayerPlanner::bundled().unwrap();
        for y in 1..127{for x in 1..127{
            let grid=std::array::from_fn(|i|{
                let id=map.tiles[(y+i/3-1)*128+x+i%3-1];
                aa_assets::TerrainLayerInput{palette_index:id,priority:priorities[&id]}
            });
            let layers=planner.plan(grid).unwrap();
            assert!(layers.iter().all(|layer|map.palette.contains_key(&layer.palette_index)));
        }}
    }


}
