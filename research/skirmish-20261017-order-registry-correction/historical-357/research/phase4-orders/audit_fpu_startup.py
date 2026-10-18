"""Read-only static precision evidence; deliberately not a simulation CW oracle."""
from pathlib import Path
import csv
import hashlib
import json
import struct
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
GAME = ROOT.parent
sys.path.insert(0, str(ROOT / 'research/phase3-orders'))
from original_orders import PE0, PE1
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

sha = lambda b: hashlib.sha256(b).hexdigest()
functions_path = GAME / 'reconstruction/research/native-aa/functions.tsv'
functions = {int(r['address'], 16): r for r in csv.DictReader(
    functions_path.open(encoding='utf-8'), delimiter='\t')}
addresses = [0x96f484, 0x96ec76, 0x96bdb7, 0x972c6b,
             0x96f6ea, 0x96f778, 0x96f7aa, 0x7f1965, 0x7f1a39,
             0x710ac0, 0x891890, 0x88b1a0]
md = Cs(CS_ARCH_X86, CS_MODE_32)
windows = []
assembly = []
for va in addresses:
    size = int(functions[va]['bodyBytes'])
    a = PE0.range(va, size)
    b = PE1.range(va, size)
    assert a == b, hex(va)
    windows.append({'va': hex(va), 'bytes': size, 'sha256': sha(a),
                    'original_equals_v14': True})
    assembly.append('\n' + hex(va) + '\n' + '\n'.join(
        f'{i.address:08x} {i.bytes.hex():20s} {i.mnemonic} {i.op_str}'
        for i in md.disasm(a, va)))
assert struct.unpack('<I', PE0.range(0xb35098, 4))[0] == 0x96bdb7
assert PE0.range(0xb35098, 4) == PE1.range(0xb35098, 4)
assert PE0.range(0x972c6b, 18).hex() == '68000003006800000100e830cbffff5959c3'
assembly_path = HERE / 'fpu-startup-static.asm.txt'
assembly_path.write_text('\n'.join(assembly) + '\n', encoding='utf-8')
report = {
    'schema': 1,
    'status': 'STATIC_STARTUP_AND_RENDERER_FACTS_ONLY',
    'simulation_thread_precision_proven': False,
    'default_precision_recommendation': None,
    'method': 'Read original and v14 images, compare code windows, inspect exact x86 disassembly. No native process or emulator run.',
    'original_sha256': sha(PE0.b),
    'v14_sha256': sha(PE1.b),
    'producer_sha256': sha(Path(__file__).read_bytes()),
    'functions_tsv_sha256': sha(functions_path.read_bytes()),
    'assembly_sha256': sha(assembly_path.read_bytes()),
    'windows': windows,
    'fpmath_pointer': {'va': '0xb35098', 'value': '0x96bdb7',
                       'sha256': sha(PE0.range(0xb35098, 4))},
    'facts': [
        {'addresses': ['0x96f484', '0x96ec76', '0xb35098', '0x96bdb7', '0x972c6b'],
         'finding': 'Executable CRT entry calls __cinit(1); its nonnull __fpmath pointer calls __setdefaultprecision for this nonzero argument. That requests __controlfp(0x10000, 0x30000).'},
        {'addresses': ['0x96f6ea', '0x96f778', '0x96f7aa'],
         'finding': 'Native abstraction precision 0x10000 maps to hardware x87 PC 0x0200 (53 significant bits). __control87 reads current control word, applies the supplied mask, converts back, then FLDCW. This proves CRT startup PC53, not its persistence.'},
        {'addresses': ['0x7f1965', '0x7f1a39'],
         'finding': 'A separate save/restore pair stores __controlfp(0,0) at object+0x2a0, sets PC53, then later restores precision bits from the saved value. This pair is not established as the world/simulation thread policy.'},
        {'addresses': ['0x710c16', '0x710c2c', '0x710c6b', '0x710c72', '0x710cea', '0x710ac0'],
         'finding': 'KKC_Renderer::ChangeResolution builds its creation flags in EBX from zero with optional 0x10, 0x20, 4, and 8. Its normal creation path passes that value to NiDX9Renderer::Create; no 0x40 FPU-preserve request is added on this path.'},
        {'addresses': ['0x88b1a0', '0x89194a', '0x891988', '0x891990', '0x891998'],
         'finding': 'Base D3D behavior flags are one of 0x50,0x40,0x80,0x20. Ni renderer bit0x40 adds D3D bit2 (FPU_PRESERVE), while bit0x20 adds D3D bit4 (MULTITHREADED). The inspected caller does not request the former. These flags are passed to CreateDevice at IDirect3D9 vtable+0x40.'},
    ],
    'official_api_sources': [
        {'url': 'https://learn.microsoft.com/en-us/windows/win32/direct3d9/d3dcreate',
         'finding': 'Without D3DCREATE_FPU_PRESERVE, Microsoft specifies single-precision round-to-nearest D3D behavior; the preserve option retains calling-thread precision.'},
        {'url': 'https://learn.microsoft.com/en-us/windows/win32/dxtecharts/top-issues-for-windows-titles',
         'finding': 'The floating-point control word is thread-specific; Direct3D initialization can alter the calling thread.'},
    ],
    'limits': [
        'Neither CRT PC53 nor D3D creation flags establish the control word of a particular later simulation tick.',
        'No proof here identifies renderer-thread versus simulation-thread ownership, child-thread inheritance, subsequent FLDCW calls, or effects of the shipped dgVoodoo wrapper.',
        'No live thread CW/status word or preserved dump thread-role measurement was performed.',
        'Pending x87 exception-status flags and native asin status-sensitive branches remain separate from precision-control bits.',
        'Keep explicit precision/status inputs in parity APIs until a complete thread/lifecycle proof is available; do not select PC53 merely to avoid a trig discrepancy.',
    ],
}
out = HERE / 'fpu-startup-static.json'
out.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
print(json.dumps({'report': str(out), 'sha256': sha(out.read_bytes()),
                  'original_v14_equal_windows': len(windows),
                  'simulation_thread_precision_proven': False}))
