"""Original/v14 CV input getter and4618B0 argument-assembly proof.

Actual immutable source inputs populate the already-proven native layouts.
The4618B0 prefix stops BEFORE Children30C;45E090 is an explicit recorded leaf
returning laboratory values, not a claim to compute this game's combat values.
Native getter bodies, property reduction, argument gating and iteration execute.
"""
from pathlib import Path
import sys,json,struct
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
from native_machine import Machine,original,modified,PRECISIONS,STACK,STOP,sha
from unicorn.x86_const import *

def machine(image,pc):
    m=Machine(pc)
    old=(original.image_size+0xfff)&~0xfff;new=(image.image_size+0xfff)&~0xfff
    if new>old:m.u.mem_map(original.base+old,new-old)
    for _,rva,size,offset in image.sections:m.u.mem_write(image.base+rva,image.b[offset:offset+size])
    return m

def local(m,row):
    definition=m.alloc(0x80);value=m.alloc(0x70)
    m.w(value,definition);m.w(value+4,row['value_04']);m.w(value+0x18,row['recharge_18'])
    for offset,key in [(0x24,'range_24'),(0x38,'area_38'),(0x54,'flags_54'),(0x64,'power_64')]:m.w(definition+offset,row[key])
    m.w(definition+0x3c,m.alloc(4) if row['projectile_3c'] else 0)
    if row['first_entry_10'] is not None:
        clip=m.alloc(0x20);m.w(clip+0x10,row['first_entry_10'])
        clips=m.array([clip]);vector=m.alloc(0xc);m.w(vector,clips);m.w(vector+4,1)
        m.w(value+0x3c,m.array([vector]));m.w(value+0x40,1)
    return value,definition

def main():
    sourcepath=Path(__file__).with_name('actual-source-inputs.json')
    source=json.loads(sourcepath.read_text());rows=[];prefixes=[];pins={}
    for label,image in [('original',original),('v14',modified)]:
        for row in source['rows']:
            pc=row['precision']
            for index,ability in enumerate(row['abilities']):
                m=machine(image,pc);loc,definition=local(m,ability);values={}
                for key,entry,owner,args in [('first_entry_10',0x4be680,loc,[]),('range_24',0x4be910,definition,[0]),('power_64',0x4be3f0,definition,[0]),('recharge_18',0x4be4b0,loc,[0])]:
                    m.call(entry,owner,args);values[key]=m.st0()['f32_bits']
                    assert values[key]==(ability[key] if ability[key] is not None else 0)
                m.verify_v14();pins.update(m.visited)
                rows.append({'image':label,'id':row['id'],'precision':pc,'ability':index,'output':values})
            m=machine(image,pc);actor=m.alloc(0x600);m.w(actor+0x174,row['flags_174'])
            if row['body'] is not None:m.w(actor+0x288,row['body'][0]);m.w(actor+0x284,row['body'][1])
            if row['speed_228'] is not None:m.w(actor+0x228,row['speed_228'])
            if row['bias_210'] is not None:m.w(actor+0x210,row['bias_210'])
            head=0
            for _,bits in reversed(row['properties_1d4']):
                prop=m.alloc(0x30);m.w(prop+0x28,bits);node=m.alloc(8);m.w(node,prop);m.w(node+4,head);head=node
            m.w(actor+0x1d4,head)
            locals=[local(m,a)[0] for a in row['abilities']]
            m.w(actor+0x1ec,m.array(locals));m.w(actor+0x1f0,len(locals))
            manager=m.alloc(0x100);vt=m.alloc(0x20);m.w(manager+0x4c,vt)
            m.w(vt+0x14,m.stub(lambda m:m.ret(0)));m.w(0xb39f58,manager)
            calls=[]
            def call(m):
                args=[m.arg(i) for i in range(1,8)]
                calls.append({'ability':locals.index(args[0]),'defense':args[1],'health':args[2],'speed':args[3],'property_sum':args[4],'bias':args[5],'modifier':args[6]})
                m.ret_st0_f32(struct.unpack('<I',struct.pack('<f',float(len(calls))))[0])
            m.seam(0x45e090,call)
            m.w(STACK,STOP);m.u.reg_write(UC_X86_REG_ESP,STACK);m.u.reg_write(UC_X86_REG_ECX,actor)
            m.u.reg_write(UC_X86_REG_FPCW,PRECISIONS[pc]);m.u.reg_write(UC_X86_REG_FPSW,0)
            m.u.emu_start(0x4618b0,0x461ba0,count=50000)
            assert m.u.reg_read(UC_X86_REG_EIP)==0x461ba0
            assert not m.null_accesses
            assert len(calls)==(len(locals) if row['flags_174']&(1<<5) else 0)
            m.verify_v14();pins.update(m.visited)
            prefixes.append({'image':label,'id':row['id'],'precision':pc,'calls':calls})
    ranges=[]
    for a,z,label in [(0x4654b6,0x4654c8,'Performer.cv_bias->ActorData210'),(0x4bf3eb,0x4bf3fa,'AbilityDefinition.range->24'),(0x4bf4f0,0x4bf507,'AbilityDefinition.area->38'),(0x4bf990,0x4bfa30,'Ability.reload_time->local4'),(0x4c4cf0,0x4c4e10,'Property reference prepend'),(0x4c43d4,0x4c43e3,'Property.performer_cv_bias->28')]:
        raw=original.range(a,z-a);assert raw==modified.range(a,z-a);ranges.append({'name':label,'start':hex(a),'end':hex(z),'sha256':sha(raw)})
    document={'scope':__doc__,'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'source_sha256':sha(sourcepath.read_bytes()),'producer_sha256':sha(Path(__file__).read_bytes()),'getters':rows,'argument_prefixes':prefixes,'pins':[{'address':hex(a),'bytes':b.hex()} for a,b in sorted(pins.items())],'mapping_ranges':ranges}
    out=Path(__file__).with_name('native-source-inputs.json');out.write_text(json.dumps(document,indent=2)+'\n')
    print(json.dumps({'getters':len(rows),'argument_prefixes':len(prefixes),'pins':len(pins),'sha256':sha(out.read_bytes())}))
if __name__=='__main__':main()
