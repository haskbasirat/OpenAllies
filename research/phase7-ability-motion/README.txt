Phase7 Element ability wait/pursuit and Mover target assignment

Implemented:
- aa_mind::ability_motion::wait: original52D820, actual action::stop52D660,
  recharge/schedule ordering, post-schedule bombard and decision checks, utility
  comparison through the original extended-to-binary32 boundary.
- aa_mind::ability_motion::pursue: original52EF40. Execute required real
  embedded Move7C Think first, reread actor/active Mover, compute effective
  range minus epsilon, optionally subtract moving target displacement only
  when the native facing dot product is positive, then assign target and
  dispatch the signed Mind decision result/reconsideration.
- aa_movement::target::set_target: original576AD0. Write the target GTID first;
  choose the input range only if strictly greater than the extended product
  of current world delta and requested speed. Preserve negative and signed-zero
  behavior. No guessed distance/route or extra limit.
- Additive aa_combat::admission::effective_range: original4C8270 calls the
  existing4BE910 base/modifier logic then adds local fuzzy range times instance
  fuzzy range without an early store. Prior admission paths are unchanged.

Evidence:
native_motion.py executes213 original wait/pursuit traces and42 direct target
setter cases across PC24/53/64 nearest precision. It also executes actual
52D660/529C30/4CF110/4DB0F0 rather than replacing their outer control flow.
Explicit fixture leaves are recharge, weights, range, MoveThink537B90, Mover
stop, stance, scheduler, target moving/speed and Mind virtuals. These leaves are
not claims of complete original bodies in this particular replay. Mutable
Think/scheduler/stop/speed/decision callbacks exercise live actor/target reads;
the target speed callback also changes delta to prove the earlier capture.
native_range.py separately executes144 complete4C8270/4BE910/501D50/501DB0
cases, with no gameplay hook reached. Three successive binary32 residual
stores check extended results beyond their first binary32 rounding.
Total399 new original executions. Six focused Rust tests passed: five new and
one existing admission regression. Strict scoped Clippy passed.

All visited original instruction windows and complete directly ported routine
ranges match the accepted v14 executable. Visited hook entry instructions
are not evidence that their intercepted bodies executed. Native stack/callee
register assertions belong to the original harness, not Rust ABI claims.

Integration contracts and limits:
The adapter must bind the real owned Ability/Performer/Modifier state to the
already implemented admission and weights functions, actual scheduler events,
aa_spatial::moving_state::excluded, actual Mover state, and the concrete
aa_mind::element_move::think537B90 from its separate owner. No successful
default callbacks, mock route, instant attack or fabricated cooldown is supplied.
Definition/weight/recharge and movement predicates are coherent native leaf
reads. Mutable lifecycle boundaries may change the state exposed by their
explicit parameters; arbitrary unrelated alias writes are not claimed.
The active pursuit branch requires a nonnull target and Mover. Native invalid
null dereferences become explicit errors; they are not successful game paths.
Finite nearest PC24/53/64 arithmetic is supported; NaN/exception flags and
binary32 overflow are outside the finite numeric API. Errors preserve earlier
effects (including a committed target ID/range); callers must not retry blindly.
This work does not independently establish a playable match, live UI orders,
multiplayer, projectile rendering or complete movement world integration.

Validation commands (workspace root):
python research/phase7-ability-motion/native_motion.py
python research/phase7-ability-motion/native_range.py
cargo test -p aa-mind --test ability_motion_native
cargo test -p aa-movement --test target_native
cargo test -p aa-combat --test native_admission
cargo clippy -p aa-mind -p aa-movement -p aa-combat --all-targets --no-deps -- -D warnings
