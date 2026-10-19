//! The reference inverse index and the load-validation rectangle memo give
//! exactly the answers of the scans/computations they replace.
use super::*;
use serde_json::Value;

/// The original reference_for_key: scan every reference in slot order.
fn reference_for_key_scan(index: &SpatialIndex, key: ActorKey) -> Result<u32> {
    let mut matches = index.references.iter().filter_map(|(&slot, &(generation, candidate))| {
        (candidate == key).then_some((u32::from(generation) << 16) | u32::from(slot))
    });
    let reference = matches
        .next()
        .ok_or_else(|| Error::World("spatial key has no current reference".into()))?;
    if matches.next().is_some() {
        return Err(Error::World("spatial key has duplicate current references".into()));
    }
    Ok(reference)
}
/// The original register_load_reference, applied to a plain map.
fn register_scan(map: &mut BTreeMap<u16, (u16, ActorKey)>, key: ActorKey, reference: u32) -> std::result::Result<(), String> {
    if key == 0 || reference as u16 == 0 {
        return Err("null".into());
    }
    if let Some(&(generation, old_key)) = map.get(&(reference as u16)) {
        if generation == (reference >> 16) as u16 && old_key == key {
            return Ok(());
        }
        return Err("conflict".into());
    }
    if map.values().any(|&(_, old_key)| old_key == key) {
        return Err("owned".into());
    }
    map.insert(reference as u16, ((reference >> 16) as u16, key));
    Ok(())
}
fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../research/phase7-spatial/native-index.json")).unwrap()
}
fn index_for(case: &Value) -> SpatialIndex {
    SpatialIndex::new(
        serde_json::from_value(case["grid"].clone()).unwrap(),
        serde_json::from_value(case["flags"].clone()).unwrap(),
        serde_json::from_value(case["precision"].clone()).unwrap(),
    )
    .unwrap()
}

#[test]
fn reference_inverse_index_equals_full_reference_scan() {
    let fixture = fixture();
    let mut index = index_for(&fixture["cases"][0]);
    let mut model: BTreeMap<u16, (u16, ActorKey)> = BTreeMap::new();
    let mut state = 0x9e37_79b9u32;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        state
    };
    for step in 0..20_000u32 {
        let key = next() % 12;
        let reference = ((next() % 3) << 16) | (next() % 10);
        if next() % 3 == 0 {
            index.retire_reference(reference);
            if model.get(&(reference as u16)).is_some_and(|&(g, _)| g == (reference >> 16) as u16) {
                model.remove(&(reference as u16));
            }
        } else {
            let fast = index.register_load_reference(key, reference).map_err(|e| e.to_string());
            let slow = register_scan(&mut model, key, reference);
            assert_eq!(fast.is_ok(), slow.is_ok(), "step {step}: {fast:?} {slow:?}");
        }
        assert_eq!(index.references, model, "step {step}");
        for key in 0..12 {
            assert_eq!(
                index.reference_for_key(key).map_err(|e| e.to_string()),
                reference_for_key_scan(&index, key).map_err(|e| e.to_string()),
                "step {step} key {key}"
            );
        }
    }
}

#[test]
fn load_validation_memo_equals_direct_rectangle_checks() {
    let fixture = fixture();
    let mut checked = 0;
    for case in fixture["cases"].as_array().unwrap() {
        let mut index = index_for(case);
        let members: Vec<Member> = case["members"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| serde_json::from_value(m.clone()).unwrap())
            .collect();
        for member in &members {
            let _ = index.register_load_reference(member.key, member.reference);
        }
        let direct = |index: &SpatialIndex, member: &Member| -> std::result::Result<(), String> {
            if index.resolve_reference(member.reference) != Some(member.key) {
                return Err("generation".into());
            }
            tile_rectangle(&index.grid, &member.placement(false), index.precision)
                .map(|_| ())
                .map_err(|e| e.to_string())
        };
        for member in &members {
            let mut variants = vec![member.clone()];
            let mut moved = member.clone();
            moved.actor.position[0] = moved.actor.position[0].wrapping_add(0x0010_0000);
            variants.push(moved);
            let mut invalid = member.clone();
            invalid.actor.position = [0x7f80_0000, 0x7f80_0000];
            variants.push(invalid);
            let mut wide = member.clone();
            wide.soft.radius_bits = 0x7f7f_ffff;
            variants.push(wide);
            // Each variant twice, after every other variant: a memo hit must
            // never hide a different outcome.
            for round in 0..2 {
                for variant in &variants {
                    let expected = direct(&index, variant);
                    let actual = index.validate_load_fields(variant).map_err(|e| e.to_string());
                    assert_eq!(actual.is_ok(), expected.is_ok(), "round {round}");
                    if let (Err(a), Err(e)) = (&actual, &expected) {
                        if e != "generation" {
                            assert_eq!(a, e);
                        }
                    }
                    checked += 1;
                }
            }
        }
    }
    assert!(checked > 100, "{checked}");
}

/// A by-reference lease installs exactly the members a cloned-vector lease
/// installs (same map, same errors), whatever the previous lease parked.
#[test]
fn by_reference_lease_equals_cloned_vector_lease() {
    let fixture = fixture();
    let mut compared = 0;
    for case in fixture["cases"].as_array().unwrap().iter().take(12) {
        let base: Vec<Member> = case["members"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| serde_json::from_value(m.clone()).unwrap())
            .collect();
        let mut fast = index_for(case);
        let mut slow = index_for(case);
        for member in &base {
            let _ = fast.register_load_reference(member.key, member.reference);
            let _ = slow.register_load_reference(member.key, member.reference);
        }
        let mut state = 0x1357_9bdfu32 ^ base.len() as u32;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state
        };
        for round in 0..40 {
            // A random subset, sometimes modified, sometimes out of order or
            // duplicated, sometimes with an invalid placement.
            let mut fields: Vec<Member> = base.iter().filter(|_| next() % 4 != 0).cloned().collect();
            for member in &mut fields {
                match next() % 6 {
                    0 => member.actor.flags ^= 1 << (next() % 27),
                    1 => member.actor.position[1] = member.actor.position[1].wrapping_add(0x0004_0000),
                    2 if next() % 5 == 0 => member.actor.position = [0x7f80_0000, 0x7f80_0000],
                    _ => {}
                }
            }
            if round % 7 == 3 && fields.len() > 1 {
                fields.swap(0, 1);
            }
            if round % 11 == 5 && !fields.is_empty() {
                let copy = fields[0].clone();
                fields.push(copy);
            }
            let refs: Vec<&Member> = fields.iter().collect();
            let a = fast.lease_load_fields_by_ref(&refs).map(|lease| lease.index().members.clone());
            let b = slow.lease_load_fields(fields.clone()).map(|lease| lease.index().members.clone());
            match (a, b) {
                (Ok(a), Ok(b)) => assert_eq!(a, b, "round {round}"),
                (Err(a), Err(b)) => assert_eq!(a.to_string(), b.to_string(), "round {round}"),
                (a, b) => panic!("round {round}: {:?} vs {:?}", a.map(|m| m.len()), b.map(|m| m.len())),
            }
            assert!(fast.members.is_empty() && slow.members.is_empty());
            compared += 1;
        }
    }
    assert!(compared >= 400, "{compared}");
}

/// The reference part of the validation memo: with references retired,
/// re-registered under other generations/keys and members whose reference
/// is stale, validate_load_fields keeps giving exactly the direct answer.
#[test]
fn load_validation_memo_follows_reference_changes() {
    let fixture = fixture();
    let mut checked = 0;
    for case in fixture["cases"].as_array().unwrap().iter().take(8) {
        let mut index = index_for(case);
        let members: Vec<Member> = case["members"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| serde_json::from_value(m.clone()).unwrap())
            .collect();
        if members.is_empty() {
            continue;
        }
        for member in &members {
            let _ = index.register_load_reference(member.key, member.reference);
        }
        let direct = |index: &SpatialIndex, member: &Member| -> std::result::Result<(), String> {
            if index.resolve_reference(member.reference) != Some(member.key) {
                return Err("load spatial world error: load fields do not match the registered generation".into());
            }
            tile_rectangle(&index.grid, &member.placement(false), index.precision)
                .map(|_| ())
                .map_err(|e| e.to_string())
        };
        let mut state = 0x2468_ace1u32 ^ members.len() as u32;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state
        };
        for step in 0..3000u32 {
            let member = &members[next() as usize % members.len()];
            match next() % 6 {
                0 => index.retire_reference(member.reference),
                1 => {
                    let _ = index.register_load_reference(member.key, member.reference);
                }
                2 => {
                    // Another generation of the same slot, or another key.
                    let other = &members[next() as usize % members.len()];
                    let reference = member.reference.wrapping_add(0x1_0000 * (next() % 2));
                    index.retire_reference(member.reference);
                    let _ = index.register_load_reference(other.key, reference);
                }
                _ => {}
            }
            let mut variant = member.clone();
            if next() % 4 == 0 {
                variant.reference = variant.reference.wrapping_add(0x1_0000);
            }
            if next() % 5 == 0 {
                variant.actor.position[1] = variant.actor.position[1].wrapping_add(0x0008_0000);
            }
            let expected = direct(&index, &variant);
            let actual = index.validate_load_fields(&variant).map_err(|e| e.to_string());
            assert_eq!(actual.is_ok(), expected.is_ok(), "step {step}: {actual:?} {expected:?}");
            if let (Err(a), Err(e)) = (&actual, &expected) {
                if e.contains("generation") {
                    assert!(a.contains("load fields do not match the registered generation"), "{a}");
                } else {
                    assert_eq!(a, e);
                }
            }
            checked += 1;
        }
    }
    assert!(checked > 1000, "{checked}");
}
