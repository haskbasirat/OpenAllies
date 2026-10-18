//! Actual Session76C7C0's positive-global-World scan over the retained BC
//! roster. Player+28 is read raw: its constructor-null and the early765E10
//! store are real even when later VWorld construction has not completed.
//! A matching player requires the genuine765E10 replacement service. This
//! immutable resource-worker loan deliberately stops before that child.
use super::*;
use aa_runtime::{load_tail_world::KingdomId, world_revision::ConstructedWorldRevision};

fn first_required_clear(players: &[OwnedPlayer], removed: KingdomId) -> Option<(usize, u32)> {
    for (index, player) in players.iter().enumerate() {
        // Native765E00 returns the raw slot when B36534 is nonnull. In the
        // managed-load caller the same sealed World is already published.
        // Do not require binding_complete/view: those certify later work.
        if player.core.field_28.map(|id| id.raw()).unwrap_or(0) == removed.raw() {
            return Some((index, player.core.id));
        }
    }
    None
}

pub(super) fn clear_in_created_world(
    players: &[OwnedPlayer],
    removed: KingdomId,
    world: &ConstructedWorldRevision,
) -> std::result::Result<(), String> {
    match first_required_clear(players, removed) {
        None => Ok(()),
        Some((index, id)) => Err(format!(
            "required765E10(NULL) actual player/VWorld replacement at Session BC index {index}, player {id}, World revision {}",
            world.value()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn actor(raw: u32) -> KingdomId { KingdomId::from_raw(raw) }
    fn row_players(links: &[Value]) -> Vec<OwnedPlayer> {
        links.iter().enumerate().map(|(index, link)| {
            let mut core = PlayerFields::from_constructor(index as u32 + 1, PeerLink::Node);
            // Explicit oracle case inputs, not production construction paths.
            core.field_28 = match link.as_str() {
                Some("removed") => Some(actor(0x1000055)),
                Some("other") => Some(actor(0x1000066)),
                None if link.is_null() => None,
                _ => panic!("non-positive-World oracle input"),
            };
            OwnedPlayer { allocation_lifetime: Arc::new(()), core, vector_44: Vec::new(), view: None }
        }).collect()
    }

    #[test]
    fn native_positive_world_scan_reads_actual_raw_slots_and_stops_at_first_required_clear() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261017-configure-kingdom-removal/native-session-scan.json"
        )).unwrap();
        let mut no_match = 0;
        let mut required_prefix = 0;
        let mut outside_scope = 0;
        for row in fixture["rows"].as_array().unwrap() {
            if row["input"]["world"] == false {
                // The native no-World poison case proves the getter's other
                // branch; this deliberately positive-World adapter cannot
                // claim that case or inspect its poisoned raw slot.
                outside_scope += 1;
                continue;
            }
            let links = row["input"]["links"].as_array().unwrap();
            let players = row_players(links);
            let before: Vec<_> = players.iter().map(|p| p.core.field_28).collect();
            let got = first_required_clear(&players, actor(0x1000055));
            let native_clears = row["output"]["clears"].as_array().unwrap();
            match got {
                None => {
                    assert!(native_clears.is_empty());
                    assert!(row["output"]["error"].is_null());
                    assert_eq!(row["input"]["links"], row["output"]["links"]);
                    no_match += 1;
                }
                Some((index, id)) => {
                    // Only the first real child boundary is admitted. Native
                    // successful/second-cutoff labchildren are not a Rust
                    // implementation of765E10 and do not certify its effects.
                    assert_eq!(native_clears[0].as_u64().unwrap() as usize, index);
                    assert_eq!(id as usize, index + 1);
                    required_prefix += 1;
                }
            }
            assert_eq!(players.iter().map(|p| p.core.field_28).collect::<Vec<_>>(), before);
            assert!(players.iter().all(|p| !p.core.binding_complete && p.view.is_none()));
        }
        assert_eq!((no_match, required_prefix, outside_scope), (24, 24, 6));
    }

    #[test]
    fn nonempty_constructor_roster_does_not_claim_completed_vworld_binding() {
        let players = row_players(&[Value::Null, Value::Null]);
        assert!(first_required_clear(&players, actor(1677)).is_none());
        assert!(players.iter().all(|p| p.core.field_28.is_none()
            && !p.core.binding_complete && p.view.is_none()));
        // A raw store survives a later replacement failure and must still be
        // observed: a missing view is never permission to invent null here.
        let mut partial = players;
        partial[1].core.field_28 = Some(actor(1677));
        assert_eq!(first_required_clear(&partial, actor(1677)), Some((1, 2)));
        assert_eq!(partial[1].core.field_28, Some(actor(1677)));
    }
}
