Character default-stance reconstruction checkpoint

Native authority:4CFC80 actor dispatch;563BA0 default stance;563770 playback;
4DB0E0/4CFC10 activation;4DB080 both Character enter/leave;6A71D0 true
for its inactive permission and Updatable predicate;4D0720/45E690 view;
4D0750/45E660 ability set;49B4D0/85D840 idle repetition;4DB120 timing;
42A590/429080 empty Animation FX. Both PE images,PC24/53/64.

The new borrowed-fields coordinator owns no Character. Every mutable loan ends
before a world callback and later reads reach the same allocation. The legacy
runtime::play/default_stance entrypoints delegate to this one algorithm.
The live early-return guard reads Queues.owner_head, not the old constructor
header word. Existing native/legacy fixtures retain their explicit raw head.

map_character_stance::default_stance borrows the actual Staged actor arena,
active component, adopted spatial owner, AnimationBank, deferred queue, clock
and RNG. Its canonical source Arc and source Bank precision are validated.
Source precision remains separate from the explicit live arithmetic PC.
Character activation uses the existing movement_world::activate_component;
both native Character callbacks cancel its same queue owner. Native seed logs
borrow the actual A thenS synchronizers before consuming the shared RNG.
Animation timing/idle list/interval inputs come from the same source binding.

Positive pending variant remap/property stance changes, positive state view,
nonempty Animation FX, unrelated component activation and movement visual
scale remain required reached services. There is no generic success callback,
character extraction, cloned mutable state or automatic gameplay readiness.

Evidence so far:
-Existing accepted720 animation rows independently reexecuted on original and
 v14:1440 executions; strict FS-only page0 guard; fixture unchanged.
-New native-stance-owner.json:96 paired rows,192 executions,298 instruction
 pins plus exact Character/Updatable vtable entries. Real activation and
 emptyFX bodies execute. Queue insertion/cancellation and integer logger
 leaves remain explicit transports. Source data are explicit raw fixtures.
-aa-animation native_runtime/scoped_runtime/borrowed_runtime:6/6 tests PASS.
 All existing native rows and old scoped String identities remain covered;
 two new tests cover same Character allocation, callback rereads, every
 tested service-error prefix and authoritative queue-head selection.

Additional source proof:
-native-idle-bounds-source.json:30 executions of the exact required integer
 getter/store prefix49E93F..49E965; missing-first/missing-second retain their
 native partial stores. Separate static-only context pins prove49E0A5 finds
 View and ESI retains that pointer through the getter island. Lookup transport
 is explicit; this is not a full source parser/49DC40 LoadData execution.

Pending validation:
-The first direct actual-source capture exposed the real source field path
 mistake (actor-root instead of View). Both production and test were corrected
 to View.idle_loops_min/max; failed log remains preserved. The parent will run
 the next coherent test cohort; no additional private runtime relink is begun.
-A direct actual-source Character constructor/service test for three original
 infantry definitions at all three live PCs awaits that parent cohort.
 It is explicitly a partial constructor/service fixture, not an initialized
 Staged mission. Native source-capture replay follows its successful run.
-The full actual Staged public wrapper and4D6F10 caller require the parent's
 genuine selected-map initialization path. No such successful record is yet
 claimed by this checkpoint. Existing unsupported factory tails remain open.
