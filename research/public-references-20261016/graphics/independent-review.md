# Independent metadata-retention review

Accepted with no blockers. The reviewed production diff adds only the three
authored base-texture descriptor words to conversion output and an optional
Rust representation. It does not choose a sampler, interpret enum values,
change UV selection, or supply defaults when source metadata is absent.

The independent probe read three original NIF 10.1.0.0 files. It intercepted
the existing reader at each TexDesc boundary and unpacked the on-disk little
endian clamp/filter/UV words directly. Six actual base descriptors and one
material without a base descriptor matched the exported metadata. Source
hashes remained unchanged.

Using the backed-up converter and the new converter on the same decoded NIFs,
the probe compared their entire converted outputs after removing only the new
metadata field. All geometry, transforms, indices, UVs, bounds, material fields
and remaining output matched. Conversion stayed in memory; no assets, caches,
converted-model manifests or releases were regenerated.

The focused release/locked/offline Rust test passed all three cases. It covers
typed decode, RawMesh preparation, serialization, old payloads/default absence,
uninterpreted unknown numeric words, rejection of incomplete present records,
and the explicitly synthetic descriptor-without-source probe. An absent field
remains None and is omitted on serialization; no old payload acquires authored
sampler defaults.

The client Material-to-CpuMaterial projection does not consume this field.
The change therefore retains information for future work but does not fix or
prove visible filtering/clamping behavior. Public Niflib/NifSkope sources were
downloaded as references; their implementation is not linked or copied into
this bounded change. Their tool defaults remain separate from native AA
behavior. All 18 selected reference-file hashes and author evidence pins were
verified.

This acceptance is limited to metadata preservation in the checked formats and
seven material rows. It is not a whole-corpus, visual, animation, billboard,
GPU-sampler or native rendering parity result. No native/SDK binary or newly
downloaded implementation was executed. The reviewer changed no production
file. Exact source, fixture, report and test hashes are in
`independent-review.json`.
