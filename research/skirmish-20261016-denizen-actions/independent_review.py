"""Read-only independent Denizen569C70 instruction and native replay audit.

Imports the author's isolated original-image setup, audits its lack of gameplay
seams, then compares re-executed native results against pinned fixtures and an
independently derived instruction predicate. Writes only independent-review
artifacts. No production/source fixture or user game state is modified.
"""
from pathlib import Path
import copy,json,itertools,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(HERE))
from native_denizen_actions import Denizen,A,MODES,ORIGINAL,ACCEPTED,sha
from unicorn import UC_HOOK_MEM_WRITE
from capstone import Cs,CS_ARCH_X86,CS_MODE_32

def independently_expected(c):
    mask=c['actions'];trace=[]
    def write(bit,on):
        nonlocal mask
        mask=(mask|(1<<bit)) if on else (mask&~(1<<bit))
        trace.append(['write',mask])
    if c['state']&(1<<30):
        for bit in (23,24,25,26,27,28):write(bit,False)
    else:
        trace.append(['call','0x4ced40'])
        parent=c['settlement']
        if not parent:
            trace.append(['call','0x4ced60'])
            parent=c['structure'] and c['parent']
            if not parent and c['wall']:
                trace.append(['call','0x58cb30'])
                if c['wall_target']:
                    trace.append(['call','0x58cb30']);parent=True
        if not parent:write(23,False);write(24,False)
        elif c['flags38']&8:
            write(23,c['byte20']==0);write(24,c['byte20']!=0)
        if c['flags38']&0x80:
            write(27,c['byte21']==0);write(28,c['byte21']!=0)
        write(25,bool(c['flags38']&c['flags3c']&0x10))
        write(26,bool(c['flags38']&c['flags3c']&0x40))
    return {'actions':mask,'trace':trace}

def extra_inputs():
    base={'name':'independent','state':0,'actions':0xa5f0b68fffffffff,
        'settlement':True,'structure':False,'parent':False,'wall':False,'wall_target':False,
        'byte20':0,'byte21':0,'flags38':0xffffffd8,'flags3c':0x12345650}
    # All256 raw byte values, including128..255: only zero is false.
    for byte in range(256):
        c=copy.deepcopy(base);c.update(name=f'all_bytes_{byte}',byte20=byte,byte21=255-byte);yield c
    # Every possible component/reference presence combination, including
    # nonnull irrelevant fields behind absent components; razing must skip all.
    for flags in itertools.product((False,True),repeat=5):
        for razing in (False,True):
            c=copy.deepcopy(base);c.update(zip(('settlement','structure','parent','wall','wall_target'),flags))
            c.update(name=f'presence_{flags}_{razing}',state=(1<<30)if razing else(1<<62),byte20=7,byte21=128);yield c

def raw_stores(c):
    m=Denizen(0x27f,c,'original');out=[]
    def hook(u,access,addr,size,value,user):
        if addr in (A+0xf0,A+0xf4):out.append({'offset':addr-A,'value':value})
    m.uc.hook_add(UC_HOOK_MEM_WRITE,hook)
    result=m.run()
    assert all(s['value']==c['actions']>>32 for s in out if s['offset']==0xf4)
    return {'input':c,'raw_stores':out,'result':result}

def main():
    replayed={};pins={}
    for name in ('native-denizen-actions.json','native-saved-denizen-actions.json'):
        path=HERE/name;before=path.read_bytes();pins[name]=sha(before)
        data=json.loads(before)
        for row in data['cases']:
            cw=dict(MODES)[row['precision']]
            m=Denizen(cw,row['input'],row['image']);assert m.seams=={}
            actual=m.run();assert actual==row['output'],row
            assert actual==independently_expected(row['input']),row
        assert path.read_bytes()==before
        replayed[name]=len(data['cases'])
    extras=[]
    for image in ('original','v14'):
        for pc,cw in MODES:
            for c in extra_inputs():
                m=Denizen(cw,c,image);assert m.seams=={}
                result=m.run();assert result==independently_expected(c),(image,pc,c,result)
                extras.append({'image':image,'precision':pc,'input':c,'native':result})
    probes=[]
    c=next(extra_inputs());c.update(name='raw_razing',state=1<<30)
    probes.append(raw_stores(c))
    c=next(extra_inputs());c.update(name='raw_no_parent',settlement=False,flags38=0)
    probes.append(raw_stores(c))
    ranges=[(0x569c70,0x569e3f),(0x4ced40,0x4ced55),(0x4ced60,0x4ced97),(0x58cb30,0x58cb34),(0x517fa0,0x518004)]
    md=Cs(CS_ARCH_X86,CS_MODE_32);lines=[]
    for begin,end in ranges:
        assert ORIGINAL.at(begin,end-begin)==ACCEPTED.at(begin,end-begin)
        lines.extend(f'{i.address:08X} {i.mnemonic} {i.op_str}' for i in md.disasm(ORIGINAL.at(begin,end-begin),begin))
    (HERE/'independent-instructions.txt').write_text('\n'.join(lines)+'\n')
    result={'schema':1,'independent_reviewer':'organization_morale_source','scope':__doc__,
            'original_sha256':sha(ORIGINAL.data),'v14_sha256':sha(ACCEPTED.data),'author_fixture_sha256':pins,
            'replayed_cases':replayed,'extra_cases':extras,'raw_store_probes':probes,'generator_sha256':sha(Path(__file__).read_bytes())}
    path=HERE/'independent-review-native.json';path.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({'replayed':replayed,'extra_cases':len(extras),'raw_store_probes':len(probes),'sha256':sha(path.read_bytes())}))
if __name__=='__main__':main()
