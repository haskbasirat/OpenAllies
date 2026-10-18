use super::*;
#[test]
fn missing_global_retains_constructor_prefix_and_unknown_cells() {
    struct Missing;
    impl Globals for Missing {
        fn word(&mut self, _: u32) -> Result<u32> {
            Err("missing actual process global".into())
        }
    }
    let failure = match State::<usize>::from_constructor(&mut Missing) {
        Err(f) => f,
        Ok(_) => panic!("missing global unexpectedly completed constructor"),
    };
    for offset in [4, 8, 12, 16] {
        assert_eq!(failure.state.word(offset).unwrap(), Some(0));
    }
    assert_eq!(failure.state.word(0x14).unwrap(), None);
    assert_eq!(failure.state.word(0x28).unwrap(), None);
    assert!(failure.state.grid_present().is_err());
}

#[test]
fn original_v14_constructor_first_grid_and_all_read_store_prefixes() {
    use serde_json::{Value, json};
    use std::collections::BTreeMap;
    struct Probe<'a> {
        state: State<usize>,
        input: &'a Value,
        stop: Option<&'a str>,
        counts: BTreeMap<String, usize>,
        events: Vec<String>,
    }
    impl Probe<'_> {
        fn event(&mut self, label: String) -> Result<()> {
            let count = self.counts.entry(label.clone()).or_default();
            let event = format!("{label}:{count}");
            *count += 1;
            if self.stop == Some(event.as_str()) {
                return Err(event);
            }
            self.events.push(event);
            Ok(())
        }
        fn project(&self, stopped: Option<String>) -> Value {
            let words: Vec<_> = self
                .state
                .words
                .iter()
                .map(|w| match w {
                    Word::Unwritten => Value::Null,
                    Word::Bits(bits) => json!(bits),
                    Word::GridAllocation => json!("grid"),
                })
                .collect();
            let grid = matches!(self.state.words[0], Word::GridAllocation);
            json!({"events":self.events,"words":words,"grid":grid,
                "cells":self.state.cells.as_ref().map(Vec::len),
                "zero":self.state.cells.as_ref().map(|cells| cells.iter().all(Cell::is_empty)),
                "stopped":stopped})
        }
    }
    impl ConstructorWorld for Probe<'_> {
        fn global(&mut self, address: u32) -> Result<u32> {
            let label = format!("load{address:x}");
            self.event(label.clone())?;
            let count = self.counts[&label] - 1;
            let base = self.input["globals"][format!("0x{address:x}")]
                .as_u64()
                .unwrap() as u32;
            let step = self.input["step"].as_u64().unwrap() as u32;
            Ok(base.wrapping_add(step.wrapping_mul(count as u32)))
        }
        fn store(&mut self, offset: u32, value: u32) -> Result<()> {
            self.event(format!("store{offset:x}"))?;
            self.state.store(offset, value)
        }
    }
    impl InitWorld for Probe<'_> {
        fn store(&mut self, offset: u32, value: u32) -> Result<()> {
            <Self as ConstructorWorld>::store(self, offset, value)
        }
        fn allocate_zero_cells(&mut self, dimensions: [i32; 2]) -> Result<()> {
            self.event("allocate".into())?;
            //The native allocation is not published in VGrid+0 until after its
            //zero fill. A stop at publication leaves the prior grid field null.
            self.event("publish".into())?;
            Initializer {
                state: &mut self.state,
            }
            .allocate_zero_cells(dimensions)
        }
    }
    struct Seed<'a>(&'a Value);
    impl Globals for Seed<'_> {
        fn word(&mut self, address: u32) -> Result<u32> {
            Ok(self.0["globals"][format!("0x{address:x}")]
                .as_u64()
                .unwrap() as u32)
        }
    }
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-view-grid/native-view-grid.json"
    ))
    .unwrap();
    let rows = fixture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 795);
    for (index, row) in rows.iter().enumerate() {
        let input = &row["input"];
        let state = if input["phase"] == "init" {
            assert_eq!(input["step"], 0);
            State::from_constructor(&mut Seed(input)).unwrap()
        } else {
            State::allocation()
        };
        let mut probe = Probe {
            state,
            input,
            stop: row["stop"].as_str(),
            counts: BTreeMap::new(),
            events: Vec::new(),
        };
        let result = if input["phase"] == "init" {
            let pc = match row["pc"].as_str().unwrap() {
                "Single24" => Precision::Single24,
                "Double53" => Precision::Double53,
                "Extended64" => Precision::Extended64,
                _ => unreachable!(),
            };
            initialize(
                &mut probe,
                std::array::from_fn(|i| input["extents"][i].as_u64().unwrap() as u32),
                input["shift"].as_u64().unwrap() as u32,
                pc,
            )
        } else {
            construct(&mut probe)
        };
        assert_eq!(
            probe.project(result.err()),
            row["output"],
            "native row {index}"
        );
    }
}
