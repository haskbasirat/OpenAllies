use aa_navigation::{OwnedPath, endpoint, materialize, search, solver};
use aa_numeric::{Binary, Precision};
use aa_spatial::tile_walk::{TileGrid, TileWalk};
use serde_json::{Value, json};

struct StaticWorld {
    grid: search::SearchGrid,
    extent: [u32; 2],
    precision: Precision,
    walk: Option<TileWalk>,
    collision_trace: Vec<[i32; 2]>,
    path_logging: bool,
}
impl StaticWorld {
    fn flags(&self, cell: [i32; 2]) -> u32 {
        self.grid.cell(cell).unwrap().flags20
    }
    fn mask_blocked(&mut self, cell: [i32; 2]) -> bool {
        self.collision_trace.push(cell);
        self.flags(cell) & 2 != 0
    }
}
impl endpoint::World for StaticWorld {
    fn acceptable(&mut self, _: [u32; 2], point: [u32; 2]) -> endpoint::Result<bool> {
        for (axis, value) in point.iter().enumerate() {
            let v = Binary::from_f32_bits(*value)?;
            if v.is_negative_nonzero()
                || v.compare(&Binary::from_f32_bits(self.extent[axis])?) != std::cmp::Ordering::Less
            {
                return Ok(false);
            }
        }
        let shift = self.grid.coordinate_shift & 31;
        let cell = [
            Binary::from_f32_bits(point[0])?.truncate_i32()? >> shift,
            Binary::from_f32_bits(point[1])?.truncate_i32()? >> shift,
        ];
        Ok(self.flags(cell) & 1 == 0 && !self.mask_blocked(cell))
    }
    fn displace_blocked(&mut self, _: [u32; 2], _: &mut [u32; 2]) -> endpoint::Result<bool> {
        Ok(false)
    }
}
impl search::SearchWorld for StaticWorld {
    fn collision_blocked(
        &mut self,
        _: &search::SearchGrid,
        cell: [i32; 2],
    ) -> search::Result<bool> {
        Ok(self.mask_blocked(cell))
    }
    fn accumulated_cost(
        &mut self,
        grid: &search::SearchGrid,
        current: search::QueueEntry,
        next: [i32; 2],
        p: Precision,
    ) -> search::Result<u32> {
        search::unmodified_step_cost(grid, current, next, p)
    }
}
impl materialize::World for StaticWorld {
    fn begin_trace(
        &mut self,
        from: [u32; 2],
        to: [u32; 2],
        _: &mut materialize::State,
        _: &mut OwnedPath,
    ) -> materialize::Result<()> {
        self.walk = Some(
            TileWalk::new(
                TileGrid {
                    width: self.grid.width,
                    height: self.grid.height,
                    coordinate_shift: self.grid.coordinate_shift,
                    cell_size_bits: self.grid.cell_size_bits,
                },
                from,
                to,
                self.precision,
            )
            .map_err(|e| materialize::Error::World(e.to_string()))?,
        );
        Ok(())
    }
    fn next_trace_cell(
        &mut self,
        _: &mut materialize::State,
        _: &mut OwnedPath,
    ) -> materialize::Result<Option<[i32; 2]>> {
        let walk = self.walk.as_mut().unwrap();
        let mut budget = walk.remaining_candidates();
        walk.next_cell_with_budget(&mut budget)
            .map_err(|e| materialize::Error::World(e.to_string()))
    }
    fn cell_blocked(
        &mut self,
        cell: [i32; 2],
        _: &mut materialize::State,
        _: &mut OwnedPath,
    ) -> materialize::Result<bool> {
        Ok(self.flags(cell) & 1 != 0 || self.mask_blocked(cell))
    }
}
impl solver::World for StaticWorld {
    fn disable_path_logging(&mut self) {
        self.path_logging = false;
    }
    fn use_region_pather(&self) -> bool {
        false
    }
    fn collision_actor_flags(&self) -> Option<u32> {
        None
    }
    fn begin_region_probe(&mut self) -> solver::Result<()> {
        panic!("region disabled")
    }
    fn region_line_clear(&mut self, _: [u32; 2], _: [u32; 2]) -> solver::Result<bool> {
        panic!("region disabled")
    }
    fn region_path(&mut self, _: [u32; 2], _: [u32; 2], _: &mut OwnedPath) -> solver::Result<i32> {
        panic!("region disabled")
    }
    fn collision_actor_id(&self) -> Option<u32> {
        None
    }
    fn collision_target_id(&self) -> Option<u32> {
        None
    }
    fn collision_word28(&self) -> u32 {
        0
    }
}
fn pair(v: &Value) -> [i32; 2] {
    [v[0].as_i64().unwrap() as i32, v[1].as_i64().unwrap() as i32]
}
#[test]
fn whole_native_solve_search_and_materialization_match_original_routes() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-navigation/solve-fixtures.json"
    ))
    .unwrap();
    for row in doc["fixtures"].as_array().unwrap() {
        let c = &row["input"];
        let name = c["name"].as_str().unwrap();
        let p: Precision = serde_json::from_value(row["precision"].clone()).unwrap();
        let [width, height] = pair(&c["size"]);
        let cell = c["cell_size"].as_f64().unwrap_or(2.) as f32;
        let shift = c["shift"].as_u64().unwrap_or(1) as u32;
        let start = pair(&c["start"]);
        let goal = pair(&c["goal"]);
        let mut cells = vec![];
        for y in 0..height {
            for x in 0..width {
                let blocked = c["blocked"]
                    .as_array()
                    .is_some_and(|a| a.iter().any(|v| pair(v) == [x, y]));
                cells.push(search::SearchCell {
                    flags20: if x == 0 || y == 0 || x == width - 1 || y == height - 1 {
                        1
                    } else if blocked {
                        2
                    } else {
                        0
                    },
                    visit_epoch: 0,
                    predecessor_direction: 0,
                });
            }
        }
        let mut grid = search::SearchGrid {
            width,
            height,
            coordinate_shift: shift,
            cell_size_bits: cell.to_bits(),
            epoch: c["initial_epoch"].as_u64().unwrap_or(0) as u32,
            cells,
        };
        let coords = |key: &str, indices: [i32; 2]| -> [u32; 2] {
            std::array::from_fn(|axis| {
                if let Some(value) = c[key][axis].as_f64() {
                    (value as f32).to_bits()
                } else {
                    (indices[axis] as f32 * cell + cell * 0.5).to_bits()
                }
            })
        };
        let extent = [
            (width as f32 * cell).to_bits(),
            (height as f32 * cell).to_bits(),
        ];
        let mut world = StaticWorld {
            grid: grid.clone(),
            extent,
            precision: p,
            walk: None,
            collision_trace: vec![],
            path_logging: true,
        };
        let mut path = OwnedPath {
            nodes: vec![],
            kind1_count: 0,
            kind2_count: 0,
            dirty: false,
            word18: 0,
        };
        let mut state = solver::StrategyState {
            route: materialize::State {
                start_cell: [0, 0],
                goal_cell: [0, 0],
                start_bits: [0, 0],
                goal_bits: [0, 0],
                path_linked: false,
                collision_linked: false,
                collision_word28: 0,
            },
            actor_id: 0,
            target_id: 0,
            option_word: 0,
        };
        let result = solver::solve(
            &mut grid,
            solver::Request {
                start_bits: coords("start_world", start),
                goal_bits: coords("goal_world", goal),
                option_word: c["option_word"].as_u64().unwrap_or(0) as u32,
            },
            &mut state,
            &mut path,
            solver::Options {
                bounds: endpoint::Bounds {
                    cell_size_bits: cell.to_bits(),
                    extent_bits: extent,
                },
                precision: p,
                queue_limit: c["queue_limit"]
                    .as_u64()
                    .filter(|v| *v > 0)
                    .map(|v| v as usize),
            },
            &mut world,
        )
        .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            json!(result),
            row["output"]["status"],
            "{name} result {p:?}"
        );
        assert_eq!(
            serde_json::to_value(path).unwrap(),
            row["output"]["path"],
            "{name} path {p:?}"
        );
        assert_eq!(
            json!(grid.epoch),
            row["output"]["visit_epoch"],
            "{name} epoch {p:?}"
        );
        assert!(!world.path_logging);
        let strategy = [
            0,
            0,
            state.route.start_cell[0] as u32,
            state.route.start_cell[1] as u32,
            state.route.goal_cell[0] as u32,
            state.route.goal_cell[1] as u32,
            0,
            state.route.start_bits[0],
            state.route.start_bits[1],
            state.route.goal_bits[0],
            state.route.goal_bits[1],
            0,
            state.option_word,
            state.actor_id,
            state.target_id,
        ];
        assert_eq!(
            json!(strategy),
            row["output"]["strategy_words"],
            "{name} strategy {p:?}"
        );
        let native_trace: Vec<[i32; 2]> = row["output"]["collision_trace"]
            .as_array()
            .unwrap()
            .iter()
            .map(pair)
            .collect();
        assert_eq!(
            world.collision_trace, native_trace,
            "{name} collision order {p:?}"
        );
    }
}
