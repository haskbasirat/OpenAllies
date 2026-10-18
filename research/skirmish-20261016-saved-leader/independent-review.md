# Independent leader/formation binding review

Scope: the borrowed `aa-movement::borrowed_formation` coordinator, the saved
`aa-runtime::saved_organization_leader` adapter, and root's required restore
context/Catalog wiring. This review makes no claim about complete Actor Init,
PostCreate, published worlds, playable battles, or multiplayer.

The original `579A00`, `5797E0`, and `578250` instructions agree with the Rust
order. The leader store precedes the old Element lookup; the old slot runs
before rereading the new leader. A same-identity assignment still computes
twice. Negative slots read nothing. Origin slots bypass unrelated layout
sources. Scattering captures the first positions allocation before RNG, then
rereads the current positions allocation before scaling. Each axis is stored
immediately; later failures retain earlier stores and consumed RNG draws.

The saved adapter mutates the held Organization and its actual offsets. Other
actors pass full generation-checked registry lookup and actual Actor payload
checks. Required Elements check their owner. The active owner uses the actual
held Actor/Thing shells while temporarily outside the registry. A mismatched
Organization owner is a portable structural refusal, not native pointer-fault
equivalence.

Canonical primary slots come from the existing layout source without a second
IDS translation. Duplicate canonical IDs are refused by the native source
producer. Formation handles select the current layout formation, its actual
FormationData organized flag, and source offset bits. The handle is read again
at `578250`'s native boundary. `Catalog::layout_data` checks the original source
owner at actual source use; no eager preflight changes the native store prefix.

The production RNG and message sink come directly from LoadContext. The saved
native oracle executes `85D840` over the captured state rather than replacing
the returned random values. A present Mind reaches an explicit missing `+114`
error. It is not treated as successful initialization. Positions `()` is valid
only within this exclusively borrowed, non-reentrant saved adapter; the core's
separate mutable laboratory world proves allocation recapture behavior.

The detached diagnostic uses signed IDs and six fractional digits. Proof covers
the sink, arguments, order, function, and line; it does not establish arbitrary
CRT/locale/nonfinite byte-string formatting equivalence. Native missing-Element
page-zero faults are compared only for retained stores against Rust's typed
refusal. Field-boundary mutation/error probes are explicit test seams, not
claims that ordinary native reads are virtual callbacks.

The independent core replay reran all 678 rows against both pinned PE images
and all three explicit x87 precisions without changing author fixtures. Four
Rust tests passed, including the new full-trace replay and existing formation
and direction-table regressions. Final saved replay and source pins are in the
adjacent `independent-review.json`.

No vendor SDK implementation was used. No production files, original assets,
executables, saves, profiles, or releases were changed by this reviewer.
