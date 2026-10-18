use super::*;
#[test]
fn actual_native_major_slot_holes_and_first_positive_alias_cutoff() {
    let f: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-mission-start/native-merge-kingdoms.json"
    ))
    .unwrap();
    assert_eq!(f["cases"].as_array().unwrap().len(), 42);
    for row in f["cases"].as_array().unwrap() {
        //Explicit portable leaf fixture; this cannot mint a loaded World.
        let input = row["input"].as_array().unwrap();
        let names: Vec<Vec<u16>> = input
            .iter()
            .map(|v| {
                v.as_array()
                    .map(|a| a.iter().map(|v| v.as_u64().unwrap() as u16).collect())
                    .unwrap_or_default()
            })
            .collect();
        let major: Vec<_> = input
            .iter()
            .enumerate()
            .map(|(i, v)| {
                if v == "hole" {
                    None
                } else {
                    Some(ActorId::from_raw(i as u32 + 1))
                }
            })
            .collect();
        let mut seen = Vec::new();
        let r = slots(&major, |id| {
            let i = (id.raw() - 1) as usize;
            seen.push(i);
            Ok(names[i].as_slice())
        });
        let expected: Vec<_> = row["output"]["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| {
                v.as_array()
                    .filter(|v| v[0] == "name")
                    .map(|v| v[1].as_u64().unwrap() as usize)
            })
            .collect();
        assert_eq!(seen, expected, "{row}");
        assert_eq!(r.is_ok(), row["output"]["outcome"] == "ok", "{row}");
        if let Err(e) = r {
            assert!(e.contains("4F48E0"));
        }
    }
}
#[test]
fn actual_sync_owner_marker_is_lazy_and_remains_before_later_required_alias() {
    use aa_orders::net::sync::{EntryKind, Synchronizer};
    let mut sync = Synchronizer::new(8);
    let mut forbidden = || Err("unbound thread gate".into());
    marker(&mut SyncLog {
        owner: &mut sync,
        gate: &mut forbidden,
    })
    .unwrap();
    //The native host enables the log through77D900 Reset, which also
    //initializes its window. Toggling only enabled leaves ctor INT_MIN state.
    sync.reset(0,-1);
    sync.suspend = 1;
    marker(&mut SyncLog {
        owner: &mut sync,
        gate: &mut forbidden,
    })
    .unwrap();
    sync.suspend = 0;
    assert!(
        marker(&mut SyncLog {
            owner: &mut sync,
            gate: &mut forbidden
        })
        .is_err()
    );
    assert_eq!(sync.end(), 1);
    marker(&mut SyncLog {
        owner: &mut sync,
        gate: &mut || Ok(true),
    })
    .unwrap();
    let before = sync.entry(1).unwrap().clone();
    assert_eq!(
        (before.label.as_str(), before.kind, before.value),
        ("MergeKingdoms", EntryKind::Empty, 0)
    );
    assert!(slots(&[Some(ActorId::from_raw(1))], |_| Ok(&[0])).is_err());
    assert_eq!(sync.entry(1), Some(&before));
}
