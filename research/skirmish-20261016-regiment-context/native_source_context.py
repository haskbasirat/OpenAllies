"""Original528C20 integration fixture with real source IDS and owned containers.

This is a synthetic saved payload, not an existing game save. The native named
registry seam only returns distinct pointers; Rust integration tests separately
resolve these IDS against actual v14 definitions. Constructor, command reader,
state lookup, path, targets and join containers execute original machine code.
"""
from pathlib import Path
import sys, json
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'skirmish-20261016-regiment-load'))
import native_regiment as r

def stop(version=3, seed=0, empty=False):
    # Actual plain command3 saved format; distinct instances retain raw bytes.
    return (r.W(3) + r.text('stop') + r.W(64, seed) + r.text(None)
            + r.W(0, 0x80000000, 0, 0, 0x3f800000) + bytes([2, 255])
            + r.text(None) + r.text(None))

def main():
    r.command = stop
    source = r.payload(21, 0x528c35, True, 3)
    assert source.count(r.text('bunker')) == 1
    source = source.replace(r.text('bunker'), r.text('usa_infantry'))
    cases = []
    for pc, cw in [('Single24', 0x7f), ('Double53', 0x27f), ('Extended64', 0x37f)]:
        world_time = 0x427b0000
        native = r.Machine(source + b'AFTER', cw, world_time).regiment()
        assert native['outcome'] == 'loaded' and native['consumed'] == len(source)
        cases.append(dict(version=21, precision=pc, payload=source.hex(),
                          world_time=world_time, native=native))
    doc = dict(scope=__doc__, original_sha256=r.H(r.P.b), v14_sha256=r.H(r.V.b),
               producer_sha256=r.H(Path(__file__).read_bytes()), cases=cases,
               instructions=[dict(va=hex(a), bytes=b) for a, b in sorted(r.VISITED.items())])
    path = HERE / 'native-source-context.json'
    path.write_text(json.dumps(doc, indent=1) + '\n')
    print(json.dumps(dict(cases=len(cases), sha256=r.H(path.read_bytes()))))

if __name__ == '__main__':
    main()
