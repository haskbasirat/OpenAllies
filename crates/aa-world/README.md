# aa-world

An original-instruction-backed outer world dispatcher with explicit leaf
callbacks. It implements update/phase/list ordering, not complete gameplay.

Use `World::step(&mut callbacks)` to request one tick and
`World::transition(phase, &mut callbacks)` for the recovered phase transition.
Queues, manager and actor slots retain their native numeric names.
`enqueue_back` and `enqueue_front` expose laboratory lists. The native
removal request/dedup API 0x4F6760 is `World::request_removal` (phase8-tick
oracle: newest deferred request drains first; immediate remove/destroy in
phases 3/6 and phase 0 of session mode 3). See `docs/WORLD-ORDER.txt` for the full
contract, evidence and limitations.

`cargo test -p aa-world` compares complete callback traces and final states
against 12 scenarios executed in the original x86 functions, plus 11 removal
request scenarios (research/phase8-tick/native-removal-request.json).
