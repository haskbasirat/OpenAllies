from pathlib import Path
import sys,csv,struct,json,hashlib,bisect
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'));import native_machine as h
from capstone import Cs,CS_ARCH_X86,CS_MODE_32,CS_AC_WRITE
from capstone.x86_const import X86_OP_MEM
cs=Cs(CS_ARCH_X86,CS_MODE_32);cs.detail=True
functions=list(csv.DictReader((ROOT.parent/'reconstruction/research/native-aa/functions.tsv').open(),delimiter='\t'))
functions.sort(key=lambda r:int(r['address'],16));starts=[int(r['address'],16) for r in functions]
refs=[]
for address in [0xb2814c,0xb28150,0xb28154]:
    pattern=struct.pack('<I',address);at=0
    while True:
        at=h.P.b.find(pattern,at)
        if at<0:break
        hit=None
        for _,r,size,file in h.P.sections:
            if file<=at<file+size:hit=h.P.base+r+at-file;break
        assert hit is not None
        function=functions[bisect.bisect_right(starts,hit)-1];start=int(function['address'],16)
        candidates=[i for i in cs.disasm(h.P.range(start,hit-start+16),start) if i.address<=hit<i.address+i.size]
        assert len(candidates)==1,(hex(hit),function)
        i=candidates[0]
        operands=[op for op in i.operands if op.type==X86_OP_MEM and op.mem.disp==address]
        assert len(operands)==1 and not(operands[0].access&CS_AC_WRITE),(hex(i.address),i.mnemonic,i.op_str)
        assert i.mnemonic in ['mov','fld']
        assert h.P.range(i.address,i.size)==h.V.range(i.address,i.size)
        refs.append(dict(global_address=hex(address),address=hex(i.address),bytes=i.bytes.hex(),
                         instruction=i.mnemonic+' '+i.op_str,function=function['address'],access='read'))
        at+=1
assert len(refs)==66
windows={}
for label,a,b in [('sun_ctor',0x4ee0a0,0x4ee1e7),('sun_load',0x4edbd0,0x4edd26),
                  ('updatable_ctor',0x50ab90,0x50aba0),('name_list_manager_ctor_call',0x48ae75,0x48aea0),
                  ('misc_ctor_prefix',0x4f8018,0x4f8072),('misc_ctor_speed',0x4f8125,0x4f813d),
                  ('game_speed_registration',0x507db6,0x507de9),('fixed_variable_ctor',0x401710,0x4017b0)]:
    rows=[]
    for i in cs.disasm(h.P.range(a,b-a),a):
        assert h.P.range(i.address,i.size)==h.V.range(i.address,i.size)
        rows.append(dict(address=hex(i.address),bytes=i.bytes.hex(),instruction=i.mnemonic+' '+i.op_str))
    windows[label]=rows
out=dict(schema=1,scope='Exhaustive exact-address literal references in complete image; no claim ruling out arbitrary indirect aliases or perpetual native immutability',
         producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
         globals_hex=h.P.range(0xb2814c,12).hex(),references=refs,windows=windows)
path=HERE/'static-ctor-and-global-references.json';path.write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps(dict(references=len(refs),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
