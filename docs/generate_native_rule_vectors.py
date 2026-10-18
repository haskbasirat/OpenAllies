"""Execute unchanged original x86 fragments in isolated Unicorn fixtures.

No live game process is launched, attached or modified. Output is test evidence
for the new Rust crate, not proof of a complete native simulation.
"""
from pathlib import Path
import hashlib, json, struct, sys

ROOT = Path(__file__).resolve().parents[1]
GAME = ROOT.parent
sys.path.insert(0, str(GAME/'reconstruction/tools/python-packages'))
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_ECX, UC_X86_REG_ESI, UC_X86_REG_ESP, UC_X86_REG_FPCW

EXPECTED = '5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26'
IMAGE = (GAME/'AA.exe').read_bytes()
assert hashlib.sha256(IMAGE).hexdigest() == EXPECTED
RESTORED = (GAME/'reconstruction/native-game/runtime-expanded-v14/AA.exe').read_bytes()
RANGES = {'rng':(0x85d840,0x85d86c),'clock_fields':(0x4f9a07,0x4f9a3b),
          'provider_function':(0x4e3940,0x4e39cf),'upkeep_helper_prefix':(0x4e1f60,0x4e2050)}
for start,end in RANGES.values():
    assert IMAGE[start-0x400000:end-0x400000] == RESTORED[start-0x400000:end-0x400000]


def machine():
    uc=Uc(UC_ARCH_X86,UC_MODE_32)
    uc.mem_map(0x400000,0xc00000);uc.mem_write(0x400000,IMAGE)
    uc.mem_map(0x2000000,0x10000);uc.mem_map(0x3000000,0x10000)
    uc.reg_write(UC_X86_REG_FPCW,0x037f)
    return uc


def u32(uc,addr,value):uc.mem_write(addr,struct.pack('<I',value&0xffffffff))
def read32(uc,addr):return struct.unpack('<I',uc.mem_read(addr,4))[0]
def bits(value):return struct.unpack('<I',struct.pack('<f',value))[0]


def rng_vectors():
    uc=machine();context=uc.context_save();rows=[]
    seeds=[0,1,2,0xffff,0x10000,0x7fffffff,0x80000000,0xffffffff,0x12345678,0xffffcfc6]
    state=0xc0decafe
    for _ in range(64):
        seeds.append(state);state=(state*1664525+1013904223)&0xffffffff
    for seed in seeds:
        uc.context_restore(context);u32(uc,0x2000000,seed)
        u32(uc,0x3008000,0x300f000);u32(uc,0x3008004,0x2000000)
        uc.reg_write(UC_X86_REG_ESP,0x3008000)
        uc.emu_start(0x85d840,0x300f000,count=100)
        assert uc.reg_read(UC_X86_REG_ESP)==0x3008004
        rows.append({'seed':seed,'state':read32(uc,0x2000000),'output':uc.reg_read(UC_X86_REG_EAX)})
    return rows


def clock_vectors():
    uc=machine();context=uc.context_save();rows=[]
    ticks=[0,1,15,16,65535,16777215,16777216,16777217,0x7ffffffe,0x7fffffff,0xfffffffe,0xffffffff]
    steps=[bits(0.0625),bits(0.1),bits(1/30),bits(1.0),1,0x00800000,0x3f123457]
    for step in steps:
        for tick in ticks:
            uc.context_restore(context)
            u32(uc,0xb36534,0x2000000);u32(uc,0x20000dc,tick)
            u32(uc,0x20000e0,0);u32(uc,0x20000e4,bits(17.25));u32(uc,0x20000e8,step)
            uc.reg_write(UC_X86_REG_ESI,0x2000000);uc.reg_write(UC_X86_REG_ESP,0x3008000)
            uc.emu_start(0x4f9a07,0x4f9a3b,count=100)
            rows.append({'tick_before':tick,'step_bits':step,'old_end_bits':bits(17.25),
                         'tick_after':read32(uc,0x20000dc),'begin_bits':read32(uc,0x20000e0),'end_bits':read32(uc,0x20000e4)})
    return rows


def provider_vectors():
    uc=machine();kingdom=0x2000000;raw=0x2001000;effective=0x2002000;consumed=0x2003000
    registry=0x2004000;table=0x2005000;definition=0x2006000;svar=0x2007000;limit=0x2008000
    for addr,value in [(kingdom+0x1ac,raw),(kingdom+0x1a0,effective),(kingdom+0x1b8,consumed),
        (kingdom+0x1a4,1),(0xb36530,registry),(registry+0x34,table),(table,definition),
        (definition+0x18,1),(0xb39ab0,svar),(svar+0x1b4,limit)]:u32(uc,addr,value)
    context=uc.context_save();rows=[]
    historical=json.loads((GAME/'reconstruction/native-game/mods/expanded/caps-native-evidence.json').read_text())
    cases=[(bits(r['old_raw']),bits(r['delta']),r['maximum'],r['limited']) for r in historical['machine_code_cases']]
    cases += [(a,b,m,l) for a,b in [(0x80000000,0x80000000),(0x80000000,0),(1,1),
               (bits(0.1),bits(0.2)),(bits(16777216),bits(1)),(bits(21),bits(-3)),(bits(-1),bits(-2))]
              for m,l in [(20,True),(100,True),(100,False)]]
    for old,delta,maximum,limited in cases:
        uc.context_restore(context);u32(uc,limit,maximum)
        uc.mem_write(definition+0x2c,bytes([limited]));u32(uc,raw,old);u32(uc,effective,old);u32(uc,consumed,0)
        stack=0x3008000;u32(uc,stack,0x300f000);u32(uc,stack+4,0);u32(uc,stack+8,delta)
        uc.reg_write(UC_X86_REG_ECX,kingdom);uc.reg_write(UC_X86_REG_ESP,stack)
        uc.emu_start(0x4e3940,0x300f000,count=10000)
        assert uc.reg_read(UC_X86_REG_ESP)==stack+12
        rows.append({'old_raw_bits':old,'delta_bits':delta,'maximum':maximum,'limited':limited,
                     'raw_bits':read32(uc,raw),'effective_bits':read32(uc,effective)})
    return rows


def main():
    result={'schema':1,'original_executable_sha256':EXPECTED,
        'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'method':'Unchanged original x86 in isolated Unicorn; no decision hooks, live process or native game launch.',
        'fpu_control_word':'0x037f (round nearest, extended precision; no claim of complete engine FPU lifecycle)',
        'ranges':{name:{'start_va':hex(start),'end_va_exclusive':hex(end),
                  'sha256':hashlib.sha256(IMAGE[start-0x400000:end-0x400000]).hexdigest(),
                  'unchanged_in_v14_native':True} for name,(start,end) in RANGES.items()},
        'rng':rng_vectors(),'clock':clock_vectors(),'providers':provider_vectors(),
        'limits':['Clock executes only the field-update fragment, not whole GWorld phases.',
                  'Provider fixture contains one upkeep definition and no purchase resources: shortage effects are not tested or ported.',
                  'These arithmetic vectors do not establish complete gameplay, save, wire-protocol or actor-order parity.']}
    target=ROOT/'crates/aa-simulation/tests/fixtures/native-rule-vectors.json';target.parent.mkdir(parents=True,exist_ok=True)
    target.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({'path':str(target),'rng_cases':len(result['rng']),'clock_cases':len(result['clock']),
                      'provider_cases':len(result['providers']),'sha256':hashlib.sha256(target.read_bytes()).hexdigest()},indent=2))


if __name__=='__main__':main()
