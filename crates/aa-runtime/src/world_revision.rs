//! Process B3CC94, zero in the PE loader's uninitialized section tail.
//! World constructor4F8072..4F809C increments it with native u32 wrapping.
//! Kept by the actual managed application; each World retains its own stamp.
#[derive(Debug)]
pub(crate) struct Counter {
    value: u32,
}
impl Counter {
    pub(crate) fn fresh_application() -> Self {
        Self { value: 0 }
    }
    pub(crate) fn construct(&mut self) -> ConstructedWorldRevision {
        self.value = self.value.wrapping_add(1);
        ConstructedWorldRevision {
            value: self.value,
            identity: std::sync::Arc::new(()),
        }
    }
}
/// Native revision value created by the same application's World constructor.
/// No Clone/Default/public scalar constructor can manufacture this receipt.
#[derive(Debug)]
pub struct ConstructedWorldRevision {
    value: u32,
    identity: std::sync::Arc<()>,
}
/// Nonowning identity metadata for a read of the SAME retained World. It
/// cannot keep a destroyed World allocation alive or be made from a scalar.
/// The caller must derive it from the current validated load/frame owner.
#[derive(Debug, Clone)]
pub struct Witness {
    value: u32,
    identity: std::sync::Weak<()>,
}
impl Witness {
    pub(crate) fn require_live(&self)->Result<(),String>{
        self.identity.upgrade().map(|_|()).ok_or_else(||"World revision witness belongs to a destroyed allocation".into())
    }
    pub(crate) fn value(&self) -> u32 {
        self.value
    }
    pub(crate) fn owner_identity(&self) -> &std::sync::Weak<()> {
        &self.identity
    }
    ///Readonly correlation with the actual retained World allocation. This
    ///does not certify a completed frame or promote render data to a receipt.
    pub fn is_for(&self, world: &ConstructedWorldRevision) -> bool {
        self.value == world.value
            && self
                .identity
                .upgrade()
                .is_some_and(|identity| std::sync::Arc::ptr_eq(&identity, &world.identity))
    }
}
impl ConstructedWorldRevision {
    pub fn value(&self) -> u32 {
        self.value
    }
    pub fn witness(&self) -> Witness {
        Witness {
            value: self.value,
            identity: std::sync::Arc::downgrade(&self.identity),
        }
    }
    /// Stable pointer identity metadata. Equal numeric native revisions in
    /// two applications still name different actual World allocations.
    pub(crate) fn owner_identity(&self) -> &std::sync::Arc<()> {
        &self.identity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_constructor_interval_and_zero_tail() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/development10/world-revision/native-world-revision.json"
        ))
        .unwrap();
        assert_eq!(fixture["native_executions"], 48);
        let section = &fixture["zero_tail"];
        assert!(section["raw_size"].as_u64().unwrap() <= section["word_offset"].as_u64().unwrap());
        let mut fresh = Counter::fresh_application();
        assert_eq!(fresh.value, 0);
        assert_eq!(fresh.construct().value(), 1);
        assert_eq!(fresh.construct().value(), 2);
        for row in fixture["rows"].as_array().unwrap() {
            let mut owner = Counter {
                value: row["before"].as_u64().unwrap() as u32,
            };
            assert_eq!(u64::from(owner.construct().value()), row["after"]);
        }
    }
    #[test]
    fn native_global_world_publication_precedes_sim_alias_return_store() {
        let fixture:serde_json::Value=serde_json::from_str(include_str!(
            "../../../research/skirmish-20261017-session-kingdom-cleanup/world-presence/native-global-world.json"
        )).unwrap();
        assert_eq!(fixture["rows"].as_array().unwrap().len(), 12);
        for row in fixture["rows"].as_array().unwrap() {
            for key in [
                "global_is_new",
                "sim_is_null",
                "singleton_backlink",
                "constructor_this",
            ] {
                assert_eq!(row["partial"][key], true);
            }
            assert_eq!(row["after_constructor_return_store"]["global_is_new"], true);
            assert_eq!(row["after_constructor_return_store"]["sim_is_new"], true);
        }
    }
    #[test]
    fn nonowning_world_witness_rejects_foreign_destroyed_and_replaced_allocations() {
        use crate::load_sim_world::WorldConstructorSink;
        let mut first_application = Counter::fresh_application();
        let world = first_application.construct();
        let witness = world.witness();
        assert!(witness.is_for(&world));
        let mut sim = crate::live_sim::Sim::from_native_constructor_fields();
        assert!(sim.require_created_world_witness(&witness).is_err());
        sim.store_world_76aa90(&world).unwrap();
        sim.require_created_world(&world).unwrap();
        sim.require_created_world_witness(&witness).unwrap();
        let mut second_application = Counter::fresh_application();
        let foreign_same_value = second_application.construct();
        assert_eq!(foreign_same_value.value(), world.value());
        assert!(!witness.is_for(&foreign_same_value));
        assert!(
            sim.require_created_world_witness(&foreign_same_value.witness())
                .is_err()
        );
        let replacement = first_application.construct();
        assert!(!witness.is_for(&replacement));
        assert!(sim.store_world_76aa90(&replacement).is_err());
        assert!(
            sim.require_created_world_witness(&replacement.witness())
                .is_err()
        );
        let mut other_sim = crate::live_sim::Sim::from_native_constructor_fields();
        other_sim.store_world_76aa90(&replacement).unwrap();
        assert!(other_sim.require_created_world_witness(&witness).is_err());
        drop(world);
        // Neither the Sim's native pointer alias nor a cloned read witness
        // is an owner of the World allocation/lifetime.
        assert!(witness.owner_identity().upgrade().is_none());
        assert!(!witness.is_for(&replacement));
        assert!(sim.require_created_world_witness(&witness).is_err());
        let replacement_witness = replacement.witness();
        other_sim
            .require_created_world_witness(&replacement_witness)
            .unwrap();
        drop(replacement);
        assert_eq!(other_sim.bound_world_revision(), Some(2));
        assert!(
            other_sim
                .require_created_world_witness(&replacement_witness)
                .is_err()
        );
    }
}
