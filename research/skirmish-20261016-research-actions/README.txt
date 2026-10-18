Research action contributor583FC0 -- 2026-10-16

Scope/API
---------
aa_actors::research_actions::contribute(&mut impl World, Research storage owner)
aa_runtime::saved_research_actions::contribute(&mut SavedActions, ActorId)
(the runtime module/API are crate-private).
No source loader, parser, dependency, Cargo configuration, original executable,
original data/save/profile or Development08 release is changed. New modules
only, plus one export in each shared lib.rs; prior exports are backed up in
backups/. The restore_checkpoint agent owns dispatcher forwarding.

Native behavior
---------------
583FC0..584007:
  583FC3 reads Research+4 owner;
  583FC8 calls4D14C0(owner,group13);
  583FDC reads selected ActorStateData+28;
  583FE4 tests exact value1 (BUSY), which skips the source list read;
  otherwise583FE8 rereads Research+4 and583FEE reads owner ActorData+558;
  583FFA rereads Research+4;
  584000 calls517FA0(owner,35,enabled).
Action35 is in the high word.517FA0 first writes the unchanged low word+F0,
then the updated high word+F4. Other63bits are preserved. No callback occurs
between those stores. No source list node is traversed and no ResearchState
in-progress list is inspected. There is no research-cost/eligibility test here.
Type values other than1 are not treated as BUSY, including arbitrary u32words.

The Rust contributor reuses native state::group and ability_flags::set_action.
It preserves short-circuit and field-read order; an unavailable selected state
or required source list fails before any action store. No selected group13
descriptor is explicit unsupported rather than guessing a state/default.
World methods are authoritative field reads/writes, not gameplay callbacks.

Native proof
------------
native_research_actions.py independently loads original AA.exe and accepted
v14 AA.exe. Executes complete583FC0,4D14C0,4D0820,517FA0 and shift helper,
with no gameplay seams. Original477FC0 group registration masks are reused
from the pinned phase7-actors/native-actor-states.json. Source type/head values
are explicit fixture inputs, not a new claim about generic definition loading.
Every executed instruction is checked against the other image by the economy
harness. Native image/function-window/generator hashes are retained in JSON.

552runs =92inputs x3control-word modes x2independently loaded images.
56cases cross all7 group13 states, types0/1/2/FFFFFFFF and null/non-null list
heads.32cases vary full64-bit masks, multiple selected bits, other groups,
arbitrary upper-word action bits and non-normalized source head values.
Two missing-source-pointer cases prove BUSY skips ActorData and non-BUSY
stops before action writes. Two missing-group cases observe guarded null
access. These controlled invalid-pointer rows do not claim the original
would fault identically for every arbitrary corrupt world.
Recorded events are actual selected-type/head reads and both ordered stores.
The Rust test replays all552rows exactly. No floating-point arithmetic is
present, so there is no precision parameter on this leaf API; all three native
control words are still exercised by the proof.

Saved runtime binding
---------------------
Reads actual ResearchState.header.owner, authoritative component state bits,
and actor F0/F4 action words. ActorState type comes from the existing proven
recruit_data::actor_state_type provider. ActorData+558 presence comes from
RuntimeDefinition.ground.research_items, already bound by Catalog using
ActorCatalog::native_research_property_ids for5849E0 saved restoration.
It does not perform a second canonical ActorData lookup or inspect the
component's in-progress ResearchState.items. The stored source definition is
used for both the active context owner and registry-owned actors. None means
unavailable and returns an error when read; Some(empty) means actual absence.
This reuses prior definition-loading behavior and does not expand its parity
scope or validate a new ResearchItem producer implementation.

The ignored actual-saved-record regression loads tutorial02 under explicit
native mode1/PC53 with diagnostic InspectPartial. It creates SavedActions
from real records and a real nonempty source list. Explicit probe state/mask
inputs cover all7 group13 states using actual source type definitions. The
in-progress list is preserved. Injected missing-source and empty-list inputs
test BUSY short-circuit versus a real required-boundary error before stores.
No original map/save is written. This proves the leaf adapter over real owned
data, not complete map restoration, publication or research execution.

Validation/reproduction
-----------------------
  $env:PYTHONPATH='E:\Games\Axis and Alies\reconstruction\tools\python-packages'
  python research/skirmish-20261016-research-actions/native_research_actions.py
  $env:CARGO_TARGET_DIR='target/s-research-actions'
  cargo test -p aa-actors --release --locked --offline -j4 --test native_research_actions -- --nocapture
  $env:AA_GAME_ROOT='E:\Games\Axis and Alies'
  cargo test -p aa-runtime --release --locked --offline -j4 --lib saved_research_actions::tests -- --ignored --nocapture
tests.log and adapter-tests.log contain actual outcomes. findings.json records
final hashes and passed counts after those checks. No broad workspace/rules/
metagame test is claimed. No new arbitrary runtime cap is introduced.

Remaining scope
---------------
Research command execution, costs, queues and completion effects; complete
saved-world restoration/publication; live simulation/HUD; playable skirmish
and multiplayer are separate work. Source registry/load-error behavior is
unchanged. Malformed address behavior beyond the bounded guarded fixtures,
allocator exhaustion and concurrent mutation during this leaf are unproven.
