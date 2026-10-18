Positive Commander StartMission4E6DB0, bounded integration

The existing Kingdom mission adapter now reads the actual Commander's canonical
Faction+30 starting Property list from the same completed source Bank. The list
uses the source precision and resolves Property names at the Faction's original
load position. Native48BDC0 prepends entries, so traversal is reverse authored
order and keeps duplicates. Unknown references/reloads remain errors.

After the unchanged stockpile/resource/profile/start-vector sequence, the adapter
checks the actual Nation/Faction relation. Invalid-faction localization remains
a required real service. On the valid path it lazily reads the SAME current SVar
StartingExperience134 and invokes aa_economy::kingdom_start::set_commander_experience
(native4CDF10) on the SAME Commander experience allocation. Status19C and started
23B follow. Positive Property grants are not acknowledged as successful: they
must use the actual staged actor/instance/effect/event owners when reached.

StartingExperience is registered internally:507FB9..507FE7 constructs a Fixed Var
with flags0 and initial0 and registers index4D, hence cache+134. It has no shipped
TGI SVar declaration. The actual WorldRule default200 is applied by474C50 BEFORE
snapshot/reset. Its current value now lives in the existing load_view_variables
State, beside the other current SVar cells. Both source reset and diagnostic
publication validate private source declarations against public Settings when
an override exists; the normal declared-absent constructor path is explicit.
The retained actual Rules Owner publishes this same cell in registration order.
The fixed setter preserves raw words and does not substitute UI limits.

Property allocation State now has require_in_bank/bind_in_bank for the SAME Bank
after staging moves it out of Catalog. The original pre-stage APIs remain and
completed-cache validation retains its shared Bank loan behavior. Mutable source
interning is only needed for initial binding. Source failures remain retained.
ActorStore.split_property_state borrows the same State and a disjoint view of the
same entities/identities/Mind/Ability/animation storage; no entity/component,
instance list or Bank is removed, cloned, reconstructed or copied back.

Native immutable evidence (original/v14 and all three x87 precision modes):
- native-starting-experience.json:15 paired rows,315 executed pins. Actual
  registration constructor/default/reset/publication; CString/allocator/registry
  capture/lookup/log transport is explicit. Source default200 is an input from
  the actual WorldRule, not a claimed full TGI parse in this oracle.
- native-faction-properties.json:24 paired source rows plus3 paired constructor
  intervals,5008 executed pins. Actual parser/getters/tokenizer/prepend stores;
  explicitly skips unrelated Faction art/Abilities using the real prologue and
  stack push. Property Find is a canonical identity service. Heap/string-pool,
  line source/mutex/EH services are inherited harness transport. Intercepted
  entry bytes are excluded from executed pins. The pre-pin-correction JSON is
  retained only as superseded evidence: it mistakenly included856370's seam
  entry. All semantic row outputs are unchanged by that correction.
- native-commander-start.json:384 paired rows,199 executed pins. Actual whole
  positive Commander4E6DB0,4E39D0,4730D0,4CDF10; normal rows have no game-code
  seams. Required rows stop at the first grant/localization entry or actual
  current134 memory read and preserve previous writes. Native allocations are
  explicit valid fixture inputs, not complete saved/source startup.

Final current-source Rust cohort passes24/24 from commander-tests-final-v4.exe
(SHA256 e30754e9202dfe4ab8f38f1834f2b443a29a9a27175802ebab178cf04bf3a46d).
The five Commander tests include all384 paired native rows plus the actual
registered24-Faction census and72 constructed-Kingdom starts at all three live
PCs, with compatible actual Nation references and one sourceDouble53 Bank
deliberately moved out of Catalog. ALL24 actual source Faction property lists
were empty. The same experience pointer, current rule value, stockpile allocation
and Property source State were retained. Cached Property validation is also
repeated while the same Catalog Bank has an existing shared loan, then repeated
against the moved Bank; a wrong/empty Catalog Bank is refused after the move.
The remaining19 focused regression tests cover current mission variables,
configuration, camera, merge, finalization, task gates, Rules and actual
three-map Creator/resource ownership. These are bounded leaf/regression tests,
not successful whole-map startup. Exact source/capture/log pins are in findings.
The earlier4/4 v1 capture used constructor-null Nations and remains historical.

An independent raw Header90 read shows all7 Firefight source parameter records
have empty Handicap, starting Property definitions and merge aliases. This is
source-header evidence and does not certify later mutable launch selections.
No successful selected full mission, playable battle, campaign or multiplayer
claim follows from these tests. Original binaries/assets/saves/profiles were not
modified, and no vendor SDK source was integrated.
