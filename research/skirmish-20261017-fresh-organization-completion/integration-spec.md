# Live integration contract

`4FA150` calls the existing actor creator, performs the actual IsGActor assertion, and then invokes `finish_organization_members`. The extraction leaves that existing entry point and its native regression unchanged. A separate runtime adapter may invoke the extracted tail only after the same creation and assertion prefix has completed.

The adapter retains the real created Actor identity and the original Organization component identity. It borrows the same layout/groups/member arrays, registry, mutable request Params, RNG, queues and recursive creation services. It creates no member snapshot or alternate owner.

| Read or child | Native owner and order |
| --- | --- |
| Organization presence | Capture actor+7C once at4FA2EB. Absence returns the created actor. |
| Leader slot | Captured Org+18 current layout+18, once. Negative skips leader creation. |
| Slot definition | Captured Org+18 current layout+28[slot], then captured Org+1C[group]. |
| Leader spawn |57AD30 on the captured Org, flag0. |
| Nonleader traversal | Signed slot/count; reread captured Org's current layout/count at every tail. |
| Nonleader spawn | Read fresh actor+7C, then current Params+51, then57AD30(slot,flag). The getter must borrow that same field, not an entry-time copied byte. |
| Final population | Fresh actor+7C→57A730. Zero logs404F60 first, then destroys4F5490, and returnsNULL. |

`hero_postpass` implements only the post-4FA150 portion of4FA3D0. Its `None` input skips all component reads. A nonnull result captures the original Org once, reads signed group type<=1, then rereads the actor's current Org for the member. Member+4 definition is captured before the two distinct current virtual108 owner calls. The first NULL owner skips; the second owner+1FC derived-pointer NULL test is distinct. A concrete generation-safe adapter must reject a second NULL owner when its derived pointer would be invalid1FC; it must not convert that into the native derived-pointer-NULL branch.

The actual4E2A60 table query passes the captured definition and literal1. Its positive result calls57B830 on fresh actor+7C. A successful attachment requires real localization/name formatting and notification before the loop-tail reread of the original captured Org's current layout/count. A child error preserves every prior mutation. No recruit, attachment or notification callback has a successful default.

`HeroWorld` is the portable integration interface. Its associated identities should be the existing typed identities or opaque borrowed-owner capabilities. The core does not construct an ActorStore, layout, Kingdom table, source definition, or presentation owner.

The current checkpoint proves the portable completion constituents. It does not claim that recursive member creation, positive hero insertion, notification, or a fully initialized same-world battle has completed. Parent owns the concrete `stored_organization_spawn` adapter and natural mission acceptance.
