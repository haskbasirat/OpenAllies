"""Independent read-only Team Init(NULL) oracle replay and instruction audit."""
from pathlib import Path
import json,struct,sys,copy
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(HERE))
import native_team_init as native
from capstone import Cs,CS_ARCH_X86,CS_MODE_32

def expected(c):
    flags=c['flags'];position=list(c['position']);events=[];stores=[]
    def result(outcome='ok',value=None):
        return {'outcome':outcome,'result':value,'flags':flags,'position':position,
                'name':c['name'],'events':events,'height_stores':stores}
    if flags&(1<<16):
        events.append(['snap',c['tile']])
        for i,b in enumerate(c['snapped']):
            position[i]=b
            if i==2:stores.append(b)
        if c.get('failure')=='snap':return result('snap')
    events.append(['placement',position.copy()])
    if c.get('failure')=='placement':return result('placement')
    if c['placement_return']&255==0:return result(value=False)
    events.append(['index']);flags=c.get('index_flags',flags)
    if c.get('failure')=='index':return result('index')
    if flags&(1<<21) or not flags&(1<<12):
        events.append(['height',position.copy(),0x123 if flags&(1<<21) else 0x122])
        if c.get('failure')=='height':return result('height')
        value=struct.unpack('<d',struct.pack('<Q',c['height_f64_bits']))[0]
        word=struct.unpack('<I',struct.pack('<f',value))[0]
        position[2]=word;stores.append(word)
    events.append(['log',c['name']])
    if c.get('failure')=='log':return result('log')
    return result(value=True)

def main():
    files=['crates/aa-mapworld/src/team_init.rs','crates/aa-mapworld/tests/native_team_init.rs',
           'research/skirmish-20261016-team-init/native-team-init.json','research/skirmish-20261016-team-init/native_team_init.py']
    before={p:(ROOT/p).read_bytes() for p in files}
    fixture=json.loads(before[files[2]])
    for row in fixture['cases']:
        actual,_=native.run(row['input'],row['image'],row['precision'])
        assert actual==row['output']==expected(row['input']),row
    extras=[]
    base=next(native.cases())
    for name in [[],[0x41,0,0x42],[0x03a9,0x4e2d],[0xd800,0xdc00,0xdfff]]:
        for flags in [(1<<16)|(1<<12),(1<<16)|(1<<21),1<<12]:
            for placement in [0x100,0x101]:
                c=copy.deepcopy(base);c.update(name=name,flags=flags,placement_return=placement,
                    label='independent_name_flags',index_flags=1<<21,height_f64_bits=0x3ff0000010000000)
                for image in ['original','v14']:
                    for pc in ['Single24','Double53','Extended64']:
                        actual,_=native.run(c,image,pc);assert actual==expected(c),(c,actual)
                        extras.append({'image':image,'precision':pc,'input':c,'output':actual})
    # Actual vtable constants and RET (not RET8) establish the stack nuance.
    windows={}
    ranges={'Team_virtual_C':(0xa29780+0xc,4),'Team_virtual_E8':(0xa29780+0xe8,4),
            'grid_context_getter':(0x4e3e90,6),'snap_callsite':(0x4efa55,0x19)}
    for name,(addr,size) in ranges.items():
        data=native.original.range(addr,size);assert data==native.modified.range(addr,size)
        windows[name]={'address':hex(addr),'bytes':data.hex(),'sha256':native.sha(data)}
    assert struct.unpack('<I',bytes.fromhex(windows['Team_virtual_C']['bytes']))[0]==0x4e3e90
    assert struct.unpack('<I',bytes.fromhex(windows['Team_virtual_E8']['bytes']))[0]==0x4e68c0
    assert bytes.fromhex(windows['grid_context_getter']['bytes'])==bytes.fromhex('b8c4a8b300c3')
    lines=[];md=Cs(CS_ARCH_X86,CS_MODE_32)
    for a,n in [(0x4e68c0,0xac),(0x4efa40,0xcf),(0x4e3e90,6),(0x46ee00,0x80)]:
        data=native.original.range(a,n);assert data==native.modified.range(a,n)
        lines.extend(f'{i.address:08X} {i.mnemonic} {i.op_str}' for i in md.disasm(data,a))
    (HERE/'independent-instructions.txt').write_text('\n'.join(lines)+'\n')
    assert all((ROOT/p).read_bytes()==data for p,data in before.items())
    report={'schema':1,'reviewer':'organization_morale_source','replayed_rows':len(fixture['cases']),
            'extra_rows':extras,'native_windows':windows,
            'protected_file_sha256':{p:native.sha(data) for p,data in before.items()},
            'protected_bytes_unchanged':True,'method':__doc__}
    path=HERE/'independent-native-replay.json';path.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'replayed':len(fixture['cases']),'extra':len(extras),'protected_bytes_unchanged':True,'sha256':native.sha(path.read_bytes())}))
if __name__=='__main__':main()
