"""Original464BD0 Detection block, typed Fixed getters as explicit f32 seams.

No game process or asset is written. NativeFST/FCOMP/FNSTSW/TEST/JP and bit writes
execute. General reload control inputs are isolated probes, not fresh load state.
"""
from pathlib import Path
import hashlib
import json
import struct
import sys
sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT/'research/phase4-rng'))
from original_rng import Machine, original, modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
from capstone import Cs, CS_ARCH_X86, CS_MODE_32


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


class DetectionMachine(Machine):
    def __init__(self, cw):
        super().__init__(cw)
        self.visited = {}; self.present = True; self.getters = []
        self.stub = self.STOP-0x200; self.data = self.STATE+0x800
        for i in range(2):
            self.u.mem_write(self.stub+i*16, b'\xd9\x05'+struct.pack('<I',self.data+i*4)+b'\xc2\x04\x00')
        self.u.hook_add(UC_HOOK_CODE, self.hook)
    def hook(self, uc, address, size, opaque):
        if original.base <= address < original.base+original.image_size:
            self.visited[address] = bytes(uc.mem_read(address,size))
        if address == 0x5b4410:
            esp = uc.reg_read(UC_X86_REG_ESP)
            uc.reg_write(UC_X86_REG_EAX, self.STATE+0x900 if self.present else 0)
            uc.reg_write(UC_X86_REG_EIP, self.r(esp)); uc.reg_write(UC_X86_REG_ESP, esp+12)
        elif address == 0x5b5130:
            parameter = self.r(uc.reg_read(UC_X86_REG_ESP)+4)
            assert parameter == [0xa09718,0xa096e8][len(self.getters)]
            uc.reg_write(UC_X86_REG_EIP, self.stub+len(self.getters)*16)
            self.getters.append(parameter)
    def run(self, present, initial, control, remote):
        self.reset(0); self.present=present; self.getters=[]
        self.w(self.STATE+0x174,initial); self.w(self.STATE+0x218,control)
        self.w(self.data,0x40e00000); self.w(self.data+4,remote)
        self.u.reg_write(UC_X86_REG_EBP,self.STATE)
        self.u.emu_start(0x465011,0x465089,count=1000)
        assert self.u.reg_read(UC_X86_REG_EIP)==0x465089
        assert self.u.reg_read(UC_X86_REG_ESP)==self.STACK
        assert len(self.getters)==(2 if present else 0)
        return {'mask_after':self.r(self.STATE+0x174),'remote_stored_bits':self.r(self.STATE+0x1d0) if present else None,
                'getter_calls':len(self.getters)}


def main():
    values = [0xff800000,0xc1200000,0xbf800000,0x80800000,0x80000001,
              0x80000000,0,1,0x00800000,0x3f800000,0x41200000,0x7f800000,
              0x7fc12345,0xffc12345,0x7fa12345,0xffa12345]
    rows = []; visited = {}
    for pc, cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
        machine = DetectionMachine(cw)
        for initial in [0,0x505065,0xffffffff]:
            for control in values:
                for remote in values:
                    result = machine.run(True,initial,control,remote)
                    rows.append({'precision':pc,'component_present':True,'mask_before':initial,
                                 'control_bits':control,'remote_bits':remote,**result})
            for remote in [0,0x3f800000,0x7fc12345]:
                rows.append({'precision':pc,'component_present':False,'mask_before':initial,
                             'control_bits':0,'remote_bits':remote,**machine.run(False,initial,0,remote)})
        visited.update(machine.visited)
    for address, instruction in visited.items():
        # Getter-entry bytes are pinned too, though their bodies are seams.
        assert original.range(address,len(instruction))==instruction==modified.range(address,len(instruction))
    cs = Cs(CS_ARCH_X86,CS_MODE_32)
    constructor = list(cs.disasm(original.range(0x4624b0,0x8a0),0x4624b0))
    zero_ebx = [x for x in constructor[:80] if x.mnemonic=='xor' and x.op_str=='ebx, ebx']
    assert len(zero_ebx)==1
    indices = [i for i,x in enumerate(constructor) if '0x218' in x.op_str]
    assert len(indices)==1
    nearby = constructor[max(0,indices[0]-12):indices[0]+5]
    asm = [f'{x.address:08x} {x.bytes.hex()} {x.mnemonic} {x.op_str}' for x in constructor[:80]]
    asm += ['--- +218 store ---']
    asm += [f'{x.address:08x} {x.bytes.hex()} {x.mnemonic} {x.op_str}' for x in nearby]
    # Preserve enough local instructions for explicit review of the zero source.
    (HERE/'constructor-218.asm.txt').write_text('\n'.join(asm)+'\n')
    block = list(cs.disasm(original.range(0x465011,0x78),0x465011))
    (HERE/'detection-block.asm.txt').write_text('\n'.join(f'{x.address:08x} {x.bytes.hex()} {x.mnemonic} {x.op_str}' for x in block)+'\n')
    spans = [(0x465011,0x465089),(0x4624b0,0x462d50)]
    pins = []
    for a,b in spans:
        raw = original.range(a,b-a); assert raw==modified.range(a,b-a)
        pins.append({'start':hex(a),'end':hex(b),'sha256':sha(raw)})
    doc = {'schema':1,'producer_sha256':sha(Path(__file__).read_bytes()),
           'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'cases':rows,'ranges':pins,
           'instructions':[{'va':hex(a),'bytes':b.hex()} for a,b in sorted(visited.items())],
           'scope':'Original Detection block executes at allPC with typed f32 getters. Full464BD0 is not executed. Constructor+218 zero store is statically pinned/inspected; general nonzero control values are isolated reload-branch probes.',
           'nan_qualification':'NaN rows establish unordered mask decisions only. No signaling-NaN payload quieting or x87 status parity is claimed because Unicorn has known differences.',
           'page_zero':'unmapped by the base machine, so null data reads fault'}
    output=HERE/'native-detection-mask.json'; output.write_text(json.dumps(doc,indent=2)+'\n')
    print(json.dumps({'cases':len(rows),'sha256':sha(output.read_bytes()),'constructor_store':asm}))


if __name__=='__main__':main()
