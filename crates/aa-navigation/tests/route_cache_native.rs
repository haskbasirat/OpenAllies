//! Replays research/phase8-movement/native-route-cache.json: original
//! 0x7ABC30/0x53F0A0 (create), 0x5D8560 (reset), 0x7AC9A0 tail (destroy),
//! 0x53F320 flag 1 and flag 0 on a persistent native solver, and 0x53E5B0.
use aa_navigation::region_search::{
    Error, NATIVE_QUEUE_LIMIT, QueueEntry, RegionGraph, RegionNode, Request, State, search,
};
use aa_navigation::route_cache::{
    CacheError, EMPTY_PATH_COST_BITS, RouteCache, path_cost, reset_path_planner, route_cost,
    search_cached,
};
use aa_numeric::Precision;
use serde_json::{Value, json};

const DOC: &str = include_str!("../../../research/phase8-movement/native-route-cache.json");

fn rle(data: &[u8]) -> String {
    let mut runs: Vec<(u8, usize)> = Vec::new();
    for &byte in data {
        match runs.last_mut() {
            Some((b, n)) if *b == byte => *n += 1,
            _ => runs.push((byte, 1)),
        }
    }
    runs.iter()
        .map(|(b, n)| format!("{b:02x}*{n}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn cache_json(cache: &Option<RouteCache>) -> Value {
    match cache {
        None => Value::Null,
        Some(c) => json!({
            "count": c.row_count(),
            "row_sizes": c.rows().iter().map(|r| r.len().to_string()).collect::<Vec<_>>().join(" "),
            "rle": rle(&c.bytes()),
        }),
    }
}

fn words(entries: &[QueueEntry]) -> Vec<String> {
    entries
        .iter()
        .map(|e| {
            let w = e.words();
            format!("{:08x} {:08x} {:08x} {:08x}", w[0], w[1], w[2], w[3])
        })
        .collect()
}

fn node_states(graph: &RegionGraph) -> String {
    graph
        .nodes
        .iter()
        .map(|n| format!("{:02x}{:02x}", n.visit_epoch, n.parent_neighbor))
        .collect()
}

fn finite_f32(bits: u32) -> bool {
    (bits >> 23) & 0xff != 0xff
}

#[test]
fn route_cache_lifecycle_cached_search_and_path_cost_match_native() {
    let doc: Value = serde_json::from_str(DOC).unwrap();
    assert_eq!(doc["schema"], 1);
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), doc["case_count"].as_u64().unwrap() as usize);
    let (mut searches, mut hits, mut faults, mut costs, mut snan_rows) = (0, 0, 0, 0, 0);
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let precision: Precision = serde_json::from_value(case["precision"].clone()).unwrap();
        let nodes: Vec<RegionNode> = serde_json::from_value(case["nodes"].clone()).unwrap();
        let mut graph = RegionGraph {
            nodes,
            epoch: case["epoch"].as_u64().unwrap() as u8,
            ready: case["ready"].as_bool().unwrap(),
        };
        let mut state = State {
            busy: false,
            output_linked: false,
        };
        let mut cache: Option<RouteCache> = None;
        // 0x53F290 sets the persistent solver's queue limit to 0x2000.
        let mut queue_limit = Some(NATIVE_QUEUE_LIMIT);
        let ops = case["ops"].as_array().unwrap();
        let results = case["results"].as_array().unwrap();
        for (step, (op, expected)) in ops.iter().zip(results).enumerate() {
            let at = format!("{name} step {step} {op}");
            match op["op"].as_str().unwrap() {
                "create" => {
                    cache = Some(RouteCache::build(graph.nodes.len()).unwrap());
                    assert_eq!(cache_json(&cache), expected["cache"], "{at}");
                }
                "reset" => {
                    // 0x5D8560 dereferences the cache without a null check.
                    reset_path_planner(cache.as_mut(), graph.nodes.len()).unwrap();
                    assert_eq!(cache_json(&cache), expected["cache"], "{at}");
                }
                "destroy" => {
                    cache = None;
                    assert_eq!(expected["cache"], Value::Null, "{at}");
                }
                "set" => {
                    if let Some(v) = op.get("busy") {
                        state.busy = v.as_u64().unwrap() != 0;
                    }
                    if let Some(v) = op.get("ready") {
                        graph.ready = v.as_u64().unwrap() != 0;
                    }
                    if let Some(v) = op.get("queue_limit") {
                        queue_limit = Some(v.as_u64().unwrap() as usize);
                    }
                }
                "search" => {
                    searches += 1;
                    let flag = op["flag"].as_u64().unwrap();
                    let request = Request {
                        start: op["start"].as_u64().unwrap() as u32,
                        goal: op["goal"].as_u64().unwrap() as u32,
                        precision,
                        queue_limit,
                    };
                    // The 0x7A1B30 composition on clones must agree with the
                    // separate search + cost below.
                    let composed = (flag == 1).then(|| {
                        let (mut g, mut s, mut c) = (graph.clone(), state, cache.clone());
                        route_cost(&mut g, request, &mut s, c.as_mut())
                    });
                    let mut path = Vec::new();
                    let result = if flag == 1 {
                        search_cached(&mut graph, request, &mut state, cache.as_mut(), &mut path)
                    } else {
                        search(&mut graph, request, &mut state, &mut path)
                    };
                    if let Some(fault) = expected.get("fault") {
                        faults += 1;
                        let e = result.expect_err(&at);
                        match fault["pc"].as_str().unwrap() {
                            // 0x53E610 reads [null+4]: no cache object.
                            "0x53e636" => assert!(
                                matches!(e, Error::RouteCache(CacheError::Missing)),
                                "{at}: {e}"
                            ),
                            // 0x53E920 follows an unknown (0xFF) hop byte past
                            // the adjacency array.
                            "0x53ea0e" => assert!(
                                matches!(
                                    e,
                                    Error::RouteCache(CacheError::InvalidHop { index: 0xff, .. })
                                ),
                                "{at}: {e}"
                            ),
                            // Flag-0 finaliser on a 0xFE predecessor byte.
                            "0x7b90a0" => {
                                assert!(matches!(e, Error::InvalidPredecessor(_)), "{at}: {e}")
                            }
                            other => panic!("{at}: unclassified native fault {other}"),
                        }
                        assert!(composed.is_none_or(|r| r.is_err()), "{at}");
                        break;
                    }
                    let outcome = result.unwrap_or_else(|e| panic!("{at}: {e}"));
                    if flag == 1 && outcome.popped.is_empty() && outcome.status == 1 {
                        hits += 1;
                    }
                    let actual = json!({
                        "status": outcome.status,
                        "route": path,
                        "popped": words(&outcome.popped),
                        "discovered": outcome.discovered.iter().map(|[a, b]| format!("{a}>{b}")).collect::<Vec<_>>().join(" "),
                        "node_states": node_states(&graph),
                        "epoch": graph.epoch,
                        "busy": state.busy,
                        "output_linked": state.output_linked,
                        "queue_remaining": outcome.queue_remaining,
                        "cache": cache_json(&cache),
                    });
                    let mut want = expected.clone();
                    for k in ["cost_x87", "cost_bits", "heap"] {
                        want.as_object_mut().unwrap().remove(k);
                    }
                    assert_eq!(actual, want, "{at}");
                    // 0x53E5B0 on the output list.
                    costs += 1;
                    let bits = expected["cost_bits"].as_u64().unwrap() as u32;
                    let cost = path_cost(&graph, &path, precision);
                    if finite_f32(bits) {
                        assert_eq!(cost.unwrap_or_else(|e| panic!("{at}: {e}")), bits, "{at}");
                        if path.is_empty() {
                            assert_eq!(bits, EMPTY_PATH_COST_BITS, "{at}");
                        }
                    } else {
                        // Native +inf / NaN results: explicit numeric errors.
                        let e = cost.expect_err(&at);
                        let snan = case["snan_input"].as_bool().unwrap();
                        snan_rows += usize::from(snan);
                        let want = if name.starts_with("cost_overflow") {
                            aa_numeric::Error::Binary32Overflow
                        } else {
                            aa_numeric::Error::NonFiniteInput
                        };
                        assert!(
                            matches!(e, Error::Numeric(ref n) if *n == want),
                            "{at}: {e}"
                        );
                    }
                    if let Some(composed) = composed {
                        match composed {
                            Ok(r) => {
                                assert!(finite_f32(bits), "{at}");
                                assert_eq!(r.outcome.status, outcome.status, "{at}");
                                assert_eq!(r.path, path, "{at}");
                                assert_eq!(r.cost_bits, bits, "{at}");
                            }
                            Err(_) => assert!(!finite_f32(bits), "{at}"),
                        }
                    }
                }
                other => panic!("{at}: unknown op {other}"),
            }
        }
    }
    assert_eq!(searches, doc["search_count"].as_u64().unwrap() as usize);
    let native_faults = cases
        .iter()
        .flat_map(|c| c["results"].as_array().unwrap())
        .filter(|r| r.get("fault").is_some())
        .count();
    assert_eq!(faults, native_faults);
    assert_eq!(faults, 4 * 3);
    // The cost_snan scenario's flag-1 and flag-0 rows at three precisions.
    assert!(costs > 900 && snan_rows == 6, "{costs} {snan_rows}");
    let native_hits = cases
        .iter()
        .flat_map(|c| {
            c["ops"]
                .as_array()
                .unwrap()
                .iter()
                .zip(c["results"].as_array().unwrap())
        })
        .filter(|(o, r)| {
            o["flag"] == 1
                && r["status"] == 1
                && r["popped"].as_array().is_some_and(|p| p.is_empty())
        })
        .count();
    assert_eq!(hits, native_hits);
    assert!(hits >= 300, "cached early exits {hits}");
}

fn line(n: usize) -> RegionGraph {
    let nodes = (0..n)
        .map(|i| RegionNode {
            position_bits: [(i as f32).to_bits(), 0],
            category: 1,
            neighbors: [i.checked_sub(1), (i + 1 < n).then_some(i + 1)]
                .into_iter()
                .flatten()
                .map(|v| v as u32)
                .collect(),
            distance_cache: Default::default(),
            visit_epoch: 0,
            parent_neighbor: 255,
        })
        .collect();
    RegionGraph {
        nodes,
        epoch: 0,
        ready: true,
    }
}

fn request(start: u32, goal: u32) -> Request {
    Request {
        start,
        goal,
        precision: Precision::Double53,
        queue_limit: Some(NATIVE_QUEUE_LIMIT),
    }
}

#[test]
fn inconsistent_caches_are_explicit_errors_not_guessed_routes() {
    let mut state = State {
        busy: false,
        output_linked: false,
    };
    // A cached walk that revisits (0,2): the original loops forever.
    let mut graph = line(3);
    let mut cache: RouteCache = serde_json::from_value(json!({"rows": [[0, 0], [0]]})).unwrap();
    let mut path = vec![];
    let e = search_cached(
        &mut graph,
        request(0, 2),
        &mut state,
        Some(&mut cache),
        &mut path,
    )
    .unwrap_err();
    assert!(
        matches!(e, Error::RouteCache(CacheError::Cycle(0, 2))),
        "{e}"
    );
    // The graph grew after the cache was built: the original reads past the rows.
    let mut graph = line(4);
    let mut cache = RouteCache::build(3).unwrap();
    let mut state = State {
        busy: false,
        output_linked: false,
    };
    let e = search_cached(
        &mut graph,
        request(0, 3),
        &mut state,
        Some(&mut cache),
        &mut path,
    )
    .unwrap_err();
    assert!(
        matches!(e, Error::RouteCache(CacheError::PairOutOfRange(0, 3))),
        "{e}"
    );
    // No cache object: start == goal and guard failures still succeed.
    let mut graph = line(3);
    let mut state = State {
        busy: false,
        output_linked: false,
    };
    let r = search_cached(&mut graph, request(1, 1), &mut state, None, &mut path).unwrap();
    assert_eq!((r.status, path.clone()), (1, vec![1]));
    let e = search_cached(&mut graph, request(0, 1), &mut state, None, &mut path).unwrap_err();
    assert!(matches!(e, Error::RouteCache(CacheError::Missing)), "{e}");
    assert!(matches!(
        reset_path_planner(None, 3),
        Err(Error::RouteCache(CacheError::Missing))
    ));
}

#[test]
fn path_cost_edges() {
    let graph = line(3);
    assert_eq!(
        path_cost(&graph, &[], Precision::Single24).unwrap(),
        0xbf80_0000
    );
    // A single region is not looked up (even an unknown id), like 0x53E5B0.
    assert_eq!(path_cost(&graph, &[99], Precision::Single24).unwrap(), 0);
    assert_eq!(
        path_cost(&graph, &[0, 1, 2], Precision::Extended64).unwrap(),
        2.0f32.to_bits()
    );
    assert!(matches!(
        path_cost(&graph, &[0, 9], Precision::Double53),
        Err(Error::InvalidNode(9))
    ));
}
