# Independent Builder source review

Accepted the bounded native Builder source projection. Fresh native replay matched all 252 synthetic rows and six actual-source rows across original/v14 and PC24/53/64. Five focused Rust tests passed, including the real 1084-definition census (five Builder owners and 15 references). Nine independent instruction ranges and three native vtable predicate slots also passed byte/decoded-instruction checks.

The review caught and repaired the initial exclusion of `RANDOM_ACTOR` kind19: it inherits ActorData construction/loading and is accepted by native FindActor. Both owner and target classification now include it. Source owners remain provenance-bound; authored references translate once; stable nodes preserve duplicates, head/tail order and earlier callback effects at first failure.

This accepts a Builder callback subsequence with explicit lookup/allocation seams. It does not certify complete global deferred resolution, full historical source loading, arbitrary allocator reentry/failure, exact diagnostic formatting or a playable battle. See `independent-review.json` for commands, pins and precise qualifications.
