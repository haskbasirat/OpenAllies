Shared World name table and concrete trigger field bindings

names.rs ports the observable World+F0 name operations. Original4F7D00 starts
59 buckets/count0 with automatic growth. Alias4F94D0 skips an empty name, then
prepends without deduplication. Publication4F9510 clears Thing.flags bit31,
skips an empty name, removes only the first equal name and prepends the new ID.
Lookup4154D0 selects the first case-insensitive match;5C9180 removes one match.

Native593570 growth is observably significant for duplicates: when count is at
least twice the bucket count, the new bucket count follows the exact +1/+3/+5
test against divisibility by3/5, and prepending rehashed nodes reverses equal-name
order. The Rust map therefore retains an ordered vector per equal name plus
actual count/bucket growth state. Cross-name bucket traversal is not exposed as
simulation order. Memory capacity does not become an entity/gameplay cap.

Native named actor/Kingdom getters resolve the selected raw GTID through the
generation table, then apply virtual+B4/+A8. A stale first alias never falls
through to an older live alias. Rust resolve operates on the actual central
IdentityRegistry. Lookup of an absent name returnsNone in the valid logical
name domain; accidental aliasing of the native missing-name string pointer to
a registered GTID is excluded, as are arbitrary native pointer identities.

11 actual executable sequences comprise2,569 registration, removal, rehash and
lookup transitions. Native hash405030 and C-locale wide compare96CAD6 execute.
Seams are string lifetime (stable buffers), allocation, the CRT thread-locale
accessor supplying C locale and4F4C40's later spatial publication. Registration
class branches in these fixtures are false; full actor PostLoad/PostExisting
side effects are not covered. Every operation compares count/bucket changes;
non-alias observations additionally compare full per-name duplicate ordering.
Three real growth points and signed generation mismatch/class rejection appear.

The supported portable name domain is ASCII UTF16 without embeddednull. Other
locale/string cases fail explicitly. Publication flag writes precede an error.
The initial implementation deliberately was not accepted until growth order was
inspected: it would otherwise have missed duplicate reversal during rehash.

trigger_world.rs supplies actual canonical store bindings for raw IDs, checked
ObjectKey->Body GTID conversion, named actor/Kingdom lookup, mutable Kingdom238,
staged name publication and alias registration. Name population is an explicit
publication action; it is never reconstructed by scanning actor strings.
Presentation state is owned and caller-supplied. The current module is not a
complete WorldProgram::World: live Mind command dispatch, property execution,
SAI handles and Kingdom membership must bind their actual runtime owners.

The source composition test explicitly publishes the three restored tutorial01
Kingdom names, checks canonical flag/victory mutation and immutable saved bytes,
then constructs/registers a source usa_infantry to check arena key vsGTID, class
selection and alias handling. The chosen test publication sequence is explicit
fixture setup and is not a claim of complete native PostExisting order. Actual
game readiness remains false until remaining Load/PostLoad/world services finish.

Reproduce native_names.py then record_findings.py with original files present.
No original game/assets/save/release files are modified.
