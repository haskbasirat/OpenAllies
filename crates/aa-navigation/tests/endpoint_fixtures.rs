use aa_navigation::endpoint::*;
use aa_numeric::Precision;
use serde::Deserialize;
use serde_json::{Value, json};
#[derive(Deserialize)]
struct Displace {
    result: bool,
    position: Option<[u32; 2]>,
}
#[derive(Deserialize)]
struct Case {
    name: String,
    precision: Precision,
    start: [u32; 2],
    destination: [u32; 2],
    fallback: [u32; 2],
    bounds: Bounds,
    accept_results: Vec<bool>,
    displacements: Vec<Displace>,
}
struct Host {
    accept: Vec<bool>,
    displace: Vec<Displace>,
    a: usize,
    d: usize,
    events: Vec<Value>,
}
impl World for Host {
    fn acceptable(&mut self, start: [u32; 2], candidate: [u32; 2]) -> Result<bool> {
        let result = self.accept[self.a.min(self.accept.len() - 1)];
        self.a += 1;
        self.events
            .push(json!({"name":"Acceptable","start":start,"candidate":candidate,"result":result}));
        Ok(result)
    }
    fn displace_blocked(&mut self, fallback: [u32; 2], candidate: &mut [u32; 2]) -> Result<bool> {
        let response = &self.displace[self.d.min(self.displace.len() - 1)];
        self.d += 1;
        let before = *candidate;
        if let Some(p) = response.position {
            *candidate = p;
        }
        self.events.push(json!({"name":"Displace","fallback":fallback,"before":before,"after":candidate,"result":response.result}));
        Ok(response.result)
    }
}
#[test]
fn original_adjustment_preserves_candidate_and_collision_order() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-navigation/endpoint-fixtures.json"
    ))
    .unwrap();
    for row in doc["fixtures"].as_array().unwrap() {
        let mut c: Case = serde_json::from_value(row["input"].clone()).unwrap();
        let mut host = Host {
            accept: c.accept_results,
            displace: c.displacements,
            a: 0,
            d: 0,
            events: vec![],
        };
        let accepted = adjust(
            c.start,
            &mut c.destination,
            c.fallback,
            c.bounds,
            c.precision,
            &mut host,
        )
        .unwrap_or_else(|e| panic!("{}: {e}", c.name));
        assert_eq!(
            json!({"destination":c.destination,"accepted":accepted,"events":host.events,"abi_preserved":true}),
            row["output"],
            "{}",
            c.name
        );
    }
}

#[test]
fn invalid_steps_and_nonprogressing_native_loops_are_errors() {
    let mut host = Host {
        accept: vec![false],
        displace: vec![Displace {
            result: false,
            position: None,
        }],
        a: 0,
        d: 0,
        events: vec![],
    };
    let mut destination = [20f32.to_bits(), 25f32.to_bits()];
    let mut bounds = Bounds {
        cell_size_bits: 0.5f32.to_bits(),
        extent_bits: [64f32.to_bits(); 2],
    };
    assert!(matches!(
        adjust(
            [0, 0],
            &mut destination,
            [0, 0],
            bounds,
            Precision::Double53,
            &mut host
        ),
        Err(Error::InvalidStep)
    ));
    assert!(host.events.is_empty());
    bounds.cell_size_bits = 4f32.to_bits();
    destination = [1e20f32.to_bits(); 2];
    assert!(matches!(
        adjust(
            [0, 0],
            &mut destination,
            [0, 0],
            bounds,
            Precision::Double53,
            &mut host
        ),
        Err(Error::NoProgress)
    ));
    assert!(!host.events.is_empty());
}
