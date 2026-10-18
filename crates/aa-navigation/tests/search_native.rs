use aa_navigation::search::{
    QueueEntry, Result, SearchCell, SearchGrid, SearchInput, SearchWorld, search,
    unmodified_step_cost,
};
use aa_numeric::Precision;
use serde_json::Value;

#[derive(Default)]
struct FlagCollision {
    trace: Vec<[u32; 4]>,
}
impl SearchWorld for FlagCollision {
    fn collision_blocked(&mut self, grid: &SearchGrid, cell: [i32; 2]) -> Result<bool> {
        self.trace.push([
            cell[0] as u32,
            cell[1] as u32,
            grid.cell(cell)?.visit_epoch,
            grid.cell(cell)?.predecessor_direction.into(),
        ]);
        Ok(grid.cell(cell)?.flags20 & 2 != 0)
    }
    fn accumulated_cost(
        &mut self,
        grid: &SearchGrid,
        current: QueueEntry,
        next: [i32; 2],
        precision: Precision,
    ) -> Result<u32> {
        unmodified_step_cost(grid, current, next, precision)
    }
}
fn pair(v: &Value) -> [i32; 2] {
    std::array::from_fn(|i| v[i].as_i64().unwrap() as i32)
}

#[test]
fn search_matches_original_x86_control_flow_heap_cost_and_predecessors() {
    let report: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-pathsearch/native-search-fixtures.json"
    ))
    .unwrap();
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 75);
    for case in cases {
        let input = &case["input"];
        let [width, height] = pair(&input["size"]);
        let size = input["cell_size"].as_f64().unwrap_or(2.) as f32;
        let mut grid = SearchGrid {
            width,
            height,
            coordinate_shift: input["shift"].as_u64().unwrap_or(1) as u32,
            cell_size_bits: size.to_bits(),
            epoch: input["initial_epoch"].as_u64().unwrap_or(0) as u32,
            cells: vec![SearchCell::default(); (width * height) as usize],
        };
        for y in 0..height {
            for x in 0..width {
                let blocked = input["blocked"]
                    .as_array()
                    .is_some_and(|list| list.iter().any(|c| pair(c) == [x, y]));
                grid.cells[(y * width + x) as usize].flags20 =
                    if x == 0 || y == 0 || x == width - 1 || y == height - 1 {
                        1
                    } else if blocked {
                        2
                    } else {
                        0
                    };
            }
        }
        let goal_cell = pair(&input["goal"]);
        let request = SearchInput {
            start_cell: pair(&input["start"]),
            goal_cell,
            goal_world_bits: std::array::from_fn(|i| {
                input["goal_world"][i]
                    .as_f64()
                    .map(|v| v as f32)
                    .unwrap_or(goal_cell[i] as f32 * size + size * 0.5)
                    .to_bits()
            }),
            precision: serde_json::from_value(case["precision"].clone()).unwrap(),
            queue_limit: input["queue_limit"].as_u64().map(|v| v as usize),
        };
        let mut world = FlagCollision::default();
        let actual = search(&mut grid, request, &mut world).unwrap();
        let expected = &case["output"];
        assert_eq!(
            actual.status,
            expected["status"].as_u64().unwrap() as u32,
            "{} {}",
            input["name"],
            case["precision"]
        );
        let expected_end = expected["finished"].as_array().unwrap().first().map(pair);
        assert_eq!(
            actual.end_cell, expected_end,
            "{} {}",
            input["name"], case["precision"]
        );
        let popped: Vec<[u32; 5]> = expected["popped"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| std::array::from_fn(|i| v[i].as_u64().unwrap() as u32))
            .collect();
        assert_eq!(
            actual
                .expanded
                .iter()
                .map(|v| v.words())
                .collect::<Vec<_>>(),
            popped,
            "heap order/cost {} {}",
            input["name"],
            case["precision"]
        );
        assert_eq!(
            actual.queue_remaining,
            expected["queue_remaining"].as_u64().unwrap() as usize
        );
        assert_eq!(grid.epoch, expected["visit_epoch"].as_u64().unwrap() as u32);
        let trace: Vec<[u32; 4]> = expected["collision_trace"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| std::array::from_fn(|i| v[i].as_u64().unwrap() as u32))
            .collect();
        assert_eq!(
            world.trace, trace,
            "collision order {} {}",
            input["name"], case["precision"]
        );
        let visited: Vec<Value> = (0..height)
            .flat_map(|y| (0..width).map(move |x| [x, y]))
            .filter_map(|cell| {
                let tile = grid.cell(cell).unwrap();
                (tile.visit_epoch == grid.epoch).then(
                    || serde_json::json!({"cell":cell,"direction":tile.predecessor_direction}),
                )
            })
            .collect();
        assert_eq!(
            visited,
            *expected["visited"].as_array().unwrap(),
            "predecessors {} {}",
            input["name"],
            case["precision"]
        );
    }
}
