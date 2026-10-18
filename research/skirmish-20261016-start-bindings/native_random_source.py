"""Original parser and RandomObject49F06C..49F132 / RandomActor4688D4..468995
contribution windows. Registers/frame slots are the documented enclosing loader
inputs, not substituted child/weight results. Actual GetString/GetFixed,
iterator, type check and4B6BA0 execute. Canonical lookup is an explicit seam.
"""
from pathlib import Path
import json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-layouts'))
from layout_machine import Machine,P,V,H,native_windows
from unicorn.x86_const import *
TEMPLATE='''[Template Thing]
{ string IDS
 string category = ACTOR
 [list RandomObject]
 { string random_object_ids
 fixed weight = 1
 }
 [list RandomActor]
 { string random_actor_ids
 fixed weight = 1
 }
}
'''
class RandomMachine(Machine):
    def __init__(self,missing=False,wrong=False):
        self.targets={};super().__init__();self.owners={}
        for name,kind in [('random_object',18),('random_actor',19)]:
            target=self.alloc(0x700);self.u.mem_write(target,b'\0'*0x700)
            cat=self.alloc(0x20);self.w(cat+0x18,1 if wrong else kind);self.w(target+0x18,cat)
            if not missing:self.targets[name]=target
    def hook(self,u,pc,size,user):
        if pc==0x427bd0:
            name=self.raw_wstr(self.arg());target=self.targets.get(name.lower(),0)
            self.recorded.append({'lookup':name,'available':bool(target)});self.ret(target,pop=4);return
        if pc==0x405d60:self.w(self.arg(),self.cstring('<diagnostic>'));self.ret();return
        super().hook(u,pc,size,user)
def run(source,missing=False,wrong=False):
    m=RandomMachine(missing,wrong);m.parse_file('template.tgi',TEMPLATE.encode());parsed=m.parse_file('objects.tgi',source.encode())
    for index,node in enumerate(m.children(parsed['root'])):
        owner=m.alloc(0x600);m.w(owner+8,m.cstring(f'contributor{index}'));m.owners[owner]=f'contributor{index}'
        for entry,end in [(0x49f06c,0x49f132),(0x4688d4,0x468995)]:
            m.u.reg_write(UC_X86_REG_ESP,m.stack);m.u.reg_write(UC_X86_REG_EBP,owner);m.u.reg_write(UC_X86_REG_EDI,node);m.w(m.stack+0x98,node)
            m.u.emu_start(entry,end,count=2000000)
            if m.last_throw:break
            assert m.u.reg_read(UC_X86_REG_EIP)==end
            assert m.u.reg_read(UC_X86_REG_ESP)==m.stack
        if m.last_throw:break
    tables={}
    for name,target in m.targets.items():
        offset=0x170 if name=='random_object' else 0x5c0;rows=[];p=m.r(target+offset)
        while p:rows.append({'definition':m.owners[m.r(p+4)],'weight_bits':m.r(p)});p=m.r(p+8)
        tables[name]={'entries':rows,'total_bits':m.r(target+offset+4)}
    return m,{'source':source,'missing':missing,'wrong':wrong,'tables':tables,'thrown':m.last_throw,'seams':m.recorded}
def node(name,rows):
    return '[Thing template=Thing]\n{ IDS='+name+'\n'+''.join('['+kind+']\n{ '+('random_object_ids'if kind=='RandomObject'else'random_actor_ids')+'='+target+'\n weight='+str(weight)+'\n}\n'for kind,target,weight in rows)+'}\n'
def main():
    cases=[(node('one',[]),False,False),(node('one',[('RandomObject','random_object',1),('RandomObject','random_object',2),('RandomActor','random_actor',3)]),False,False),(node('one',[('RandomActor','random_actor',.25)])+node('two',[('RandomActor','random_actor',.5),('RandomActor','random_actor',-1),('RandomObject','random_object',0)]),False,False),(node('one',[('RandomActor','random_actor',1)]),True,False),(node('one',[('RandomObject','random_object',1)]),False,True)]
    rows=[];visited={}
    for c in cases:
        m,row=run(*c);rows.append(row);visited.update(m.visited);print(row['tables'],bool(row['thrown']),flush=True)
    (HERE/'native-random-source.json').write_text(json.dumps({'schema':1,'scope':__doc__,'original_sha256':H(P.b),'v14_sha256':H(V.b),'template':TEMPLATE,'cases':rows,'native_windows':native_windows(visited)},indent=2)+'\n')
if __name__=='__main__':main()
