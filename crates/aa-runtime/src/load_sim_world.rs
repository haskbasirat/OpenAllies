//!76AA90's assignment after the actual World constructor returns, BEFORE
//! its RNG seed and4F27E0 Init. This is only the fresh successful-constructor
//! store boundary, not a certificate that World Init or map loading finished.
use crate::world_revision::ConstructedWorldRevision;

/// The worker lends the same retained LocalBootstrap Sim, alongside its
/// disjoint Session loan. A diagnostic load with no sink cannot establish
/// the real Sim/GWorld ownership relation for later battle activation.
pub trait WorldConstructorSink {
    fn store_world_76aa90(&mut self, world: &ConstructedWorldRevision) -> Result<(), String>;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn same_actual_revision_is_stored_once_and_never_inferred_from_later_world_presence() {
        let mut revisions = crate::world_revision::Counter::fresh_application();
        let world = revisions.construct();
        let other = revisions.construct();
        let mut sim = crate::live_sim::Sim::from_native_constructor_fields();
        assert!(sim.world_is_null());
        assert!(sim.require_created_world(&world).is_err());
        sim.store_world_76aa90(&world).unwrap();
        assert!(!sim.world_is_null());
        assert_eq!(sim.bound_world_revision(), Some(world.value()));
        sim.require_created_world(&world).unwrap();
        assert!(sim.require_created_world(&other).is_err());
        let mut second_application = crate::world_revision::Counter::fresh_application();
        let equal_revision = second_application.construct();
        assert_eq!(world.value(), equal_revision.value());
        assert!(sim.require_created_world(&equal_revision).is_err());
        assert!(sim.store_world_76aa90(&other).is_err());
        assert_eq!(sim.bound_world_revision(), Some(world.value()));
    }
    #[test]
    fn native_store_precedes_seed_and_init_and_constructor_failures_retain_prefix() {
        let f: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/development10/live-sim/native-created-world.json"
        ))
        .unwrap();
        assert_eq!(f["rows"].as_array().unwrap().len(), 30);
        for row in f["rows"].as_array().unwrap() {
            let n = &row["native"];
            match row["input"]["stop"].as_str().unwrap() {
                "post_store" => {
                    assert_eq!(n["world"], "new");
                    assert_eq!(n["field34"], 0);
                    assert_eq!(
                        n["events"],
                        serde_json::json!(["diagnostic", "allocate", "world_constructor"])
                    );
                }
                "diagnostic" => {
                    assert_eq!(n["world"], 0);
                    assert_eq!(n["field34"], 0);
                }
                "allocate" | "world_constructor" => {
                    assert_eq!(n["world"], 0);
                    assert_eq!(n["field34"], 0);
                }
                "null_allocation" => {
                    assert_eq!(n["world"], 0);
                    assert_eq!(n["field34"], 0);
                }
                _ => panic!("unknown native cutoff"),
            }
        }
    }
}
