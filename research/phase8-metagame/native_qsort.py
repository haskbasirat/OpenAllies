"""Original VS2003 CRT _qsort (96CE20, shortsort 96CDB0) with the shop cost
comparator 961790, as called by the shop loader 962290 (element = item
pointer, width 4, item+4 = cost).

Cases: generated arrays (sizes 0..70, many ties) and the authored shop lists of
MetaGame/mg_campaigns.tgi (Technology/Actor/Division rows in file order, the
order 961D90/961B10/962010 append them). Output: final item order.
Seams: none (the comparator and qsort both execute natively).
"""
import json, random, re, hashlib
from pathlib import Path
from mg_machine import Machine, P, V, VISITED, HERE, GAME

def run(m, costs):
    items = []
    for c in costs:
        it = m.alloc(0x10)
        m.w32(it + 4, c & 0xffffffff)
        items.append(it)
    base = m.array(items)
    m.call(0x96ce20, args=(base, len(items), 4, 0x961790))
    out = [m.r32(base + 4 * i) for i in range(len(items))]
    return [items.index(p) for p in out]

def shipped_shops():
    text = (GAME / 'reconstruction/assets/original/Data/MetaGame/mg_campaigns.tgi').read_text(encoding='cp1252')
    text = re.sub(r'/\*.*?\*/', '', text, flags=re.S)
    shops = []
    for m in re.finditer(r'\[Template (\w+DynamicCampaignShop) Template=DefaultDynamicCampaignShopTemplate\]\s*\{(.*?)\n\}', text, re.S):
        body = re.sub(r';[^\n]*', '', m.group(2))
        rows = {'Technology': [], 'Actor': [], 'Division': []}
        for r in re.finditer(r'\[(Technology|Actor|Division)\]\s*ids\s*=\s*(\S+)\s*cost\s*=\s*(-?\d+)', body):
            rows[r.group(1)].append({'ids': r.group(2), 'cost': int(r.group(3))})
        shops.append({'template': m.group(1), 'rows': rows})
    assert len(shops) == 5, len(shops)
    return shops

def main():
    rng = random.Random(0x96ce20)
    m = Machine(0x027f)
    cases = []
    for n in list(range(0, 41)) + [48, 55, 64, 70]:
        for pool in ([0, 1], [0, 1, 2, 3], [25, 40, 50, 75, 100, 125, 150], None, 'sorted', 'reversed', 'equal'):
            if pool is None:
                costs = [rng.randint(-5, 200) for _ in range(n)]
            elif pool == 'sorted':
                costs = sorted(rng.randint(0, 9) for _ in range(n))
            elif pool == 'reversed':
                costs = sorted((rng.randint(0, 9) for _ in range(n)), reverse=True)
            elif pool == 'equal':
                costs = [25] * n
            else:
                costs = [rng.choice(pool) for _ in range(n)]
            cases.append({'costs': costs, 'order': run(m, costs)})
    shops = []
    for shop in shipped_shops():
        lists = {}
        for kind in ('Technology', 'Actor', 'Division'):
            costs = [r['cost'] for r in shop['rows'][kind]]
            lists[kind] = {'rows': shop['rows'][kind], 'order': run(m, costs)}
        # all-items vector: technologies, then actors, then divisions (append order)
        all_costs = [r['cost'] for k in ('Technology', 'Actor', 'Division') for r in shop['rows'][k]]
        lists['all'] = {'costs': all_costs, 'order': run(m, all_costs)}
        shops.append({'template': shop['template'], 'lists': lists})
    report = {
        'schema': 1, 'scope': __doc__,
        'original_sha256': hashlib.sha256(P.b).hexdigest(), 'v14_sha256': hashlib.sha256(V.b).hexdigest(),
        'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'cases': cases, 'shipped_shops': shops,
        'instructions': [{'va': hex(a), 'bytes': b} for a, b in sorted(VISITED.items())],
    }
    out = HERE / 'native-qsort.json'
    out.write_text(json.dumps(report, indent=1) + '\n')
    print(json.dumps({'cases': len(cases), 'shops': len(shops), 'visited': len(VISITED),
                      'sha256': hashlib.sha256(out.read_bytes()).hexdigest()}))

if __name__ == '__main__':
    main()
