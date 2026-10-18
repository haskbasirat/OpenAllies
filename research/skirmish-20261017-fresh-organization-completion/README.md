# Organization completion checkpoint

This checkpoint extracts the existing ordinary4FA150 member tail into `aa_mapworld::fresh_creation::finish_organization_members`, keeps the old entry point delegating to it, and adds the previously missing portable4FA3D0 hero postpass in `aa_runtime::fresh_organization_completion`. There is one ordinary-tail algorithm. The runtime module reexports it.

The native oracle executes4FA3D0's actual component/layout/member reads and branches in the original and v14 PE at all three precision settings. It records411 cases,822 native runs and78 executed address-byte pins. Its child services are explicit:4FA150 creation result, member virtual108 owner calls,4E2A60 query,57B830 attachment, and positive notification. The notification service skips4FA4A0..4FA54F's localization/formatting/temporary cleanup subtree. This does not prove actual actor creation, Kingdom hero insertion or notification completion.

The exact production runtime module and its tests run through a small `rustc --test` harness, with the actual aa-mapworld library and serde_json dependencies. Both tests pass:411 native hero rows compare complete event/error/mutation prefixes, and the ordinary-tail test reads the same mutable parameter field after each prior spawn, retaining preceding spawns on a later getter error. Existing aa-mapworld fresh_creation regression tests pass4/4 through the unchanged old entry point and new shared tail. No additional runtime unit binary was linked.

The native hero cases cover signed groups-1/0/1/2, empty results/counts, null members, first owner absence, the second owner+1FC derived-null branch, false/true recruit and attachment results, required child failures, changed current Organization identity, changed captured Organization layout, and a member definition changed by the first owner callback. The definition captured before that callback is retained. The portable valid-identity adapter must reject a second raw NULL owner rather than calling it a null table.

`integration-spec.md` defines which reads use the captured Organization and which reread actor+7C. Root owns the concrete `stored_organization_spawn` member creation adapter and whole mission acceptance. Positive hero/recruit/notification children remain required through non-default trait methods.

Reproduce native evidence without rewriting it:

```powershell
$env:PYTHONPATH='E:\Games\Axis and Alies\reconstruction\tools\python-packages'
python research/skirmish-20261017-fresh-organization-completion/native_hero_postpass.py
```

Run the retained exact-source binary without compiling:

```powershell
& research/skirmish-20261017-fresh-organization-completion/exact-hero-tests.exe --nocapture
```

The parent runtime cohort may filter `fresh_organization_completion::tests::`. The `--lib` production check typechecks the exported module; the exact standalone tests and existing mapworld regression provide the execution evidence. Fixture and source hashes, preserved source snapshots and command outcomes are recorded in `findings.json`.
