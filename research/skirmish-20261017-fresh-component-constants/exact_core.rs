mod actor_world {
    pub type RuntimeComponent=aa_actors::owned::Component<String,
        aa_mind::construct::Installed,aa_combat::constructors::OwnedPerformer>;
}
#[path="../../crates/aa-runtime/src/map_component_constant_leaf.rs"]
mod constant_leaf;
