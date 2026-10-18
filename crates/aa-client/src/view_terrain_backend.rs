//!Modern terrain GPU stage for the actual retained battle World.
//!
//!This intentionally replaces the old Ni terrain GPU allocation. It does not
//!claim723E00 ABI/rendering parity: border skirts, cloud projection and cached
//!render-target quantization remain visual gaps. Simulation terrain, collision,
//!RNG and actor state are never changed. Heights come from the SAME completed
//!431BC0 visual heightfield; material layers use the existing native mask port.
use crate::{gpu::{CpuDraw,CpuScene,SceneRenderer,RendererWitness,Vertex},loading::LoadSettings};
use aa_assets::{AssetStore,Bounds,HeightMode,PreparedTerrain,TerrainVertex};
use aa_runtime::{catalog::Catalog,live_world_frame::World,view_heightfield,
    world_revision::Witness};
use aa_mapworld::SourceIdentity;
use anyhow::{Context,Result,ensure};
use std::{collections::BTreeMap,time::Instant};

///Sendable immutable rendering data. It contains no mutable World/actor clone
///and cannot authorize orders, simulation or a completed VWorld.
pub struct Prepared{scene:CpuScene,world:Witness,map:SourceIdentity,height:view_heightfield::SampleWitness}
///Minted only after the actual renderer accepted this terrain scene.
pub struct Installed{world:Witness,map:SourceIdentity,renderer:RendererWitness,height:view_heightfield::SampleWitness}
impl aa_runtime::view_init_world::TerrainBinding for Installed{
    fn validate(&self,world:&World<'_>,height:&view_heightfield::State)->std::result::Result<(),String>{
        self.validate_world(world).map_err(|e|e.to_string())?;
        let p=world.presentation_parts()?;
        if !self.height.is_for(height,p.revision){return Err("terrain GPU stage belongs to another visual heightfield allocation".into())}
        Ok(())
    }
}
impl Installed{
    pub fn validate_world(&self,world:&World<'_>)->Result<()>{
        let p=world.presentation_parts().map_err(anyhow::Error::msg)?;
        ensure!(self.world.is_for(p.revision)&&self.map==p.inventory.map_source,
            "terrain GPU stage belongs to another or retired World");
        ensure!(self.renderer.is_current(),"terrain GPU stage renderer was retired or reloaded");
        Ok(())
    }
    pub(crate) fn validate_renderer(&self,renderer:&SceneRenderer)->Result<()>{
        ensure!(self.renderer.is_for(renderer),"terrain GPU stage belongs to another renderer");Ok(())
    }
}
impl Prepared{
    ///Run on the retained simulation thread. Only the immutable result crosses
    ///to the UI/backend thread; every current owner is borrowed while preparing.
    pub fn from_world(world:&World<'_>,catalog:&Catalog<'_>,height:&view_heightfield::State,
        store:&AssetStore,map_id:&str,settings:&LoadSettings)->Result<Self>{
        let start=Instant::now();
        let p=world.presentation_parts().map_err(anyhow::Error::msg)?;
        catalog.require_source_owner().map_err(anyhow::Error::msg)?;
        p.definitions.combat_source().map_err(anyhow::Error::msg)?.completed_means().map_err(anyhow::Error::msg)?;
        let info=store.maps().iter().find(|m|m.id==map_id).context("current terrain map selection is absent")?;
        let raw=aa_mapworld::MapArchive::from_bytes(std::fs::read(
            store.resolve_url(&format!("/assets/original/Data/{}",info.source))?)?)?;
        ensure!(raw.source()==&p.inventory.map_source,"terrain GPU selection differs from current World map");
        let mut map=store.load_map(map_id)?.terrain;
        map.biomes=vec![String::from_utf16(&p.terrain.terrain.biome_utf16)?];
        use_effective_material_grid(&mut map,p.terrain)?;
        crate::terrain_render::require_current_materials(&map,catalog)?;
        let samples=height.samples_for_world(p.revision).map_err(anyhow::Error::msg)?;
        let coarse=coarse_vertices(&map,&samples)?;
        let terrain=PreparedTerrain{width:map.width,height:map.height,cell_size:map.cell_size,
            height_mode:HeightMode::NativeConditional,bounds:Bounds::from_positions(coarse.iter().map(|v|&v.position)).context("terrain has no vertices")?,
            vertices:coarse,batches:Vec::new(),warnings:Vec::new()};
        let (mut draws,mut warnings)=crate::terrain_render::build_parts(&map,&terrain,settings.memory_policy)?;
        tessellate(&mut draws,&samples)?;
        let textures=crate::loading::load_textures(store,&draws,settings)?;
        warnings.push("Battle terrain uses current native visual samples and transition masks with a modern GPU backend. Native cached-target quantization, border skirts, clouds and shadows remain incomplete.".into());
        let scene=CpuScene{objects:None,live:None,name:map.name,map_id:map_id.into(),
            size:[map.width as f32*map.cell_size,map.height as f32*map.cell_size],
            draws,textures,placements:0,rendered_instances:0,warnings,load_seconds:start.elapsed().as_secs_f64()};
        Ok(Self{scene,world:p.revision.witness(),map:p.inventory.map_source.clone(),height:height.witness_for_world(p.revision).map_err(anyhow::Error::msg)?})
    }
    ///Backend-thread operation. Upload success produces only a terrain-stage
    ///receipt, never a simulated native VWorld completion or mutable actor view.
    pub fn install(self,renderer:&mut SceneRenderer)->Result<Installed>{
        renderer.upload(&self.scene)?;
        Ok(Installed{world:self.world,map:self.map,renderer:renderer.identity_witness(),height:self.height})
    }
    ///The full static-map packet uses this operation for its ONE initial
    ///upload. Original static objects/picking stay in that packet; stale coarse
    ///terrain is replaced before uploading, never drawn twice or uploaded later
    ///under a second epoch that invalidates the other installed receipts.
    pub(crate) fn install_into_scene(self,scene:&mut CpuScene,source:&SourceIdentity,
        renderer:&mut SceneRenderer)->Result<Installed>{
        ensure!(source==&self.map&&scene.map_id==self.scene.map_id&&scene.size==self.scene.size,
            "static-map packet and current terrain refer to different maps");
        scene.draws.retain(|d|!d.material.terrain_layer);
        scene.draws.extend(self.scene.draws);
        scene.textures.extend(self.scene.textures);
        let used=scene.draws.iter().flat_map(|d|d.material.texture.iter().map(String::as_str)
            .chain(d.material.terrain_mask.iter().map(|m|m.texture.as_str()))).collect::<std::collections::BTreeSet<_>>();
        scene.textures.retain(|key,_|used.contains(key.as_str()));
        scene.warnings.extend(self.scene.warnings);
        renderer.upload(scene)?;
        Ok(Installed{world:self.world,map:self.map,renderer:renderer.identity_witness(),height:self.height})
    }
    pub fn scene(&self)->&CpuScene{&self.scene}
}

///Native557D4B..557E3C can carry different preceding Variation pointers for
///different tiles with the SAME unresolved serialized palette slot. Preserve
///those actual per-tile choices in a rendering-only directory; never rewrite
///the retained World tiles, source palette or native carry-forward state.
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

fn coordinate(value:f32,step:f32,limit:usize)->Result<usize>{
    ensure!(value.is_finite()&&step.is_finite()&&step>0.0,"terrain sample coordinate is invalid");
    let n=value/step;
    ensure!(n>=0.0&&n.fract()==0.0&&n<(limit as f32),"terrain tile is not aligned to the actual visual sample grid");
    Ok(n as usize)
}
fn point(s:&view_heightfield::Samples<'_>,x:usize,y:usize)->Result<[f32;3]>{
    ensure!(x<s.columns&&y<s.rows,"terrain sample is outside current visual storage");
    let step=f32::from_bits(s.step_bits);
    let h=f32::from_bits(*s.heights.get(y*s.columns+x).context("terrain sample storage is incomplete")?);
    let p=[x as f32*step,h,-(y as f32*step)];
    ensure!(p.iter().all(|v|v.is_finite()),"terrain sample is nonfinite");Ok(p)
}
fn normal(s:&view_heightfield::Samples<'_>,x:usize,y:usize)->Result<[f32;3]>{
    let xa=x.saturating_sub(1);let xb=(x+1).min(s.columns-1);
    let ya=y.saturating_sub(1);let yb=(y+1).min(s.rows-1);
    ensure!(xa!=xb&&ya!=yb,"terrain normal requires two sample dimensions");
    let step=f32::from_bits(s.step_bits);
    let hx=point(s,xb,y)?[1]-point(s,xa,y)?[1];
    let hy=point(s,x,yb)?[1]-point(s,x,ya)?[1];
    let n=glam::Vec3::new(-hx/((xb-xa)as f32*step),1.0,hy/((yb-ya)as f32*step)).normalize();
    ensure!(n.is_finite(),"terrain normal is nonfinite");Ok(n.to_array())
}
fn coarse_vertices(map:&aa_assets::TerrainMap,s:&view_heightfield::Samples<'_>)->Result<Vec<TerrainVertex>>{
    let step=f32::from_bits(s.step_bits);let mut out=Vec::new();
    out.try_reserve_exact(map.tiles.len().checked_mul(4).context("terrain vertex size overflow")?)?;
    for i in 0..map.tiles.len(){
        let x=i%map.width as usize;let y=i/map.width as usize;
        for (dx,dy) in [(0,0),(1,0),(0,1),(1,1)]{
            let gx=coordinate((x+dx)as f32*map.cell_size,step,s.columns)?;
            let gy=coordinate((y+dy)as f32*map.cell_size,step,s.rows)?;
            out.push(TerrainVertex{position:point(s,gx,gy)?,normal:normal(s,gx,gy)?,uv:[0.0;2],color:[1.0;4]});
        }
    }
    Ok(out)
}
///Material masks retain their original whole-tile coordinates while geometry
///is split on the current visual sample grid. CPU float normals are a modern
///rendering calculation; they are never a physics or native parity assertion.
fn tessellate(draws:&mut[CpuDraw],s:&view_heightfield::Samples<'_>)->Result<()>{
    let step=f32::from_bits(s.step_bits);
    for draw in draws{
        ensure!(draw.vertices.len()%4==0,"terrain material mesh does not contain quads");
        let mut vertices=Vec::new();let mut indices=Vec::new();
        for quad in draw.vertices.chunks_exact(4){
            let x0=coordinate(quad[0].position[0],step,s.columns)?;
            let x1=coordinate(quad[1].position[0],step,s.columns)?;
            let y0=coordinate(-quad[0].position[2],step,s.rows)?;
            let y1=coordinate(-quad[2].position[2],step,s.rows)?;
            ensure!(x1>x0&&y1>y0,"terrain material quad has no area");
            let count=(x1-x0).checked_mul(y1-y0).context("terrain tessellation count overflow")?;
            vertices.try_reserve(count.checked_mul(4).context("terrain vertex size overflow")?)?;
            indices.try_reserve(count.checked_mul(6).context("terrain index size overflow")?)?;
            for y in y0..y1{for x in x0..x1{
                let first=u32::try_from(vertices.len())?;
                for (dx,dy) in [(0,0),(1,0),(0,1),(1,1)]{
                    let gx=x+dx;let gy=y+dy;
                    let tx=(gx-x0)as f32/(x1-x0)as f32;let ty=(gy-y0)as f32/(y1-y0)as f32;
                    vertices.push(Vertex{position:point(s,gx,gy)?,normal:normal(s,gx,gy)?,
                        uv:[quad[0].uv[0]+(quad[1].uv[0]-quad[0].uv[0])*tx,
                            quad[0].uv[1]+(quad[2].uv[1]-quad[0].uv[1])*ty],
                        terrain_uv:[tx,1.0-ty],color:[1.0;4]});
                }
                indices.extend([first,first+1,first+2,first+1,first+3,first+2]);
            }}
        }
        draw.vertices=vertices;draw.indices=indices;
    }
    Ok(())
}

#[cfg(test)]
mod tests{
    use super::*;
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
            "../../../research/phase6-terrain/terrain-render-metadata.json")).unwrap();
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

    #[test]
    fn terrain_tessellation_preserves_interior_height_and_whole_tile_mask_coordinates(){
        //An interior peak must survive the modern backend even when all four
        //coarse tile corners are flat. Mask/texture coordinates stay continuous
        //across the new subquad edges, rather than restarting on each subquad.
        let heights=[0.0f32,0.0,0.0,0.0,7.0,0.0,0.0,0.0,0.0].map(f32::to_bits);
        let samples=view_heightfield::Samples{step_bits:1.0f32.to_bits(),columns:3,rows:3,heights:&heights};
        let mut draws=vec![CpuDraw{
            vertices:[([0.0,0.0,0.0],[1.0,1.0]),([2.0,0.0,0.0],[3.0,1.0]),
                ([0.0,0.0,-2.0],[1.0,-1.0]),([2.0,0.0,-2.0],[3.0,-1.0])].map(|(position,uv)|Vertex{
                    position,uv,normal:[0.0,1.0,0.0],terrain_uv:[0.0;2],color:[1.0;4]}).to_vec(),
            indices:vec![0,1,2,1,3,2],instances:vec![crate::gpu::Instance::identity()],
            material:crate::gpu::CpuMaterial{texture:None,color:[1.0;4],emissive:[0.0;3],cutoff:0.0,
                blend:false,double_sided:false,team_color_mask:false,native_render_state:None,
                terrain_layer:true,terrain_mask:None},source_order:None,sort_center:None,
        }];
        tessellate(&mut draws,&samples).unwrap();
        assert_eq!(draws[0].vertices.len(),16);assert_eq!(draws[0].indices.len(),24);
        let center=draws[0].vertices.iter().filter(|v|v.position[0]==1.0&&v.position[2]==-1.0).collect::<Vec<_>>();
        assert_eq!(center.len(),4);
        for v in center{assert_eq!(v.position[1],7.0);assert_eq!(v.uv,[2.0,0.0]);assert_eq!(v.terrain_uv,[0.5,0.5]);}
        assert!(draws[0].vertices.iter().all(|v|v.normal.iter().all(|n|n.is_finite())));
        assert!(coordinate(0.5,1.0,3).is_err());
        assert!(coordinate(-1.0,1.0,3).is_err());
        assert!(coordinate(3.0,1.0,3).is_err());
    }
}
