This checkpoint ports native physical texture storage metadata, independently
from the modern decoder's RGBA pixel payload. `reader-freeze.json` pins the
production sources, exact source test executable and unchanged native evidence.

`NativePixelLayout::from_input` accepts the actual selected `TextureInput` and
the reached current Source+48. It retains the Resource identity and input hash.
The concrete loader must validate those plus the current mode again before it
selects the corresponding decoded source levels and publishes the actual pixel
owner. The plan cannot set Resource flags, Source+3C or GPU readiness.

The native producer constructs the real 89A950 dev converter. Its reader list
and 897670 extension traversal select the actual DDS, TGA and PNG readers. All
three real pixel readers run, including the PNG libpng/inflate implementation,
875AC0 pixel constructor and 8C5A40 native arrays. The successful allocator/free,
OS thread/mutex/heap, ASCII splitpath/stricmp and initial setjmp paths are named
transport seams. Failed longjmp, allocation failure and short-stream behavior
are outside this proof. The script emulates unchanged original and v14 bytes;
it does not execute a game process or modify original assets.

The 165 rows run separately in both images at all three x87 precisions, giving
330 executions and 5,718 executed instruction pins. The exact production core
test matches native format words, widths, heights, offsets and stride for every
accepted row, and refuses the five native DDS header rejection forms. A separate
test checks portable invalid/truncated/oversized guards. This does not claim
all possible image formats or whole-game success.

DDS DXT1/3/5 retains compressed native offsets. A 32-bit RGB DDS produces the
native canonical RGBA32 format. Source+48 skips a top level only while levels
remain above one and both current dimensions exceed four pixels. The plan's
source-level range identifies the same actual decoder levels to retain.

Truecolor TGA types2/10 with24/32 bits produce native RGB24/RGBA32 respectively.
Supported8-bit nonpalette grayscale/RGB/grayAlpha/RGBA PNGs produce native
RGBA32. Interlaced,16-bit or palette PNG, palette/other-depth TGA and16/24-bit
DDS conversion remain explicit required domains. The finite portable binding
also rejects impossible sizes and level counts at the native local-array
boundary rather than pretending to reproduce memory corruption.

Native encoded accounting uses these native offsets; decompressed RGBA length
never replaces it. Any later resize, Source renderer upload, discard policy,
Resource publication and VWorld completion remain the actual separate owners.
