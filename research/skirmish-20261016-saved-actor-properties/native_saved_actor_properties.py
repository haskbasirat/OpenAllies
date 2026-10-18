"""Original4D4CF0/4D4E50 on captured saved Actor allocations and canonical Builder inputs.

The separately proven borrowed core's unchanged original/v14 harness executes
every input at its captured precision. Canonical source IDs map to stable opaque
u32 labels, preserving pointer identity/equality without pretending pointers are
saved-file strings. Output compares actual list order, tail value and
live allocation count. Invalid identity/source/cycle refusals remain separate
portable tests, not native malformed-pointer exception parity.
"""
from pathlib import Path
import json,sys,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261016-actor-property-core'))
from native_actor_property_core import Probe

def sha(b):return hashlib.sha256(b).hexdigest()
def run(c,image,pc):
    labels={}
    for actor in c['actors']:
        for value in actor['list']+actor['source']:
            labels.setdefault(value,len(labels)+1)
    reverse={v:k for k,v in labels.items()}
    source={'name':c['name'],'operation':c['operation'],'actor':c['actor'],
        'actors':[{**{k:a[k] for k in ('builder','flags','members','element','parent')},
                   'list':[labels[v] for v in a['list']],'source':[labels[v] for v in a['source']]}
                  for a in c['actors']]}
    native=Probe(source,image,pc).run()
    assert native['outcome']=='ok'
    owners=[i for i,a in enumerate(c['actors']) for _ in a['list']]
    owners += [None for a in c['actors'] for _ in a['source']]
    owners += [event[1] for event in native['trace'] if event[0]=='allocate_node']
    assert len(owners)==len(native['nodes'])
    actors=[]
    for index,a in enumerate(native['actors']):
        values=[];node=a['head'];seen=set()
        while node is not None:
            assert node not in seen;seen.add(node)
            n=native['nodes'][node];assert not n['freed'] and n['next_written']
            values.append(reverse[n['value']]);node=n['next']
        tail=a['tail'];tail_value=reverse[native['nodes'][tail]['value']] if tail is not None else None
        actors.append({'list':values,'tail_value':tail_value,
            'allocated_nodes':sum(owner==index and not node['freed'] for owner,node in zip(owners,native['nodes']))})
    return {'outcome':native['outcome'],'actors':actors}

def main():
    inputs=HERE/'actual-saved-inputs.json';rows=[]
    for row in json.loads(inputs.read_text()):
        for image in ('original','v14'):
            rows.append({'image':image,'precision':row['precision'],'input':row['input'],
                'output':run(row['input'],image,row['precision'])})
    doc={'schema':1,'scope':__doc__,'captured_inputs_sha256':sha(inputs.read_bytes()),
        'generator_sha256':sha(Path(__file__).read_bytes()),'cases':rows}
    path=HERE/'native-saved-actor-properties.json'
    if '--write' in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
    else:assert json.loads(path.read_text())==doc
    print(json.dumps({'cases':len(rows),'sha256':sha(path.read_bytes())}))
if __name__=='__main__':main()
