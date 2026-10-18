Fresh stored Mover/Element Init binding (author source/test freeze)

Production APIs are stored_mover_init_world::{init_existing,after_init} and
stored_element_init_world::{init_existing,after_init}. They consume the SAME
Staged ActorStore, moved source Bank, current report.world_version (the mutable
World+2C owner), scheduler Owners, intrusive Queues and NativeRng. No saved
LoadShell is constructed or cloned. Fresh4D7230 dispatch calls these directly.

Mover576700 now has one storage-independent field algorithm shared with the
original held saved API. The actual falling byte is read first; byte0 returns
without world/version/head/queue/state reads. Reached old-version queue0 uses
its existing constructor label and consumes the real queue RNG draw. State24
writes the actual component state prefix before a reached required4D4A90.
Only a successful newly-enabled state24 path performs the native OR bit12;
an already-active request or a failed enter does not write that suffix.
The published flag path has a dedicated same-index bit12 method, no general
mask/pose setter. A transient state Fields value is an in-call projection;
there is no second persistent state or flag owner.

Element5704D0 uses the SAME existing45D5B0->5014E0 rate calculation. Its source
ActorData+2CC belongs to the completed moved Bank source PC, while multiplication
uses the explicit live PC and actual current Properties modifier. It stores the
real Element+20 before the shared version/head/queue1 suffix. A later queue
error therefore preserves that store. AfterInit is the actual4B2830 RET after
portable source/type/generation/owner guards, not an ignored missing service.

Availability proof: the frozen earlier constructor receipt supplies actual
embedded Updatable tables A3CE5C(Mover) and A3C6C8(Element), not the outer component
InitExisting tables. New native_embedded_updatable.py pins table+8 in both PEs
and executes the real6A71D0 leaf at all3PC:6 paired rows/12 separate executions,
2 executed instruction pins, no successful child seams. This is NOT a fresh
both-image constructor or whole scheduler proof. The existing real queue
implementation remains the scheduling kernel authority.

Fresh immutable native replays: old Mover288 rows/576 machines (204complete,
48required50AC00,36required4D4A90),107 executed plus2 boundary-entry pins; old
Element192 rows/384 machines,45 pins. These preserve their original scopes;
calling the stored concrete queue adapter does not rewrite old required rows.

New external stored_mover_element_init is explicitly a source component-
constructor lab inside one diagnostic-loaded Staged. It constructs only the
actual USA infantry Mover/Element/Properties slots and registers their actual
owner labels using the same GroundAssembly label operation. Other component
slots, whole assembly, selected mission startup and battle are NOT claimed.
Genuine constructor inputs and explicit WorldVersion21/falling/flag31/error
probes are separately labelled. Author actual test1/1PASS11.28s and immutable replay1/1PASS10.18s; typed flag
suffix regression1/1PASS0.04s. See freeze.json. Independent review may follow.

No original assets/executables/saves/releases or root manifests/lockfiles are
changed by this checkpoint. The rewrite still needs a naturally initialized
world, mission startup and ordered live gameplay; leaf proofs alone do not
provide those end-to-end outcomes.
