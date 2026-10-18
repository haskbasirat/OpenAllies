"""Independent reverse-order replay of four frozen routing components.

Does not regenerate author fixtures. Adds visited-byte original/v14 comparisons
and nonvolatile-register sentinels for the tile search and cost producers.
"""
from pathlib import Path
import copy,hashlib,importlib.util,json

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
def load(name,path):
    spec=importlib.util.spec_from_file_location(name,ROOT/path)
    module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
    return module
pre=load('review_prepass','research/phase6-region/prepass_oracle.py')
graph=load('review_graph','research/phase6-region/graph_oracle.py')
search=load('review_search','research/phase6-pathsearch/native_search.py')
cost=load('review_cost','research/phase6-pathcost/native_cost.py')
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
original=graph.original;accepted=graph.modified
pe=pre.Pe(ROOT.parent/'AA.exe',pre.ORIGINAL_SHA)
all_visits={}
def monitor(machine,entry=None):
    regs={r:0x12340000+i*0x111 for i,r in enumerate([UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP])}
    def hook(u,address,size,_):
        if original.base<=address<original.base+original.image_size:
            a=original.range(address,size);b=accepted.range(address,size)
            assert a==b,hex(address)
            all_visits[address]=a.hex()
        if address==entry:
            for r,v in regs.items():u.reg_write(r,v)
    machine.u.hook_add(UC_HOOK_CODE,hook)
    return regs
groups=[('prepass',168,pre,'research/phase6-region/prepass-fixtures.json','fixtures'),
        ('graph',96,graph,'research/phase6-region/graph-fixtures.json','fixtures'),
        ('search',75,search,'research/phase6-pathsearch/native-search-fixtures.json','cases'),
        ('cost',177,cost,'research/phase6-pathcost/native-cost-fixtures.json','cases')]
results=[]
for name,count,module,path,key in groups:
    doc=json.loads((ROOT/path).read_text(encoding='utf-8'));rows=doc[key]
    assert len(rows)==count
    producer=Path(module.__file__)
    assert doc.get('oracle_sha256',doc.get('producer_sha256'))==sha(producer)
    for row in reversed(rows):
        data=copy.deepcopy(row['input'])
        if name=='prepass':
            m=module.Machine(pe,data);monitor(m);actual=m.run()
        elif name=='graph':
            m=module.Machine(data);monitor(m);actual=m.run()
        elif name=='search':
            m=module.SearchMachine({'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[row['precision']])
            regs=monitor(m,0x53d8d0);actual=m.run(data)
            for r,v in regs.items():assert m.u.reg_read(r)==v,(name,data['name'],'ABI')
        else:
            m=module.CostMachine(data);regs=monitor(m,0x53cb70);actual=m.run()
            for r,v in regs.items():assert m.u.reg_read(r)==v,(name,data['name'],'ABI')
        assert actual==row['output'],(name,data['name'])
    result={'component':name,'native_cases':count,'passed':True,'fixture_sha256':sha(ROOT/path)}
    results.append(result);print(json.dumps(result),flush=True)
report={'schema':1,'passed':True,'cases':sum(r['native_cases'] for r in results),'results':results,
        'order':'Reverse of each immutable authored fixture, comparing all original output fields.',
        'additional_checks':'Every visited native instruction byte equals accepted v14. Tile search and cost additionally checked callee-preserved register sentinels; region producers already check these.',
        'original_sha256':hashlib.sha256(original.b).hexdigest(),'accepted_sha256':hashlib.sha256(accepted.b).hexdigest(),
        'reviewer_sha256':sha(Path(__file__)),
        'visited_windows':[{'address':hex(a),'bytes':raw} for a,raw in sorted(all_visits.items())]}
(HERE/'review-routing-replay.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'passed':True,'native_cases':report['cases'],'sha256':sha(HERE/'review-routing-replay.json')}))
