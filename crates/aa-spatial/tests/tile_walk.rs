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
/// The binary64 PC53 step against the `Fast` step (which the native fixtures
/// above prove): same candidates, states and errors for realistic, tied,
/// tiny, huge and arbitrary binary32 walks. `AA_WALK_CASES` sets the count.
#[test]
fn pc53_shortcut_matches_the_exact_step() {
    let cases: usize = std::env::var("AA_WALK_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(40_000);
    let mut seed = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = move || {
        seed = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = seed;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    };
    let (mut compared, mut steps) = (0usize, 0usize);
    for case in 0..cases {
        let pick = |r: u64, list: &[f32]| list[r as usize % list.len()];
        let kind = case % 6;
        let cell = match kind {
            0 | 1 | 2 => pick(next(), &[1.0, 2.0, 4.0, 8.0, 16.0]),
            3 => pick(next(), &[0.5, 1.5, 3.0, 0.1, 7.25]),
            4 => f32::from_bits((next() as u32 & 0x007F_FFFF) | 0x3F80_0000),
            _ => f32::from_bits((next() as u32 % 0x7F00_0000).max(1)),
        };
        let shift = cell.log2().clamp(0.0, 31.0) as u32;
        let mut coordinate = |axis_end: bool| -> f32 {
            let r = next();
            match kind {
                // Map coordinates with a fraction.
                0 => (r % 4_096_000) as f32 / 1000.0,
                // Exactly on a cell boundary or a simple fraction of one.
                1 => (r % 64) as f32 * cell / pick(r >> 32, &[1.0, 2.0, 4.0]),
                // Short legs with deltas near the 1e-4 epsilon.
                2 => {
                    let base = (r % 1000) as f32;
                    if axis_end {
                        base + pick(r >> 32, &[0.0, 1e-4, -1e-4, 1.2e-4, 9e-5, -0.0])
                    } else {
                        base
                    }
                }
                3 | 4 => (r % 200_000) as f32 / 100.0 - 500.0,
                // Any finite binary32, tiny or huge.
                _ => {
                    let v = f32::from_bits(r as u32);
                    if v.is_finite() { v } else { 0.5 }
                }
            }
        };
        let start = [coordinate(false), coordinate(false)];
        let mut end = [coordinate(true), coordinate(true)];
        if kind == 1 && next() % 3 == 0 {
            // Exact diagonal through cell corners.
            end[1] = start[1] + (end[0] - start[0]);
        }
        let grid = TileGrid {
            width: 64,
            height: 64,
            coordinate_shift: shift,
            cell_size_bits: cell.to_bits(),
        };
        let bits = |p: [f32; 2]| [p[0].to_bits(), p[1].to_bits()];
        let make = || TileWalk::new(grid, bits(start), bits(end), Precision::Double53);
        let (Ok(mut fast), Ok(exact)) = (make(), make()) else {
            continue;
        };
        let mut exact = exact.without_shortcut();
        compared += 1;
        for _ in 0..fast.remaining_candidates().min(300) {
            let a = format!("{:?}", fast.next_candidate());
            let b = format!("{:?}", exact.next_candidate());
            assert_eq!(a, b, "case {case}: {start:?} -> {end:?} cell {cell}");
            assert_eq!(fast.state(), exact.state(), "case {case}");
            steps += 1;
            if a.starts_with("Err") {
                break;
            }
        }
    }
    assert!(compared > cases / 2, "{compared} of {cases} walks built");
    eprintln!("{compared} walks, {steps} steps");
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
