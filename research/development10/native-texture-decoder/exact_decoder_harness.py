"""Extract the current exact CPU decoder and actual PixelData methods for a
small Rust replay; no client/stream/Resource/Task/GPU integration is inferred.
"""
from pathlib import Path
import hashlib,json,re
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
decoder=ROOT/'crates/aa-client/src/native_texture_decoder.rs'
pixels=ROOT/'crates/aa-runtime/src/resource_texture.rs'
tests=ROOT/'crates/aa-client/src/native_texture_decoder_tests.rs'
native=ROOT/'crates/aa-client/src/native_texture_pixel_tests.rs'
a=decoder.read_text();b=pixels.read_text();c=tests.read_text();d=native.read_text()
core=a[a.index('fn decode_bytes('):a.index('#[cfg(test)]')]
storage=b[b.index('pub struct PixelLevel'):b.index('struct Input {')]
encoding=b[b.index('#[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub enum Encoding'):b.index('impl Encoding')]
basic=c[:c.index('#[test]#[ignore=')].replace('use super::*;','',1)
comparison=d.replace('use super::*;','',1).replace('"../../../research/skirmish-20261017-texture-reader/native-reader-pixels.json"',json.dumps(str(ROOT/'research/skirmish-20261017-texture-reader/native-reader-pixels.json')))
text='use std::io::Cursor;use image::ImageDecoder;type Result<T>=std::result::Result<T,String>;\n'+encoding+'\n'+storage+'\n'+core+'\n'+basic+'\n'+comparison
out=HERE/'exact-decoder-tests.rs';out.write_text(text)
report=dict(scope=__doc__,sources={str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest()for p in[decoder,pixels,tests,native]},extracted_sha256=hashlib.sha256(out.read_bytes()).hexdigest())
(HERE/'exact-decoder-inputs.json').write_text(json.dumps(report,indent=2)+'\n')
print(str(out))
