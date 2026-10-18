Organization terrain propagation579EB0, 2026-10-16/17

NEW load_organization_terrain_refresh::refresh(Catalog, actual Record,
actual BattleTerrain, live PC) supplies the real recursive Organization effect
to the previously frozen Actor terrain helper. It does not bind positive
Properties instance/criteria/effect services; those remain precise errors.

Native579EB0 uses4364C0 with attached-only=false. It scans the current actual
member vector/count in slot order, including duplicates and unattached members,
skipping only actual null pointers. Every next slot/count is reread after the
preceding child. A non-null slot requires its actual Actor allocation and
Element;570550 reads that Element's owner and tailcalls4D4130. Missing/stale
members, missing Element and mismatched typed owners are explicit failures,
not ignored slots. No fixed member count or recursion-depth cap was added.

The parent Actor's F8 store already happened before its Organization callback.
While that same parent remains outside its matching Loading entry, the child
can now read the actual parent F8 through a short immutable HeldView reborrow.
The ancestor generation/source/class/storage guards run on the reached path.
Each child remains in its original allocation and returns to its own registry
entry before an error is propagated. Ancestor views carry references to actual
shells and definitions; they do not clone pose/state or retain a second mutable
parent F8 word. Earlier parent/child terrain stores survive later errors.

The original public load_actor_terrain_refresh::refresh entry still supplies
no held-parent view and retains its default required579EB0 and missing-held-
parent errors. Its prior exact source is preserved here as
terrain-helper-before-held-parent.rs. The narrow new refresh_with_held entry
is crate-private and used by this concrete tree wrapper.

Limits remain explicit: cyclic/reentrant held member traversal requires its
own actual owner service and fails if reached. A held base shell does not
provide missing Kingdom/Team derived fields. Positive Properties callbacks,
null-terrain diagnostics and other required terrain-helper effects retain
their original errors. This is not an arbitrary-membership or battle proof.

Native oracle native-organization-terrain.json:69 paired rows/138 separate
original-v14 executions across all x87 precision controls. Actual579EB0,
4364C0,570550,4D4130,4D10A0,570560,4D12C0 and terrain getters execute in the
supported cases; no successful body is intercepted.158 executed address pins
and one stopped missing-Element boundary. Arrays, owner links/ready bytes,
parent F8, terrain objects and child component absence are explicit fixture
inputs. This is not source loading or Property compilation.

The independent-held-source-review.json is a separate bounded source-only
review. It found no blocker in member order, held ancestor reborrows, child
restoration before Result propagation, or the original default boundaries.
It does not claim fresh execution, panic/unwind parity or a completed battle.

Actual-source public Rust test capture passed: real original Kursk herd_horse
GTID15283 and its four real ready Elements15284..15287. All three live PCs
retain the same allocations and inherit the same actual held-parent F8. A
removed-Element fault retains the preceding parent F8 and returns the failed
child allocation to its registry entry. At all PCs the original default entry
still stops at required579EB0 without mutating the children. Gate bits/cached
cells are EXPLICIT controlled inputs; the member/Element owner/parent/ready
relationships come from the actual corrected loader. This is not an untouched
full Actor Init call. Final read-only expected-output replay is recorded in
immutable-public-final-replay.log.

The public test source is included unchanged by public_actual_test_harness.rs,
then linked by rustc --test to the exact warm production aa_runtime library and
matching dependencies. This avoids unrelated changing aa-netcode dev-dependency
compile failures without changing manifests or private production visibility.
The production library itself built successfully through Cargo. Library/binary,
source and evidence hashes are recorded in findings.json.

This test exposed a real earlier Actor-zone migration regression: Organization
Load still read removed raw Actor+14C, so all seven diagnostic Kursk Organization
loads failed before their member arrays populated. Its owner migrated that read
to the checked count of the sole zones_148 vector; no raw zero mirror was added.
The failed original captures and builds are retained as superseded evidence.

The optional standalone source census found144 Terrain registrations,28 with
authored Property views (city/forest/jungle/sand/snow infantry/vehicle modifiers),
all without reloads. It uses an explicit Double53 read-only source projection
and does not itself verify the complete startup receipt, link/criteria/instance
implementation, or gameplay reachability. Positive Properties remain a real
unbound dependency; the successful actual tree uses empty crops terrain lists.
