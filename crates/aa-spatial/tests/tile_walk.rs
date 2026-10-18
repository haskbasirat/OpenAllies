use aa_numeric::Precision;
use aa_spatial::{Error, tile_walk::*};
use serde::Deserialize;
use serde_json::Value;
#[derive(Deserialize)]
struct Input {
    grid: TileGrid,
    start_bits: [u32; 2],
    end_bits: [u32; 2],
    precision: Precision,
    name: String,
}
fn rows() -> Vec<Value> {
    serde_json::from_str::<Value>(include_str!(
        "../../../research/phase6-spatial/native-tile-walk.json"
    ))
    .unwrap()["fixtures"]
        .as_array()
        .unwrap()
        .clone()
}
#[test]
fn all_original_tile_candidate_states_and_filtered_order_match() {
    let fixtures = rows();
    assert_eq!(fixtures.len(), 360);
    for row in fixtures {
        let input: Input = serde_json::from_value(row["input"].clone()).unwrap();
        let make = || {
            TileWalk::new(
                input.grid,
                input.start_bits,
                input.end_bits,
                input.precision,
            )
            .unwrap()
        };
        let mut walk = make();
        assert_eq!(
            serde_json::to_value(walk.state()).unwrap(),
            row["initial"],
            "{} initial",
            input.name
        );
        for expected in row["raw"].as_array().unwrap() {
            assert_eq!(
                serde_json::to_value(walk.next_candidate().unwrap()).unwrap(),
                expected["cell"],
                "{} raw cell",
                input.name
            );
            assert_eq!(
                serde_json::to_value(walk.state()).unwrap(),
                expected["state"],
                "{} raw state",
                input.name
            );
        }
        let mut walk = make();
        let mut budget = walk.remaining_candidates();
        for expected in row["filtered"].as_array().unwrap() {
            assert_eq!(
                serde_json::to_value(walk.next_cell_with_budget(&mut budget).unwrap()).unwrap(),
                expected["cell"],
                "{} filtered cell",
                input.name
            );
            assert_eq!(
                serde_json::to_value(walk.state()).unwrap(),
                expected["state"],
                "{} filtered state",
                input.name
            );
        }
        assert_eq!(budget, 0);
    }
}
#[test]
fn budget_stops_without_inventing_a_clear_cell_and_can_resume_explicitly() {
    let grid = TileGrid {
        width: 3,
        height: 3,
        coordinate_shift: 1,
        cell_size_bits: 2.0f32.to_bits(),
    };
    let mut walk = TileWalk::new(
        grid,
        [(-4.0f32).to_bits(), 1.0f32.to_bits()],
        [5.0f32.to_bits(), 1.0f32.to_bits()],
        Precision::Double53,
    )
    .unwrap();
    let before = walk.state().clone();
    let mut budget = 0;
    assert!(matches!(
        walk.next_cell_with_budget(&mut budget),
        Err(Error::WalkBudget)
    ));
    assert_eq!(*walk.state(), before);
    budget = 2;
    assert!(matches!(
        walk.next_cell_with_budget(&mut budget),
        Err(Error::WalkBudget)
    ));
    assert_eq!(walk.state().current, [-1, 0]);
    budget = walk.remaining_candidates();
    assert_eq!(
        walk.next_cell_with_budget(&mut budget).unwrap(),
        Some([0, 0])
    );
}
