Phase7 live actor actions, state effects and mixed property lifetime

Production scope:
- aa-actors/ability_flags.rs ports4D1350 and Element/Organization action contributors, command availability, resupply codes, attack-class query and578140 organization status. Mind contributors are the separately verified aa-mind/action_mask implementations. Live actor/player/required contributor callbacks must be bound; none may default to success.
- aa-actors/state_effects.rs ports4D4A90/4D16B0 ordered state property/view/FX/group effects,57C400 entrenchment timestamp branch,4B4590/45E620 reverse view selection and5035E0/503390 recursive property applicability. Live property lifetime, Character transitions, Economy570020, FX emission, group removal and pending organization application remain required existing-runtime operations.
- aa-body/property_flags.rs handles kind0 FlagModifier, mixed ordered numeric descriptors and4FE460 consequence selection. Native flag0 writes Mover+4C;1 calls actor stop then Character pause/resume;2 calls Mover576800;9 conditionally refreshes Economy;11 rebuilds actions. Other indices have no actor callback in this native method. Valid registered names0..11 are recorded, NOT_DISPOSABLE is11. Index matching is full u32 while x86 shifts mask by31. Modifier.flags is distinct from Properties+D8 metadata flags.
- property_effects.rs now has backward-compatible generic Definition/Definitions defaults and required insert_with/remove_with dispatch. Existing numeric APIs remain identical; existing numeric aggregation moved into a mixed-scan helper with no arithmetic change. Existing111 native numeric sequences/390 transitions and partial-failure tests are rerun.

Native evidence:
-290 full ability rebuild/member scan inputs.
-276 Organization status inputs, covering conditional float reads and exhausted-state query.
-192 full state enter/leave inputs acrossPC24/53/64, with callback ordering, view reselection after Economy mutation, property prerequisites, FX list gate and entrenchment timestamp.
-128 actual acyclic recursive/vacant member property applicability inputs, with disabled instance prerequisites. No membership/prerequisite decision hooks.
-111 mixed Flag/Numeric lifetime sequences/246 transitions acrossPC24/53/64,17 actual flag loader tail inputs, and all12 flag domain registrations. Additional Rust callback-error tests preserve preceding mutations.

The instruction pins include observed external seam entry bytes; they do not claim execution behind intercepted callbacks. Original and accepted v14 visited bytes are compared. Definitions/list identity remain stable during a call; live component fields and instance state are reread where native code reads them. Invalid native pointers/cycles/index/count corruption are explicit errors; this is not a portable reproduction of native undefined behavior.

This checkpoint does not implement complete PostCreate/PostExisting, rendering FX, world publication, a complete playable battle, save writing or network multiplayer. Source parsing of property/actor-state records remains the aa-rules adapter's responsibility and must preserve all descriptor kinds. Unsupported kinds must never be omitted. Required live callbacks are integration blockers until their concrete adapters are available. Allocation failure and callback errors retain prior side effects as documented.

Peer review correction: Element571640 now rereads its live owner after the resupply callbacks before checking dead flags. Two original owner-rebind cases verify opposite dead flags and the final action write recipient; no immutable-owner assumption remains.
