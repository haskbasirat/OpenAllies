"""Mechanically extract the unchanged geometry arithmetic/test kernel only.

The compose_world wrapper is omitted; its shared implementation is already
covered by view_scene_init's separate native suite, not this harness. No
implementation callbacks or arithmetic are replaced with stand-ins.
"""
from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
source=ROOT/'crates/aa-runtime/src/resource_model_bounds.rs'
text=source.read_text();start=text.index('///Reuse the already proved45CAB0');end=text.index('///89FBF0',start)
out=HERE/'model_bounds_harness.rs';out.write_text(text[:start]+text[end:])
print(json.dumps(dict(source_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),harness_sha256=hashlib.sha256(out.read_bytes()).hexdigest())))
