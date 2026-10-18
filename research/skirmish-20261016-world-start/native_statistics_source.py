"""Original parser, statistic enum/manager constructor, and Statistic LoadData.

Resource lookup is a recorded fixture seam with canonical resource index.
Scoring definitions, factors, stat-name lookup, registry append order, and
linked-list construction execute original machine code. String localization
uses the existing explicit fixture string-table seam (not gameplay output).
"""
from pathlib import Path
import sys, json
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase8-layouts'))
from layout_machine import Machine, P, V, H, DATA, native_windows
from unicorn.x86_const import *

RESOURCES = ['money','ammo','oil','unit_limit_provided','unit_limit_consumed','building_limit_provided','building_limit_consumed']
CW = {'Single24':0x7f, 'Double53':0x27f, 'Extended64':0x37f}

class StatisticsMachine(Machine):
    def __init__(self):
        self.resource_lookup = {}
        super().__init__()
        # Original CRT constructors called at their real static addresses.
        for ctor, obj in [(0x58e590,0xb3d6d0),(0x58e6a0,0xb3d6f4),(0x58e790,0xb3d718)]:
            result = self.thiscall(ctor,obj,[0])
            assert 'eax' in result, result
        self.stats = self.alloc(0x100)
        result = self.thiscall(0x4b5340,self.stats,[])
        assert 'eax' in result, result
        self.w(self.gd+0x574,self.stats)
        self.w(self.gd+0x38,len(RESOURCES))
        array = self.alloc(4*len(RESOURCES)); self.w(self.gd+0x34,array)
        for i,name in enumerate(RESOURCES):
            obj=self.alloc(0x20); self.w(obj+8,self.cstring(name)); self.w(obj+12,i)
            self.w(array+i*4,obj); self.resource_lookup[name]=obj

    def hook(self,u,pc,size,user):
        if pc == 0x48d500 and self.resource_lookup:
            name=self.raw_wstr(self.arg())
            obj=self.resource_lookup.get(name.lower(),0)
            self.recorded.append({'event':'resource_lookup','name':name,'index':self.r(obj+12) if obj else None})
            self.ret(obj,pop=4); return
        if pc == 0x405d60:
            self.recorded.append({'event':'exception_text_format','format':self.raw_wstr(self.arg(1))})
            self.w(self.arg(),self.cstring('<diagnostic formatter seam>')); self.ret(); return
        super().hook(u,pc,size,user)

    def snapshot(self):
        rows=[]
        for obj in self.manager_entries(self.stats):
            terms=[]; p=self.r(obj+0x1c)
            while p:
                t=self.r(p); terms.append({'stat_index':self.r(t),'weight_bits':self.r(t+4)})
                p=self.r(p+4)
            rows.append({'ids':self.wstr(self.r(obj+8)),'index':self.r(obj+12),'display_type':self.r(obj+0x18),'terms':terms})
        return rows

def run(pc, scoring):
    m=StatisticsMachine(); m.u.reg_write(UC_X86_REG_FPCW,CW[pc])
    m.parse_file('Templates/template_scoring.tgi')
    parsed=m.parse_file('Scoring/Scoring.tgi',scoring.encode())
    for node in m.children(parsed['root']):
        label=m.node_name(node)
        if label != 'Statistic' and not label.endswith('_Statistic'):
            continue
        result=m.run_vf0(m.stats,node)
        if 'thrown' in result:
            return m, {'thrown':m.last_throw,'messages':m.messages,'definitions':m.snapshot()}
    return m, {'thrown':None,'definitions':m.snapshot(), 'production_start':m.r(0xb3b428),'rate_start':m.r(0xb3b424),'derived_start':m.r(0xb3b420)}

def main():
    source=(DATA/'Scoring/Scoring.tgi').read_text()
    rows=[]; visited={}
    for pc in CW:
        m,result=run(pc,source)
        print(pc, len(result['definitions']), result.get('thrown'), flush=True)
        rows.append({'name':'shipped','precision':pc,'source':source,'result':result})
        visited.update(m.visited)
    edits = [
        ('missing_reference', source.replace('stat_ids\t\t\t\t= STRUCTURES_KILLED','stat_ids\t\t\t\t= NO_SUCH_STAT')),
        ('string_reference', source.replace('stat_ids\t\t\t\t= STRUCTURES_KILLED','stat_ids\t\t\t\t= FAVORITE_UNIT')),
        ('forward_reference', source.replace('stat_ids\t\t\t\t= STRUCTURES_KILLED','stat_ids\t\t\t\t= economic_score')),
        ('resource_order', source.replace('resource_ids\t\t\t= money','resource_ids\t\t\t= oil',1)),
    ]
    for name,edited in edits:
        assert edited != source, name
        m,result=run('Double53',edited)
        print(name,result.get('thrown'),flush=True)
        assert result['thrown'], name
        rows.append({'name':name,'precision':'Double53','source':edited,'result':result})
        visited.update(m.visited)
    out={'schema':1,'scope':__doc__,'original_sha256':H(P.b),'v14_sha256':H(V.b),'template':(DATA/'Templates/template_scoring.tgi').read_text(),'resources':RESOURCES,'cases':rows,'native_windows':native_windows(visited)}
    path=HERE/'native-statistics-source.json'; path.write_text(json.dumps(out,indent=2)+'\n')
    print(path,H(path.read_bytes()))

if __name__=='__main__': main()
