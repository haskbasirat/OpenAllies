use aa_navigation::{OwnedPath, region::*};
use aa_numeric::Precision;
use serde::Deserialize;
use serde_json::{Value, json};
#[derive(Clone, Deserialize)]
struct Case {
    name: String,
    precision: Precision,
    start: [u32; 2],
    goal: [u32; 2],
    centers: Vec<[u32; 2]>,
    lookup_results: Vec<i32>,
    route: Vec<i32>,
    plan_result: i32,
    blocked: Vec<i32>,
    line_clear: Vec<bool>,
    adjust_success: bool,
    adjusted: Option<[u32; 2]>,
    path: OwnedPath,
}
struct Host {
    c: Case,
    events: Vec<Value>,
    lookup: usize,
    block: usize,
    line: usize,
    position: [u32; 2],
}
impl World for Host {
    fn region_at(&mut self, point: [u32; 2]) -> Result<i32> {
        let result = self.c.lookup_results[self.lookup.min(self.c.lookup_results.len() - 1)];
        self.lookup += 1;
        self.events
            .push(json!({"name":"RegionAt","point":point,"result":result}));
        Ok(result)
    }
    fn plan_regions(&mut self, start: i32, goal: i32) -> Result<RegionRoute> {
        self.events.push(
            json!({"name":"PlanRegions","start":start,"goal":goal,"result":self.c.plan_result}),
        );
        Ok(RegionRoute {
            status: self.c.plan_result,
            nodes: self.c.route.clone(),
        })
    }
    fn region_center(&self, id: i32) -> Result<[u32; 2]> {
        self.c
            .centers
            .get(id as usize)
            .copied()
            .ok_or_else(|| Error::World(format!("missing region {id}")))
    }
    fn begin_probe(&mut self) -> Result<()> {
        self.events.push(json!({"name":"BeginProbe"}));
        Ok(())
    }
    fn place_context(&mut self, point: [u32; 2]) -> Result<()> {
        self.position = point;
        Ok(())
    }
    fn blocked_here(&mut self) -> Result<i32> {
        let result = self.c.blocked[self.block.min(self.c.blocked.len() - 1)];
        self.block += 1;
        self.events
            .push(json!({"name":"BlockedHere","position":self.position,"result":result}));
        Ok(result)
    }
    fn line_clear(&mut self, start: [u32; 2], goal: [u32; 2]) -> Result<bool> {
        let result = self.c.line_clear[self.line.min(self.c.line_clear.len() - 1)];
        self.line += 1;
        self.events
            .push(json!({"name":"LineClear","start":start,"goal":goal,"result":result}));
        Ok(result)
    }
    fn adjust_endpoint(&mut self, start: [u32; 2], goal: &mut [u32; 2]) -> Result<bool> {
        let before = *goal;
        if let Some(p) = self.c.adjusted {
            *goal = p;
        }
        self.events.push(json!({"name":"Adjust","start":start,"fallback":start,"before":before,"after":goal,"result":self.c.adjust_success}));
        Ok(self.c.adjust_success)
    }
}
#[test]
fn original_region_list_pruning_waypoints_and_adjustment_match() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-region/prepass-fixtures.json"
    ))
    .unwrap();
    for row in doc["fixtures"].as_array().unwrap() {
        let c: Case = serde_json::from_value(row["input"].clone()).unwrap();
        let mut path = c.path.clone();
        let mut host = Host {
            c: c.clone(),
            events: vec![],
            lookup: 0,
            block: 0,
            line: 0,
            position: [0, 0],
        };
        let result = prepass(&mut path, c.start, c.goal, c.precision, &mut host)
            .unwrap_or_else(|e| panic!("{}: {e}", c.name));
        assert_eq!(
            serde_json::to_value(path).unwrap(),
            row["output"]["path"],
            "{} path",
            c.name
        );
        assert_eq!(json!(result), row["output"]["result"], "{} result", c.name);
        assert_eq!(
            json!(host.events),
            row["output"]["events"],
            "{} events",
            c.name
        );
        assert_eq!(
            json!(host.position),
            row["output"]["context_position"],
            "{} descriptor",
            c.name
        );
    }
}
