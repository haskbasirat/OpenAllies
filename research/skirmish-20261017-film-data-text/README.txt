Film DataText backend, 2026-10-17

Production: crates/aa-runtime/src/film_data_text.rs
Tests: crates/aa-runtime/src/film_data_text_tests.rs

Bounded behavior
---------------
PreparedDataText retains one growable memory child and one opaque WorkPath from
the SAME resource_depot::Application. It has no public path/metadata constructor,
Clone, fabricated DataFileResource, second resource manager, or copied film stream.
prepare performs the actual logical last-dot extension replacement with uppercase
TGI, asks the shared namespace for an actual absent DataFile result, and verifies
the Film's last depot is the same current work depot. Unsupported foreign depots,
positive DataFile lookup/load/class/ref/unload ownership, and parent-dot paths
outside this safely bound work domain remain explicit required errors.

Native instructions
-------------------
59FDA0 unconditionally constructs the text wrapper, calls4738C0's CURRENT text
producer, serializes MAPSTRINGS, calls859B30, then reads the CURRENT separate-file
option. film_map_strings (another author's module) owns that caller order. This
backend does not sample the option or an earlier text-table snapshot.

856E90 calls877AD0, stores text vtable AB4890, clears encoding bytes C/D/E,
and opens with actual [AB4788]=0x2010A plus0x40000/0x20000. 877B80/877940
select8C62E0's memory child, vtable ABE7D8. 8C5FE0 opens0x4000 bytes.
8C6190/8C6090 grow it; the Rust Vec remains growable without a hard buffer cap.

856BF0's default C-locale path calls970FDF/970F7F. In this explicit C-locale
scope U+0001..U+00FF write one byte; every LF receives a preceding CR, including
an LF already preceded by CR. There is no BOM. Logical embedded NUL is outside
the admitted source API and refused before appending any bytes. Unicode/codepage
changes, wctomb failure, UTF8/UTF16 modes and CRT startup are not certified here.

877AC0 dispatches child8C62C0, which forwards current buffer/current length to
the destination stream+4. It adds no filename, count or DataFile parse-tree header.
877A50 constructs an actual858660 file wrapper with default flags, forwards the
SAME memory child, then destroys it. 876C60 maps these flags to GENERIC_WRITE,
no sharing, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL. 876720/8766D0 calls
FlushFileBuffers before CloseHandle. The Rust optional writer uses the same
path receipt, Windows share_mode(0), create+truncate, writes and sync_all.
The regular successful IO payload is tested on Windows; native handle
inheritance, sharing/failure/short-write/SEH/allocator-unwind equivalence is not
claimed. Safe Rust errors retain PreparedDataText and any completed stores.

859C60 uses the LAST dot anywhere in the string, including directory components;
85AB30 appends the actual A0CF30 literal TGI. The native filename probe includes
empty/no-extension/hidden/trailing-dot/multiple-dot/parent-dot examples. Runtime
prepare additionally checks full physical filename agreement and safe work-root
binding instead of manufacturing a different path for unsupported cases.

859B30 calls resolved DeleteFileW (B3641C; delay thunk984244) and returns its Bool;
59FDA0 ignores it. WorkPath's safe regular-file removal returns real success or
failure, both ignored here. Unavailable/dead/wrong Application/path errors remain
errors. Only rewrite Userdata can be written; original files/profiles are untouched.

Proof
-----
native_data_text.py: 42 rows /84 original-v14 executions /all3 PCs. Real
constructor/open, writer/CRT C-locale leaf, buffer growth beyond16KB, forwarding,
filename kernel and release execute. Successful heap/CString/CRT-thread transports
are named, and877A50 is first tested with named output ctor/dtor transports.
859B30 executes with both true/false DeleteFileW transport results.

native_file_text.py: 12 rows /24 executions /all3 PCs. The real877A50->858660
->876C60 path, buffered raw file write, flush and close execute. Actual import
slots identify WriteFile/FlushFileBuffers/CloseHandle/GetFileSize/SetFilePointer;
CreateFileW and preexisting-parent directory preparation are named transports.
There is no successful fake DataFile class/load callback.

Normal Python invocations immutably compare fixtures; --write is generation only.
Use PYTHONPATH=E:\Games\Axis and Alies\reconstruction\tools\python-packages.
Address-byte pins distinguish intercepted image entry addresses from genuinely
executed instructions; counts in findings.json are not full engine coverage.

The actual Rust backend test constructs source-bound Header(false) and Creator
fields using current real Catalog/Bank/process/variables, then uses the SAME
Application, Film allocation, manager/depot and regular-file transport. This is
an explicit constructed-fields backend fixture, NOT full5A31D0 managed-world
capture, whole map/mission activation or playable battle completion. It checks
raw MAPSTRINGS decoding, physical .TGI output, genuine deletion after a labelled
external-file race insertion, ignored absent-file failure, positive existing-file
refusal, and dead-Application refusal without discarding prior bytes. It only
removes its two resolved regular output files, with no recursive filesystem IO.

Test build override: CARGO_PROFILE_RELEASE_LTO=false and
CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16, -j4 --release --locked --offline.
Shipping default ThinLTO settings are unchanged.
