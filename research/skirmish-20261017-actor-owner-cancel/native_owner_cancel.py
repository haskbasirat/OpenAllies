"""Actual4D8D65 owner branch, native cancellation RTTI and4FB410 catch body.

This is an instruction-slice oracle, NOT a complete Load/SEH/destructor run.
The owner generation/table/flags are explicit inputs. CString401B00 and
4F5490 destruction are named recorded leaves;5A0910 entry is the throw cutoff.
Native throw/catch metadata links the cutoff to the directly executed handler.
Both accepted PEs/all3PC execute independently. Existing fixtures are replayed
read-only unless --create is explicitly supplied; no original assets written.
"""
from pathlib import Path
import argparse, hashlib, json, struct, sys
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
from native_machine import Machine, original, modified, STACK, STOP
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_EBX, UC_X86_REG_EDI, UC_X86_REG_ESI, UC_X86_REG_EBP, UC_X86_REG_ESP, UC_X86_REG_EIP
sha = lambda b: hashlib.sha256(b).hexdigest()

def machine(image, precision):
    m = Machine(precision)
    old = (original.image_size + 4095) & ~4095
    new = (image.image_size + 4095) & ~4095
    if new > old: m.u.mem_map(image.base + old, new - old)
    for _, rva, size, offset in image.sections:
        m.u.mem_write(image.base + rva, image.b[offset:offset + size])
    return m

def branch(image, precision, case):
    m = machine(image, precision)
    obj, manager = m.alloc(0x200), m.alloc(0x60020)
    owner_pointer = m.alloc(0x40) if case['registered'] else 0
    handle = case['owner']; slot = handle & 0xffff
    m.w(obj + 8, case['flags']); m.w(obj + 0xe8, 0xa5a5a5a5)
    m.w(0xb364c0, manager)
    m.u.mem_write(manager + 4 + slot * 2, struct.pack('<H', case['generation']))
    m.w(manager + 0x20004 + slot * 4, owner_pointer)
    m.w(STACK + 0x2c, handle)
    m.u.reg_write(UC_X86_REG_ESP, STACK)
    m.u.reg_write(UC_X86_REG_ESI, obj)
    m.u.reg_write(UC_X86_REG_EBX, 0)
    m.u.reg_write(UC_X86_REG_EDI, 0x12345678)
    events = []; terminal = ['continued']
    def string(_):
        assert m.arg(1) == 0xa282f8
        events.append('CString401B00'); m.w(m.ecx(), 0x13579bdf); m.ret(pop=4)
    def cancel(_):
        assert m.arg(1) == 0x12345678
        events.append('ThrowCancel5A0910'); terminal[0] = 'load_cancel'; m.u.emu_stop()
    m.seam(0x401b00, string); m.observe(0x5a0910, cancel)
    m.u.emu_start(0x4d8d65, 0x4d8dc8, count=200)
    assert m.u.reg_read(UC_X86_REG_EIP) == (0x5a0910 if terminal[0] == 'load_cancel' else 0x4d8dc8)
    actual_pointer = m.r(obj + 0xe8)
    expected_bound = bool(case['flags'] & (1 << 23)) and case['registered'] and case['generation'] == handle >> 16
    assert bool(actual_pointer) == expected_bound
    assert (terminal[0] == 'load_cancel') == (bool(case['flags'] & (1 << 23)) and handle != 0 and not expected_bound)
    return dict(terminal=terminal[0], owner_present=bool(actual_pointer), events=events), m.visited

def handler(image, precision):
    m = machine(image, precision)
    frame, slot, obj, world = m.alloc(0x80) + 0x40, m.alloc(4), m.alloc(0x200), m.alloc(0x400)
    m.w(slot, obj); m.w(frame + 0xc, slot); m.w(frame - 0x20, world); m.w(STACK, STOP)
    m.u.reg_write(UC_X86_REG_ESP, STACK); m.u.reg_write(UC_X86_REG_EBP, frame)
    events = []
    def destroy(_):
        assert m.ecx() == world and m.arg(1) == obj
        assert m.r(slot) == obj
        events.append('destroy4F5490'); m.ret(pop=4)
    m.seam(0x4f5490, destroy)
    m.u.emu_start(0x4fb410, STOP, count=80)
    assert m.r(slot) == 0 and m.u.reg_read(UC_X86_REG_EAX) == 0x4fb4b1
    return dict(events=events, output_null=True, continuation=m.u.reg_read(UC_X86_REG_EAX)), m.visited

def metadata(image):
    word = lambda p: struct.unpack('<I', image.range(p, 4))[0]
    assert word(0xaee03c + 12) == 0xaee024
    assert word(0xaee024 + 4) == 0xaee008
    assert word(0xaee008 + 4) == 0xb281a4
    assert image.range(0xb281a4 + 8, 32).split(b'\0')[0] == b'.?AVTGC_LoadCancelException@@'
    assert [word(0xae6f98 + o) for o in [12,16]] == [1,0xae6f84]
    assert [word(0xae6f84 + o) for o in range(0,20,4)] == [13,15,17,2,0xae6f64]
    assert [word(0xae6f64 + o) for o in range(0,16,4)] == [8,0xb281a4,0,0x4fb410]
    return [dict(address=hex(a), bytes=image.range(a,n).hex()) for a,n in [(0xaee03c,16),(0xaee024,24),(0xaee008,28),(0xb281a4,40),(0xae6f98,20),(0xae6f84,20),(0xae6f64,32)]]

def main():
    options = argparse.ArgumentParser(); options.add_argument('--create', action='store_true'); args = options.parse_args()
    cases = [dict(name=name,flags=flags,owner=owner,generation=gen,registered=registered) for name,flags,owner,gen,registered in [
        ('unownable_poison_owner',0,0x90002,8,False),
        ('ownable_null',1<<23,0,0,False),
        ('bound',1<<23,0x90002,9,True),
        ('removed_same_generation',1<<23,1677,0,False),
        ('stale_generation',1<<23,0x90002,8,True),
        ('missing_slot',1<<23,0x90002,9,False),
        ('high_generation',1<<23,0xffff0002,65535,True),
        ('nonnull_handle_zero_slot',1<<23,0x80000000,0,False),
        ('other_flags',0xff7fffff,0x90002,8,False)]]
    rows=[];pins={};meta=[]
    for image_name,image in [('original',original),('v14',modified)]:
        meta.append(dict(image=image_name,pins=metadata(image)))
        for precision in ['Single24','Double53','Extended64']:
            for case in cases:
                result,visited=branch(image,precision,case)
                pins.update(visited); rows.append(dict(image=image_name,precision=precision,case=case,native=result))
            result,visited=handler(image,precision)
            pins.update(visited); rows.append(dict(image=image_name,precision=precision,handler=result))
    pinrows=[]
    for a,b in sorted(pins.items()):
        assert b==original.range(a,len(b))==modified.range(a,len(b)),hex(a)
        pinrows.append(dict(address=hex(a),bytes=b.hex()))
    result=dict(scope=__doc__,source_sha256=sha(original.b),v14_sha256=sha(modified.b),cases=rows,metadata=meta,instructions=pinrows)
    path=HERE/'native-owner-cancel.json'
    if args.create:
        with path.open('x',encoding='utf-8') as f:json.dump(result,f,indent=2);f.write('\n')
    else:assert result==json.loads(path.read_text(encoding='utf-8'))
    print(json.dumps(dict(cases=len(rows),pins=len(pinrows),fixture_sha256=sha(path.read_bytes()),mode='created' if args.create else 'read_only_replay')))
if __name__=='__main__':main()
