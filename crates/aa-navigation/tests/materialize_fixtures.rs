use aa_navigation::{OwnedPath, materialize::*};
use aa_numeric::Precision;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
struct Grid {
    width: i32,
    height: i32,
    coordinate_shift: u32,
    cell_size_bits: u32,
    predecessors: Vec<u8>,
}
impl PredecessorGrid for Grid {
    fn dimensions(&self) -> [i32; 2] {
        [self.width, self.height]
    }
    fn coordinate_shift(&self) -> u32 {
        self.coordinate_shift
    }
    fn cell_size_bits(&self) -> u32 {
        self.cell_size_bits
    }
    fn predecessor(&self, cell: [i32; 2]) -> Option<u8> {
        self.predecessors
            .get((cell[1] * self.width + cell[0]) as usize)
            .copied()
    }
}
#[derive(Deserialize)]
struct Case {
    name: String,
    precision: Precision,
    end_cell: [i32; 2],
    state: State,
    grid: Grid,
    path: OwnedPath,
    trace_cells: Vec<Vec<[i32; 2]>>,
    trace_blocked: Vec<bool>,
}
struct Host {
    traces: Vec<Vec<[i32; 2]>>,
    blocked: Vec<bool>,
    trace: usize,
    index: usize,
    events: Vec<Value>,
}
impl World for Host {
    fn begin_trace(
        &mut self,
        from: [u32; 2],
        to: [u32; 2],
        state: &mut State,
        _: &mut OwnedPath,
    ) -> Result<()> {
        self.events.push(json!({"name":"BeginTrace","collision_word28":state.collision_word28,"from_bits":from,"to_bits":to}));
        self.trace += 1;
        self.index = 0;
        Ok(())
    }
    fn next_trace_cell(
        &mut self,
        state: &mut State,
        _: &mut OwnedPath,
    ) -> Result<Option<[i32; 2]>> {
        let cells = &self.traces[(self.trace - 1).min(self.traces.len() - 1)];
        let cell = cells.get(self.index).copied();
        self.index += 1;
        self.events
            .push(json!({"name":"NextCell","collision_word28":state.collision_word28,"cell":cell}));
        Ok(cell)
    }
    fn cell_blocked(
        &mut self,
        cell: [i32; 2],
        state: &mut State,
        _: &mut OwnedPath,
    ) -> Result<bool> {
        let result = self.blocked[(self.trace - 1).min(self.blocked.len() - 1)];
        self.events.push(json!({"name":"CellBlocked","collision_word28":state.collision_word28,"cell":cell,"result":result}));
        Ok(result)
    }
}
#[test]
fn original_predecessor_materialization_and_smoothing_match_native() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-navigation/materialize-fixtures.json"
    ))
    .unwrap();
    for row in doc["fixtures"].as_array().unwrap() {
        let mut c: Case = serde_json::from_value(row["input"].clone()).unwrap();
        let mut host = Host {
            traces: c.trace_cells,
            blocked: c.trace_blocked,
            trace: 0,
            index: 0,
            events: vec![],
        };
        materialize(
            &mut c.state,
            &mut c.path,
            c.end_cell,
            &c.grid,
            c.precision,
            &mut host,
        )
        .unwrap_or_else(|e| panic!("{}: {e}", c.name));
        assert_eq!(
            serde_json::to_value(c.path).unwrap(),
            row["output"]["path"],
            "{} path",
            c.name
        );
        assert_eq!(
            serde_json::to_value(c.state).unwrap(),
            row["output"]["state"],
            "{} state",
            c.name
        );
        assert_eq!(
            serde_json::to_value(host.events).unwrap(),
            row["output"]["events"],
            "{} callbacks",
            c.name
        );
    }
}

#[test]
fn invalid_predecessor_chain_is_rejected_before_path_mutation() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-navigation/materialize-fixtures.json"
    ))
    .unwrap();
    let mut c: Case = serde_json::from_value(doc["fixtures"][0]["input"].clone()).unwrap();
    c.grid.predecessors[(c.end_cell[1] * c.grid.width + c.end_cell[0]) as usize] = 0;
    let before = c.path.clone();
    let mut host = Host {
        traces: c.trace_cells,
        blocked: c.trace_blocked,
        trace: 0,
        index: 0,
        events: vec![],
    };
    assert!(matches!(
        materialize(
            &mut c.state,
            &mut c.path,
            c.end_cell,
            &c.grid,
            c.precision,
            &mut host
        ),
        Err(Error::InvalidPredecessor(_))
    ));
    assert_eq!(c.path, before);
    assert!(host.events.is_empty());
}
