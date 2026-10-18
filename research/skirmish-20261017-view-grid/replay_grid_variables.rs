use aa_rules::resources::{Provenance,SourceIdentity};
type Result<T>=std::result::Result<T,String>;
pub struct State {
    terrain: i32,
    grid: i32,
    sources: Vec<SourceIdentity>,
    assignments: Vec<(String, i32, Provenance)>,
}
#[derive(Debug, PartialEq, Eq)]
pub struct Metrics {
    pub grid_shift: u32,
    pub terrain_quad_size: i32,
}
impl State {
    pub fn terrain_block_size(&self) -> i32 {
        self.terrain
    }
    pub fn vgrid_cell_size(&self) -> i32 {
        self.grid
    }
    ///Native512160 flags0 stores the exact signed DWORD, without a clamp.
    pub fn assign_terrain_block_size(&mut self, value: i32) {
        self.terrain = value;
    }
    pub fn assign_vgrid_cell_size(&mut self, value: i32) {
        self.grid = value;
    }
}
pub(crate) trait World {
    fn grid(&mut self) -> Result<i32>;
    fn terrain(&mut self) -> Result<i32>;
    fn quad(&mut self) -> Result<i32>;
}
///43FCF1..FD35: both signed mins and the exact unsigned highest-bit loop.
///The unrelated terrain UVar read remains mandatory before computing shift.
pub(crate) fn metrics(world: &mut impl World) -> Result<Metrics> {
    let grid = world.grid()?;
    let terrain = world.terrain()?;
    let selected = if grid > terrain { terrain } else { grid };
    let quad = world.quad()?;
    let terrain_quad_size = if quad > terrain { terrain } else { quad };
    let mut grid_shift = 0u32;
    let selected = selected as u32;
    if selected != 0 {
        while selected
            & !((1u32.wrapping_shl(grid_shift & 31))
                .wrapping_mul(2)
                .wrapping_sub(1))
            != 0
        {
            grid_shift = grid_shift.wrapping_add(1);
        }
    }
    Ok(Metrics {
        grid_shift,
        terrain_quad_size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    #[test]
    fn original_int_constructors_raw_setters_and_metric_read_prefixes() {
        let fixture: Value = serde_json::from_str(include_str!(
            "E:/Games/Axis and Alies/rust-rebuild/research/skirmish-20261017-view-grid/native-grid-variables.json"
        ))
        .unwrap();
        let rows = fixture["cases"].as_array().unwrap();
        assert_eq!(rows.len(), 168);
        struct Probe<'a> {
            input: &'a Value,
            stop: Option<&'a str>,
            events: Vec<&'static str>,
        }
        impl Probe<'_> {
            fn get(&mut self, key: &'static str) -> Result<i32> {
                if self.stop == Some(key) {
                    return Err(key.into());
                }
                self.events.push(key);
                Ok(self.input[key].as_u64().unwrap() as u32 as i32)
            }
        }
        impl World for Probe<'_> {
            fn grid(&mut self) -> Result<i32> {
                self.get("grid")
            }
            fn terrain(&mut self) -> Result<i32> {
                self.get("terrain")
            }
            fn quad(&mut self) -> Result<i32> {
                self.get("quad")
            }
        }
        for row in rows {
            let c = &row["input"];
            let output = &row["output"];
            if row["phase"] == "constructors" {
                assert_eq!(
                    output["constructors"],
                    json!([{"index":0,"name":"TerrainBlockSize","current":16,"kind":3,"flags":0},{"index":6,"name":"VGridCellSize","current":4,"kind":3,"flags":0}])
                );
                let mut state = State {
                    terrain: 16,
                    grid: 4,
                    sources: Vec::new(),
                    assignments: Vec::new(),
                };
                state.assign_terrain_block_size(c[0].as_u64().unwrap() as u32 as i32);
                state.assign_vgrid_cell_size(c[1].as_u64().unwrap() as u32 as i32);
                assert_eq!(
                    json!([state.terrain as u32, state.grid as u32]),
                    output["current"]
                );
            } else {
                let mut probe = Probe {
                    input: c,
                    stop: row["stop"].as_str(),
                    events: Vec::new(),
                };
                let result = metrics(&mut probe);
                let stopped = result.as_ref().err().cloned();
                let result=result.ok().map(|m|json!({"grid_shift":m.grid_shift,"terrain_quad_size":m.terrain_quad_size as u32}));
                assert_eq!(
                    json!({"events":probe.events,"stopped":stopped,"result":result}),
                    *output,
                    "{row}"
                );
            }
        }
    }
}
