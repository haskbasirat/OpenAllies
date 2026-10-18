# Ground actor construction and activation

The production code in this checkpoint provides native ground component constructor state, the complete GActor InitCreate caller sequence, component selection and required world callbacks, actual ground leaf queue scheduling, registered actor state bit transitions, and Structure/Settlement attachment predicates. It does not independently supply every world service needed to create a playable unit. Root's runtime must bind one authoritative actor/component arena, real ownership, effects, abilities, spatial placement and callbacks.

## Production entry points

- `aa_actors::ground_states`: Body, Character, Mover, Organization, Element, Economy and Detection constructors. Optional fields record native uninitialized storage; padding is never filled with invented state. Mover `movement_state` / `apply_movement_state` create and commit a temporary procedural movement snapshot, preserving unrelated fields and partial failures.
- `aa_actors::init_create::initialize`: original 4D6F10 ordering, actual 4D21A0 selection predicates, numbering, Character/default stance dispatch, linked-child anchor placement, generation-bearing linking, ordered component InitCreate and construction registration. `World` has required synchronous services; an unavailable service must return an error. `stored_owner` is the direct actor+E8 read, while `owner` is virtual+108. `assembly_completed` preserves the owner/UI notification boundary after newly created components. The original UI global is absent in the caller fixture, so the UI-agent branch itself is not verified here.
- `aa_actors::leaf_init`: `body` calls the independently proven Body setter and performs actual queue1 insertion after native UnitSupplyEnable, Organization absence, damage-rate, record and duplicate gates. `ground_support` implements verified Character/Mover/Experience trivial leaves, Properties/Organization/Element/Detection scheduling, and Economy supply-required scheduling. Performer and Mind leaves must use their separate concrete modules. `ground_requirement` covers 13 native constant requirement getter cases. Queue owners must uniquely identify a real component Updatable, rather than reuse an actor ID.
- `aa_actors::state`: all 42 compiled actor descriptors, exclusive masks, `group` and `set`. Native construction flag31 suppresses enter/leave callbacks but still rebuilds abilities for exclusive changes. Native disabling of an active exclusive state reselects it; the implementation preserves that behavior. Actor state enter/leave/property and ability effects remain required runtime callbacks. Loaded state clears group4 mask `0x180007ffdf` before component restoration; source `type` at descriptor+28 is distinct from compiled group+2C.
- `aa_actors::attachment`: original Structure+10 parent, parent Settlement+14 attached actor equality and post-virtual parent reread. Invalid null-dereference domains become explicit errors. This does not implement full Structure/Settlement constructors or behavior.
- `aa_mind::construct`: concrete Base/Element/Regiment/Aircraft Mind state storage and embedded state metadata. State bindings require real allocated arena IDs. Element consumes exactly one shared world RNG result and copies the exact extended-precision jitter into each actual Performer ability. This file and its two tests are included in the separate phase7-mind evidence.

## Shared runtime storage

`owned::Component<D,M,P>` is the single concrete payload enum for both newly created and loaded actors. It owns Body/Mover/Character/Properties/Organization/Element/Economy/Experience/Detection state; its Mind and Performer parameters are real runtime arena records/handles, never fabricated implementations. `owned::construct` actually allocates the ground states from explicit completed native definition inputs. Scenegraph's saved type aliases it with `Infallible` for unsupported Mind/Performer restoration.

`Components::map_values` and `IdentityRegistry::map_values` move existing state without allocating new game IDs or running constructors. Registry generations, occupied words, cursor and high-water slot are preserved exactly. Rust registry wrapper storage can change allocation, but the owned component boxes are moved intact and tested to retain their original address and modified health. `creation::construct_actor_with_components` creates the typed actor shell directly, and generic `initialize_object` registers it then performs original scale RNG. Existing creation APIs remain compatible.

`aa_mind::construct::Constructed::install` moves the core and embedded state records into the real Mind arena. The component retains an `Installed` value with the arena IDs and additional initialized fields. Duplicate arena IDs are rejected before mutation; constructor RNG and ability writes are not repeated.

The full 4CF480 factory has no Experience rank assignment after its constructor: 4CF84A calls 571690, then the common 4CFA6C tail stores EAX in the actor component slot. Initial rank assignment is a later required phase; `ExperienceState::new` correctly preserves its null rank.

## Evidence

The original AA.exe and accepted modified v14 executable are read-only. Each producer pins executed original instruction bytes against both files. Constructors use allocation and resolved-reference storage seams; these are named in the corresponding corpus, never counted as implemented gameplay. Ground constructor fixtures cover 336 executions. Full InitCreate has 192 executions, factory dispatch 39, requirement getters 13, ground leaf initialization 224, actor state transitions and lookup 982, and attachment 24. Mover +48 read instrumentation covers 186 source cases and 540 executions including poison and NaN substitutions on 177 unread paths. These numbers are fixture coverage, not a completion percentage.

The full InitCreate corpus caught and fixed the linked-child Y rotation sign and the owner-callback ordering: store rotated XY offsets first, invoke owner, then read parent position and current facing. The temporary child creation record stores parent GTID at +48. Anchor Z is ignored, actual parent Z is retained.

Mind native fixture producers are maintained by visual_audit in `research/phase7-mind`; 72 constructor executions and 41 registered metadata initializers underlie this constructor port. Constant template fields are compared byte for byte including untouched poison bytes, and native pointer values are reconstructed only in tests.

## Scope and limits

- Numeric inputs are finite at the arithmetic boundaries; invalid required fields are errors. Stock selected x87 precision is preserved. Constructor invalid turn types leave angular speed absent; a required read then errors.
- The original Mover constructor does not write +48. Target range is now `Option<f32>` and is only required on the native branch that reads it. Ordinary movement activation does not fabricate a zero range. Existing finite fixtures retain their previous serialized values.
- Saved construction is separate from InitCreate and must not consume new-actor RNG or schedule new-actor work. Scenegraph owns backward-compatible generic LoadShell component storage and saved component codecs.
- Body InitCreate uses stable source definition/SVar and creation-record inputs; its synchronous Host may mutate Body/Environment. Live structural replacement from a Body callback is outside that helper contract and must be orchestrated at the runtime boundary.
- Ground component constructor fields do not claim operational tick behavior for Economy, Detection, Organization or Element. Full gameplay still needs the respective real systems, ownership, orders, combat and save restoration; no fake callbacks are provided.
- Only original ground constructors are production-complete here. The full 31-slot dispatcher is observed, but unsupported non-ground constructor services remain explicit.
- Original network/multiplayer, campaign, save writing and full battles are not established by this checkpoint.

## Reproduction

Run the eight producers in this folder, then `cargo test -p aa-actors -p aa-movement` and `cargo clippy -p aa-actors -p aa-movement --all-targets -- -D warnings`. Run the separate Mind constructor test with `cargo test -p aa-mind --test native_constructors`. The evidence ledger records hashes and actual test logs. Independent review is separate; this producer document does not self-approve the implementation.
