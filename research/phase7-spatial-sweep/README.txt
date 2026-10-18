Native forward segment sweep553CE0

Production: crates/aa-spatial/src/sweep.rs
API: sweep(index,geometry,&mutPreparedQuery,goal,&mutQueryPolicy)->Result<Hit>
The geometry is the existing source-bound QueryGeometry; the index contains the
real actor shapes, positions, tile lists and generation identities. Hit retains
all four native result words. This operation consumes an already constructed
descriptor. It does not substitute5531F0 for the distinct554350 actor context
required by531170/ResultPlan.

The original operation first computes the displacement with nonpopping x87
stores. If squared distance is below0.0001 it calls553BA0 at the original pose.
Otherwise it sets the descriptor to the endpoint, calls553BA0, then restores
the old position through552250 (including another source-definition snap).
An endpoint hit returns immediately after restoration.

When the endpoint is clear, native46EB30/46E0E0 constructs a rectangle whose
width is twice the original radius (POINT/CIRCLE), or twice the smaller authored
positive box half-size (SQUARE/RECT), and whose length is the displacement.
The operation stores squared distance before square root, preserves the
constructor's extended radius intermediates and SQUARE equality selection,
and retains the supplied facing. Midpoint X sum stores before multiplying by
half; midpoint Y sum does not. No normalized direction, nearest-hit sort,
continuous collision substitute or approximate ray march is introduced.

The corridor descriptor deliberately copies only source actor, ignored actor,
ignored definition, required actor flags and tile mask. Original553090 resets
source definition, owner/hostility and ignore-all/moving/custom filters. This
includes the source-definition large-only distinction in553BA0. The new
rectangle goes through the existing full placed-shape first-hit operation.

Errors preserve the completed stage. In particular, if the endpoint query
fails, the caller's external descriptor is still at its endpoint. The temporary
first-hit actor marks are nevertheless cleared by the query service. Successful
endpoint queries restore through the original setter before any corridor work.
Callers must not claim rollback or blindly retry partially completed operations.

Proof
original_sweep.py executes553CE0 and the unchanged original553BA0 plus all
native shape geometry. It only inherits the prepared query fixture's explicit
allocation, source/custom exclusion and definition-view boundaries. It records
553BA0 input descriptors without replacing that function. There are1024 cases
across PC24/53/64, all source shape kinds0..3, cardinal/rotated facing, short
deltas, world edges, source snap and large-only bits, retired references and
descriptor filters.255 cases reach the actual corridor stage. Rust compares
the complete hit, ordered policy calls, external descriptor, native corridor
shape/pose words and final temporary marks. A separate failure case verifies
the endpoint mutation and mark cleanup boundary.

Source ActorList/QueryPolicy services remain required and are not implemented
by a permissive default. The prepared corpus is not an actual restored map or
a complete playable movement world. Source finite arithmetic and ordinary
immutable actor-shape getter scope are inherited from query/shape_overlap.

Reproduce:
 python research/phase7-spatial-sweep/original_sweep.py
 cargo test -p aa-spatial sweep::tests -- --nocapture
 cargo clippy -p aa-spatial --all-targets --no-deps -- -D warnings
Both focused tests and strict Clippy pass. validation-record.json attributes
the actual test tool outputs and records the saved clippy.log. An initial
premature Cargo invocation embedded the old768-case fixture while regeneration
was still running; its count check failed. The unchanged tests passed after
the1024-case producer completed. No original fixture was represented as a
completed test before that correction.

findings.json is producer evidence. Independent peer review is separate.
