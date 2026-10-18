ResourceNoNiTextures named UVar (2026-10-17)

The actual source-created Bool now lives inside the same Ui.uvars owner as
the existing indexed view Bool cells. It has no fabricated constructor index
or default allocation. Missing source/current ownership returns an error.
No current/default value is duplicated in generic UI string maps.

Ui.uvars.resource_no_ni_textures_current() returns the raw current u8.
The real Texture5A8CD0 backend must compare EXACTLY1, then OR flags0x40;
other nonzero bytes do not enable the native branch. Getter use does not
claim texture decode, GPU upload, resource flags, or readiness completion.

The native proof executes whole parser and4B3C10 source creation/overrides,
real4B2E60 named lookup(kind1), then5A8D3F..80 cache/read/flags prefix.
It ends before the decoder branch. Fifteen paired cases/30 separate
original-v14/all-PC executions agree, with3,878 executed instruction pins.
The same cached allocation is reused after current byte updates; no Bool
creation, lookup or prefix success is supplied by a callback.

Final aa-ui tests8/8 plus same-binary UI map regressions2/2 passed, including
actual original/v14 source loading. Existing indexed Bool/reset native
fixtures are replayed too. The current flags0 named byte survives indexed
reset and remains writable through the same UI raw/private override paths.

native_texture_uvar.py without arguments freshly replays and verifies the
fixture without overwriting it. findings.json records exact source/test/exe
pins, declared host transports, current scope and remaining integration.
