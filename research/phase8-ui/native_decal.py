"""Unicorn oracle for the original decal view-element geometry in AA.exe.

Executes the ORIGINAL x86 code of:
  0x5E6AB0  DecalVETiled piece rects (9-slice, pieces 0..8);
  0x5E7790  DecalVETiledDirectional piece rects (-> 0x5E6D40 horizontal,
            0x5E6DC0 vertical), pieces 0..2;
  0x5E7000  DecalVE UV fit for `stretch = false` (aspect fit / crop).
Seams: the texture size getters 0x5A6430 (width) and 0x5A64A0 (height)
return the size recorded for the fake texture handle (they read texture
page / image metadata that this harness does not host).
Both x87 precision settings (PC24 0x007F, PC53 0x027F) are run.

Writes native-decal.json next to this script.
"""
from pathlib import Path
import hashlib, json, struct, sys, itertools

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_layout import Machine, FPCW, SHA, f32bits  # noqa: E402


def setup(m, sizes):
    """Fake texture handles -> sizes; returns the handle list."""
    handles = []
    table = {}
    for s in sizes:
        if s is None:
            handles.append(0)
            continue
        h = m.alloc(0x60)
        table[h] = s
        handles.append(h)

    def width():
        ecx = m.u.reg_read(__import__('unicorn').x86_const.UC_X86_REG_ECX)
        m.ret(table[ecx][0])

    def height():
        ecx = m.u.reg_read(__import__('unicorn').x86_const.UC_X86_REG_ECX)
        m.ret(table[ecx][1])
    m.seams[0x5A6430] = width
    m.seams[0x5A64A0] = height
    return handles


RECTS = [(0.0, 0.0, 1024.0, 768.0), (52.5, 262.5, 1255.0, 849.375), (1957.5, 15.0, 2550.0, 1436.25),
         (100.25, 50.75, 160.5, 90.125), (10.0, 10.0, 12.0, 11.0)]
TEX9 = [
    [(64, 64), (32, 32), (32, 32), (32, 32), (32, 32), (32, 32)],
    [(16, 16), (8, 32), (8, 32), (32, 8), (32, 8), (32, 32)],
    [None, (32, 32), (32, 32), (32, 32), (32, 32), (32, 32)],
    [(64, 64), None, None, None, None, (32, 32)],
    [(15, 33), (7, 9), (5, 3), (11, 13), (17, 19), (21, 23)],
]


def nine_cases():
    rows = []
    for prec, cw in FPCW.items():
        m = Machine(cw)
        for rect, sizes in itertools.product(RECTS, TEX9):
            ve = m.alloc(0x120)
            for i, v in enumerate(rect):
                m.wf(ve + 0x20 + 4 * i, v)
            handles = setup(m, sizes)
            arr = m.alloc(0x20)
            for i, h in enumerate(handles):
                m.w32(arr + 4 * i, h)
            m.w32(ve + 0xc8, arr)
            pieces = []
            for piece in range(9):
                out = m.alloc(16)
                m.call(0x5E6AB0, ve, [out, piece], 8)
                pieces.append([m.r32(out + 4 * i) for i in range(4)])
            rows.append({'precision': prec, 'rect_bits': [f32bits(x) for x in rect],
                         'sizes': sizes, 'pieces_bits': pieces})
    return rows


def directional_cases():
    rows = []
    for prec, cw in FPCW.items():
        m = Machine(cw)
        for rect, ends, horizontal in itertools.product(RECTS, [(16, 32), (3, 40), None, (1, 1)], (1, 0)):
            ve = m.alloc(0x120)
            for i, v in enumerate(rect):
                m.wf(ve + 0x20 + 4 * i, v)
            handles = setup(m, [ends, (32, 32)])
            arr = m.alloc(0x10)
            m.w32(arr, handles[0])
            m.w32(arr + 4, handles[1])
            m.w32(ve + 0xc8, arr)
            m.u.mem_write(ve + 0xe8, bytes([horizontal]))
            pieces = []
            for piece in range(3):
                out = m.alloc(16)
                m.call(0x5E7790, ve, [out, piece], 8)
                pieces.append([m.r32(out + 4 * i) for i in range(4)])
            rows.append({'precision': prec, 'rect_bits': [f32bits(x) for x in rect], 'ends': ends,
                         'horizontal': horizontal, 'pieces_bits': pieces})
    return rows


def fit_cases():
    rows = []
    for prec, cw in FPCW.items():
        m = Machine(cw)
        for rect, tex, tx, ty, stretch in itertools.product(
                RECTS, [(512, 512), (256, 128), (100, 300), (640, 480), (7, 3)], (0, 1), (0, 1), (0, 1)):
            ve = m.alloc(0x120)
            for i, v in enumerate(rect):
                m.wf(ve + 0x20 + 4 * i, v)
            handles = setup(m, [tex])
            m.w32(ve + 0xc8, handles[0])
            m.w32(ve + 0x88, (0x100 if tx else 0) | (0x200 if ty else 0))
            m.u.mem_write(ve + 0x112, bytes([stretch]))
            out = m.alloc(16)
            m.call(0x5E7000, ve, [out], 4)
            rows.append({'precision': prec, 'rect_bits': [f32bits(x) for x in rect], 'tex': tex,
                         'tiled_x': tx, 'tiled_y': ty, 'stretch': stretch,
                         'uv_bits': [m.r32(out + 4 * i) for i in range(4)]})
    return rows


def main():
    report = {'schema': 1, 'original_sha256': SHA, 'scope': __doc__,
              'nine_slice': nine_cases(), 'directional': directional_cases(), 'fit': fit_cases()}
    out = HERE / 'native-decal.json'
    out.write_text(json.dumps(report, separators=(',', ':')) + '\n')
    print(json.dumps({k: len(v) for k, v in report.items() if isinstance(v, list)}),
          hashlib.sha256(out.read_bytes()).hexdigest())


if __name__ == '__main__':
    main()
