"""Original Organization morale / attack-efficiency routines under Unicorn.

Executes unchanged 578140 (morale state), 5786D0 (gain), 579660 (loss),
579750 (signed modify), 578F10 (slayer), 4CEB40 (slayer dispatch), 579010
(breaker), 579AB0 (retreat penalty), 57A0A0 (morale tick), 5785D0 (attack
efficiency), 4CA050 (MORALE_GAIN/LOSS ability effect, with real 4C8330 and
4CE6C0), 579600 (engagement experience tick, with real 5718B0/571820) and
4CEBE0 (capture reward, with real 5719E0/571820).

Explicit seams (never executed here): 4D64F0 actor state transition, 51F9F0 /
51F920 Mind (CAI) exhausted/wavering reactions, Mind virtual +5C/+90/+188,
4CF110 modifier lookup, 4D49B0 current state-group descriptor, 4D01E0 actor
value, 4CE770 experience award and the target eligibility virtual +B4.
501500/501510 modifier getters, 4D1080, 4CFE90, 4CE6C0 and 4C8330 execute.
Callbacks may mutate the Organization and Mind fields to prove reread order.
"""
import copy
from harness import (Machine, PRECISIONS, f32, f64, install_settings, install_world,
                     report)
from unicorn.x86_const import UC_X86_REG_ECX

SETTINGS = {
    'engagement_loss': 0xcc, 'engagement_loss_factor_max': 0xd0, 'retreat_penalty': 0xd8,
    'slayer_base': 0xdc, 'slayer_min': 0xe0, 'slayer_max': 0xe4, 'breaker_base': 0xe8,
    'breaker_min': 0xec, 'breaker_max': 0xf0, 'replenish': 0xf4, 'supply_bonus': 0xf8,
    'experience_minimum': 0x160, 'experience_maximum': 0x164, 'experience_kill': 0x168,
    'experience_capture': 0x16c, 'experience_rout': 0x170, 'experience_engage': 0x174,
    'experience_default': 0x178, 'attack_efficiency_climbing': 0x1ac,
    'attack_efficiency_falling': 0x1b0,
}
DEFAULT_SETTINGS = {
    'engagement_loss': f32(.5), 'engagement_loss_factor_max': f32(3), 'retreat_penalty': f32(5),
    'slayer_base': f32(2), 'slayer_min': f32(.5), 'slayer_max': f32(2), 'breaker_base': f32(5),
    'breaker_min': f32(.5), 'breaker_max': f32(2), 'replenish': f32(1), 'supply_bonus': f32(.5),
    'experience_minimum': f32(0), 'experience_maximum': f32(10), 'experience_kill': f32(1),
    'experience_capture': f32(1), 'experience_rout': f32(1), 'experience_engage': f32(1),
    'experience_default': f32(1), 'attack_efficiency_climbing': f32(.01),
    'attack_efficiency_falling': f32(.02),
}
ORG_FIELDS = {'morale': 0x5c, 'maximum': 0x60, 'living': 0x64, 'attack_efficiency': 0x84,
              'attack_efficiency_desired': 0x88, 'breakpoint': 0xb8}
ORG_BYTES = {'engaged_latch': 0x4c, 'engage_latch': 0x4d}


def organization(**k):
    o = dict(morale=f32(50), maximum=f32(100), living=4, attack_efficiency=f32(1),
             attack_efficiency_desired=f32(1), breakpoint=f32(20), engaged_latch=False,
             engage_latch=False, members_total=4)
    o.update(k)
    return o


def base(name, operation, **k):
    c = dict(name=name, operation=operation, organization=organization(), settings=dict(DEFAULT_SETTINGS),
             game_mode=0, actor=dict(flags8=0, flags100=0, flags104=0, engaged=False, definition4f4=0),
             mind=None, modifier=None, state_group=None, actor_values={}, mutations=[], args={})
    c.update(k)
    return c


class Run(Machine):
    def __init__(self, c, precision):
        super().__init__(precision)
        self.u.mem_map(0, 0x1000)
        self.c = copy.deepcopy(c)
        self.mutations = list(self.c['mutations'])
        self.values = {k: list(v) for k, v in self.c['actor_values'].items()}
        settings = {SETTINGS[k]: v for k, v in self.c['settings'].items()}
        self.settings_table, self.settings_storage = install_settings(self, settings)
        self.world = install_world(self, self.c['game_mode'], f32(10))
        self.org = self.alloc(0x100)
        self.actor = self.alloc(0x200)
        self.definition = self.alloc(0x100)
        self.actor_definition = self.alloc(0x600)
        self.w(self.org + 4, self.actor)
        self.w(self.org + 0x18, self.definition)
        self.w(self.actor + 4, self.actor_definition)
        self.w(self.actor + 0x7c, self.org)
        self.w(self.actor, self.vtable({0xb4: self.target_eligible}))
        self.mind = self.alloc(0x100)
        self.w(self.mind, self.vtable({0x5c: self.mind_query, 0x90: self.engage_target,
                                       0x188: self.capture_notify}))
        self.modifier = self.alloc(0x100)
        self.descriptor = self.alloc(0x40)
        # Other actors: 2 = other/killer/attacker, 3 = victim/target, 4 = organization actor.
        self.others = {}
        for tag in (2, 3, 4):
            actor = self.alloc(0x200)
            definition = self.alloc(0x600)
            self.w(actor + 4, definition)
            self.w(actor, self.vtable({0xb4: self.target_eligible}))
            self.others[tag] = actor
        self.tags = {self.actor: 1, **{v: k for k, v in self.others.items()}}
        self.write_state()
        self.hooks.update({0x4d64f0: self.set_state, 0x51f9f0: self.mind_exhausted,
                           0x51f920: self.mind_wavering, 0x4cf110: self.modifier_lookup,
                           0x4d49b0: self.state_group, 0x4d01e0: self.actor_value,
                           0x4ce770: self.award})

    # state transfer --------------------------------------------------------
    def write_state(self):
        o = self.c['organization']
        for k, off in ORG_FIELDS.items():
            self.w(self.org + off, o[k])
        for k, off in ORG_BYTES.items():
            self.wb(self.org + off, int(o[k]))
        self.w(self.definition + 0x4c, o['members_total'])
        a = self.c['actor']
        self.w(self.actor + 8, a['flags8'])
        self.w(self.actor + 0x100, a['flags100'])
        self.w(self.actor + 0x104, a['flags104'])
        self.w(self.actor + 0x118, 0x1234 if a['engaged'] else 0)
        self.w(self.actor_definition + 0x4f4, a['definition4f4'])
        m = self.c['mind']
        self.w(self.actor + 0x70, self.mind if m is not None else 0)
        if m is not None:
            self.w(self.mind + 0x28, m['word28'])
            self.w(self.mind + 0x2c, m['word2c'])
        mod = self.c['modifier']
        if mod is not None:
            self.w(self.modifier, mod['flags'])
            self.w(self.modifier + 0x44, mod['multiply'])
            self.w(self.modifier + 0x48, mod['divide'])
        g = self.c['state_group']
        if g is not None:
            for i, key in enumerate(['byte1c', 'byte1d', 'byte1e']):
                self.wb(self.descriptor + 0x1c + i, int(g[key]))
        for tag, actor in self.others.items():
            view = self.c.get('others', {}).get(str(tag), {})
            self.w(actor + 0x7c, 0)
            self.w(actor + 0x80, 0)
            self.w(self.r(actor + 4) + 0x4f4, view.get('definition4f4', 0))
            if 'organization_maximum' in view:
                org = self.alloc(0x100)
                self.w(actor + 0x7c, org)
                self.w(org + 0x60, view['organization_maximum'])
            if view.get('membership_organization') is not None:
                membership = self.alloc(0x40)
                self.w(actor + 0x80, membership)
                target = view['membership_organization']
                self.w(membership + 0x10, self.actor if target == 1 else (self.others[target] if target else 0))

    def read_org(self):
        o = {k: self.r(self.org + off) for k, off in ORG_FIELDS.items()}
        o.update({k: bool(self.rb(self.org + off)) for k, off in ORG_BYTES.items()})
        return o

    def event(self, name, value=None):
        self.events.append({'event': {name: value}, 'organization': self.read_org()})
        if self.mutations and self.mutations[0]['on'] == name:
            m = self.mutations.pop(0)
            for k, v in m.get('organization', {}).items():
                if k in ORG_FIELDS:
                    self.w(self.org + ORG_FIELDS[k], v)
                else:
                    self.wb(self.org + ORG_BYTES[k], int(v))
            if 'mind' in m:
                self.w(self.mind + 0x28, m['mind']['word28'])
                self.w(self.mind + 0x2c, m['mind']['word2c'])
            if 'mind_present' in m:
                self.w(self.actor + 0x70, self.mind if m['mind_present'] else 0)

    def tag(self, pointer):
        return self.tags.get(pointer, 0 if pointer == 0 else -1)

    # seams ---------------------------------------------------------------
    def set_state(self, m):
        assert self.reg(UC_X86_REG_ECX) == self.actor
        self.event('SetState', [self.arg(), bool(self.arg(1) & 0xff)])
        self.ret(8)

    def mind_exhausted(self, m):
        assert self.reg(UC_X86_REG_ECX) == self.mind
        self.event('MindExhausted')
        self.ret(0, 1)

    def mind_wavering(self, m):
        assert self.reg(UC_X86_REG_ECX) == self.mind
        self.event('MindWavering')
        self.ret(0, 1)

    def mind_query(self, m):
        assert self.reg(UC_X86_REG_ECX) == self.mind
        result = self.c['mind']['declines']
        self.event('MindDeclines', result)
        self.ret(0, int(result))

    def engage_target(self, m):
        target = self.c['args']['engage_target']
        self.event('EngageTarget', target)
        self.ret(0, self.others[target] if target else 0)

    def capture_notify(self, m):
        self.event('CaptureNotify', self.tag(self.arg()))
        self.ret(4)

    def target_eligible(self, m):
        tag = self.tag(self.reg(UC_X86_REG_ECX))
        result = self.c['args']['target_eligible']
        self.event('TargetEligible', [tag, result])
        self.ret(0, int(result))

    def modifier_lookup(self, m):
        tag = self.tag(self.reg(UC_X86_REG_ECX))
        present = self.c['modifier'] is not None
        self.event('Modifier', tag)
        self.ret(0, self.modifier if present else 0)

    def state_group(self, m):
        group = self.arg()
        self.event('StateGroup', group)
        self.ret(4, self.descriptor if self.c['state_group'] is not None else 0)

    def actor_value(self, m):
        assert self.arg() == 0
        tag = str(self.tag(self.reg(UC_X86_REG_ECX)))
        value = self.values[tag].pop(0)
        self.event('ActorValue', [int(tag), value])
        self.ret_ext(4, value)

    def award(self, m):
        tag = self.tag(self.reg(UC_X86_REG_ECX))
        self.event('Award', [tag, self.arg(), self.arg(1) & 0xff, self.arg(2) & 0xff])
        self.ret(12)

    # operations ----------------------------------------------------------
    def run(self):
        op = self.c['operation']
        a = self.c['args']
        result = None
        if op == 'state':
            result = self.call(0x578140, self.org, ())
        elif op == 'gain':
            self.call(0x5786d0, self.org, (a['amount'],))
        elif op == 'loss':
            self.call(0x579660, self.org, (a['amount'], a['kind']))
        elif op == 'modify':
            self.call(0x579750, self.org, (a['amount'], a['kind']))
        elif op == 'slayer':
            self.call(0x578f10, self.org, (self.others[2], self.others[3]))
        elif op == 'slayer_dispatch':
            self.call(0x4ceb40, self.others[2], (self.others[3],))
        elif op == 'breaker':
            self.call(0x579010, self.org, (self.others[2],))
        elif op == 'retreat':
            self.call(0x579ab0, self.org, ())
        elif op == 'tick':
            self.call(0x57a0a0, self.org, (a['delta'],))
        elif op == 'attack_efficiency':
            self.call(0x5785d0, self.org, (a['delta'],))
        elif op == 'effect':
            effect = self.alloc(0x40)
            context = self.alloc(0x40)
            self.w(effect + 4, a['effect_kind'])
            self.w(effect + 0x14, a['effect_amount'])
            target = a['target']
            self.w(context + 0x1c, (self.actor if target == 1 else self.others[target]) if target else 0)
            result = self.call(0x4ca050, effect, (0x77, context, a['scale'])) & 0xff
        elif op == 'engage_experience':
            self.call(0x579600, self.org, (a['delta'],))
        elif op == 'capture_reward':
            record = self.alloc(0x40)
            self.w(record + 0x18, a['record_kind'])
            self.call(0x4cebe0, self.actor, (self.others[3], record))
        else:
            raise ValueError(op)
        assert not self.mutations, self.mutations
        return {'result': result, 'organization': self.read_org(), 'events': self.events}


def cases():
    rows = []

    def add(c):
        rows.append(c)

    mind = lambda declines=False, w28=0, w2c=0: dict(declines=declines, word28=w28, word2c=w2c)
    # 578140 state classification.
    for morale, maximum, bp, m in [
        (50, 100, 20, mind()), (20, 100, 20, mind()), (10, 100, 20, mind()), (10, 100, 20, mind(True)),
        (0, 100, 20, mind()), (-0.0, 100, 20, mind()), (10, 0, 20, mind()), (10, -5, 20, mind()),
        (10, 100, 20, mind(False, 1, 0)), (10, 100, 20, mind(False, 0, 7)), (25, 100, 20, None),
        (0, 100, 20, None), (0, 0, 20, None), (20.000002, 100, 20, mind())]:
        add(base(f'state_{morale}_{maximum}_{bp}_{m}', 'state',
                 organization=organization(morale=f32(morale), maximum=f32(maximum), breakpoint=f32(bp)),
                 mind=m))
    # 5786D0 gain and 579750 positive branch.
    for morale, amount, maximum, m in [
        (50, 10, 100, mind()), (95, 10, 100, mind()), (100, 0.1, 100, mind()), (10, 5, 100, mind()),
        (0, 0.7, 100, mind()), (0, 0.7, 100, mind(True)), (19.9, 0.1, 100, mind()), (0, 0, 100, mind()),
        (7.3, 0.30000001, 100, mind()), (16777216, 1, 1e30, mind()), (3, -1, 100, mind()), (50, 10, 100, None)]:
        org = organization(morale=f32(morale), maximum=f32(maximum))
        add(base(f'gain_{morale}_{amount}_{maximum}_{m}', 'gain', organization=org, mind=m,
                 args=dict(amount=f32(amount))))
        add(base(f'modify_{morale}_{amount}_{maximum}_{m}', 'modify', organization=org, mind=m,
                 args=dict(amount=f32(amount), kind=0)))
    # 579660 loss and gates; 579750 negative branch.
    for morale, amount, bp, m, kind, extra in [
        (50, 10, 20, mind(), 0, {}), (25, 10, 20, mind(), 0, {}), (25, 10, 20, mind(True), 0, {}),
        (5, 10, 20, mind(), 0, {}), (5, 5, 20, mind(), 0, {}), (50, 60, 20, None, 0, {}),
        (50, 10, 20, None, 0, {}), (15, 1, 20, mind(), 0, {}), (15, 1, 20, mind(True), 0, {}),
        (50, -10, 20, mind(), 0, {}), (7.3, 0.1, 20, mind(), 0, {}), (50, 10, 20, mind(False, 1), 0, {}),
        (50, 10, 20, mind(), 0, {'game_mode': 6}), (50, 10, 20, mind(), 0, {'actor': dict(flags8=1, flags100=0, flags104=0, engaged=False, definition4f4=0)}),
        (50, 10, 20, mind(), 0, {'actor': dict(flags8=0, flags100=0, flags104=4, engaged=False, definition4f4=0)}),
        (50, 10, 20, mind(), 0, {'modifier': dict(flags=8, multiply=f32(2), divide=f32(2))}),
        (50, 10, 20, mind(), 3, {'modifier': dict(flags=8, multiply=f32(2), divide=f32(2))}),
        (50, 10, 20, mind(), 4, {'modifier': dict(flags=8, multiply=f32(2), divide=f32(2))}),
        (50, 10, 20, mind(), 0, {'modifier': dict(flags=1, multiply=f32(2), divide=f32(2))}),
        (0.30000001, 0.1, 20, mind(), 0, {})]:
        org = organization(morale=f32(morale), breakpoint=f32(bp))
        add(base(f'loss_{morale}_{amount}_{bp}_{m}_{kind}_{sorted(extra)}', 'loss', organization=org, mind=m,
                 args=dict(amount=f32(amount), kind=kind), **copy.deepcopy(extra)))
        add(base(f'modify_neg_{morale}_{amount}_{bp}_{m}_{kind}_{sorted(extra)}', 'modify', organization=org, mind=m,
                 args=dict(amount=f32(-amount), kind=kind), **copy.deepcopy(extra)))
    # Callback reread cases.
    add(base('loss_exhausted_callback_raises_morale', 'loss', organization=organization(morale=f32(5)),
             mind=mind(), args=dict(amount=f32(10), kind=0),
             mutations=[dict(on='MindExhausted', organization={'morale': f32(33)})]))
    add(base('loss_query_callback_busy', 'loss', organization=organization(morale=f32(15)),
             mind=mind(), args=dict(amount=f32(1), kind=0),
             mutations=[dict(on='MindDeclines', mind=dict(word28=1, word2c=0))]))
    add(base('gain_state_callback_changes_maximum', 'gain', organization=organization(morale=f32(0)),
             mind=mind(), args=dict(amount=f32(3), kind=0),
             mutations=[dict(on='SetState', organization={'maximum': f32(1)})]))
    # 578F10 slayer: actor values are binary64-exact extended values.
    for killer, victim, mn, mx, base_bits in [
        ([4.0, 4.0, 4.0], [2.0], .5, 2, 2), ([4.0, 4.0, 4.0], [12.0], .5, 2, 2), ([4.0, 4.0, 4.0], [1.0], .5, 2, 2),
        ([3.0, 3.0, 3.0], [1.0], .5, 2, 2.5), ([0.0], [], .5, 2, 2), ([5.0, 0.0], [], .5, 2, 2),
        ([7.0, 7.0, 7.0], [1.0 / 3.0], .25, 4, 1.1), ([7.0, 7.0, 7.0], [-7.0], .5, 2, 2),
        ([-2.0, -2.0, -2.0], [2.0], .5, 2, 2), ([1e-3, 1e-3, 1e-3], [0.1], .5, 2, 3.3),
        ([2.0, 4.0, 8.0], [5.0], .5, 2, 2)]:
        for morale, maximum, m in [(50, 100, mind()), (99, 100, mind()), (0, 100, mind()), (0, 100, mind(True))]:
            s = dict(DEFAULT_SETTINGS, slayer_min=f32(mn), slayer_max=f32(mx), slayer_base=f32(base_bits))
            values = {'2': [f64(v) for v in killer], '3': [f64(v) for v in victim]}
            add(base(f'slayer_{killer}_{victim}_{mn}_{mx}_{base_bits}_{morale}_{m}', 'slayer',
                     organization=organization(morale=f32(morale), maximum=f32(maximum)), mind=m,
                     settings=s, actor_values=values))
    for kmember, vmember, korg in [(4, 1, 1), (None, 1, 1), (4, None, 1), (4, 1, 0)]:
        others = {'2': {'membership_organization': korg if kmember else None},
                  '3': {'membership_organization': 1 if vmember else None}}
        if kmember:
            others['2']['membership_organization'] = 1 if korg else 0
        add(base(f'slayer_dispatch_{kmember}_{vmember}_{korg}', 'slayer_dispatch', mind=mind(),
                 others=others, actor_values={'2': [f64(4)] * 3, '3': [f64(2)]}))
    # 579010 breaker.
    for own_max, other_max, morale, m in [(100, 100, 50, mind()), (100, 400, 50, mind()), (100, 10, 50, mind()),
                                          (100, 130, 99, mind()), (0, 100, 50, mind()), (100, 0, 0, mind()),
                                          (100, -50, 0, mind(True)), (30, 7, 10, mind()), (100, 100, 30, None)]:
        add(base(f'breaker_{own_max}_{other_max}_{morale}_{m}', 'breaker',
                 organization=organization(maximum=f32(own_max), morale=f32(morale)), mind=m,
                 others={'2': {'organization_maximum': f32(other_max)}}))
    # 579AB0 retreat penalty.
    for morale, m, extra in [(50, mind(), {}), (22, mind(), {}), (22, mind(True), {}), (3, mind(), {}),
                             (60, None, {}), (50, mind(), {'game_mode': 6}),
                             (50, mind(False, 0, 1), {}),
                             (50, mind(), {'modifier': dict(flags=8, multiply=f32(2), divide=f32(2))}),
                             (5.3, mind(), {'settings': dict(DEFAULT_SETTINGS, retreat_penalty=f32(0.1))})]:
        add(base(f'retreat_{morale}_{m}_{sorted(extra)}', 'retreat', organization=organization(morale=f32(morale)),
                 mind=m, **copy.deepcopy(extra)))
    # 57A0A0 tick.
    for group, flags100, mod, delta in [
        (dict(byte1c=0, byte1d=1, byte1e=0), 0, None, .1), (dict(byte1c=0, byte1d=1, byte1e=0), 0x80000, None, .1),
        (dict(byte1c=0, byte1d=0, byte1e=0), 0, None, .1), (None, 0, None, .1),
        (dict(byte1c=0, byte1d=1, byte1e=0), 0, dict(flags=0, multiply=f32(1.5), divide=f32(3)), .1),
        (dict(byte1c=0, byte1d=1, byte1e=0), 0, dict(flags=0, multiply=f32(1.5), divide=f32(3)), 0.0333333)]:
        for latch, engaged, living, total, morale, m in [
            (False, False, 4, 4, 50, mind()), (True, True, 4, 4, 50, mind()), (True, True, 1, 4, 50, mind()),
            (True, True, 3, 7, 21, mind()), (True, False, 2, 4, 50, mind()), (False, True, 2, 4, 50, mind()),
            (True, True, 0, 4, 0.5, mind()), (True, True, 4, 4, 0.01, mind(True)), (True, True, 2, 4, 50, None)]:
            add(base(f'tick_{group}_{flags100}_{mod}_{delta}_{latch}_{engaged}_{living}_{total}_{morale}_{m}', 'tick',
                     organization=organization(engaged_latch=latch, living=living, members_total=total,
                                               morale=f32(morale)),
                     mind=m, modifier=mod, state_group=group,
                     actor=dict(flags8=0, flags100=flags100, flags104=0, engaged=engaged, definition4f4=0),
                     args=dict(delta=f32(delta))))
    add(base('tick_modifier_flag8_blocks_loss', 'tick', organization=organization(engaged_latch=True),
             mind=mind(), modifier=dict(flags=8, multiply=f32(1), divide=f32(1)),
             actor=dict(flags8=0, flags100=0, flags104=0, engaged=True, definition4f4=0),
             args=dict(delta=f32(.1))))
    add(base('tick_mode6_blocks_loss', 'tick', organization=organization(engaged_latch=True), game_mode=6,
             mind=mind(), actor=dict(flags8=0, flags100=0, flags104=0, engaged=True, definition4f4=0),
             args=dict(delta=f32(.1))))
    add(base('tick_mind_busy_blocks_loss', 'tick', organization=organization(engaged_latch=True),
             mind=mind(False, 1, 0), actor=dict(flags8=0, flags100=0, flags104=0, engaged=True, definition4f4=0),
             args=dict(delta=f32(.1))))
    # 5785D0 attack efficiency.
    for actual, desired, delta in [(1, 1, .1), (.5, 1, .1), (.999, 1, .1), (1, .5, .1), (.501, .5, .1),
                                   (0, 1, 10), (1, 0, 10), (.3, .7, 0), (.3, .7, -1), (.7, .3, -1),
                                   (0.30000001, 0.3, 1e-9)]:
        add(base(f'attack_efficiency_{actual}_{desired}_{delta}', 'attack_efficiency',
                 organization=organization(attack_efficiency=f32(actual), attack_efficiency_desired=f32(desired)),
                 args=dict(delta=f32(delta))))
    # 4CA050 morale effect.
    for kind, target, eligible, other_org in [(2, 1, True, None), (3, 1, True, None), (0, 1, True, None),
                                              (2, 1, False, None), (2, 0, True, None), (2, 2, True, 1),
                                              (3, 2, True, 1), (2, 2, True, None)]:
        others = {'2': {'membership_organization': other_org}} if other_org is not None else {}
        add(base(f'effect_{kind}_{target}_{eligible}_{other_org}', 'effect', mind=mind(), others=others,
                 args=dict(effect_kind=kind, effect_amount=f32(7.5), scale=f32(.6), target=target,
                           target_eligible=eligible)))
    # 579600 engagement experience.
    for flags100, latch, m, target, own, other in [
        (0x10, True, mind(), 3, 4.0, 2.0), (0x10, True, mind(), 3, 3.0, 0.0), (0x10, True, mind(), 0, 3.0, None),
        (0x10, False, mind(), 3, 4.0, 2.0), (0x10, True, None, 3, 4.0, 2.0), (0, True, mind(), 3, 4.0, 2.0),
        (0x10, True, mind(), 3, 0.0, 2.0), (0x10, True, mind(), 3, 7.0, 1.0 / 3.0),
        (0x10, True, mind(), 3, 5.0, 0.0)]:
        values = {'1': [f64(own)]}
        if target:
            values['3'] = [f64(other)] if other is not None else []
        others = {'3': {'definition4f4': 0xbf800000 if other == 0.0 and own == 3.0 else 0}}
        add(base(f'engage_experience_{flags100}_{latch}_{m}_{target}_{own}_{other}', 'engage_experience',
                 organization=organization(engage_latch=latch), mind=m,
                 actor=dict(flags8=0, flags100=flags100, flags104=0, engaged=False, definition4f4=0),
                 actor_values=values, others=others,
                 settings=dict(DEFAULT_SETTINGS, experience_engage=f32(1.25)),
                 args=dict(delta=f32(.1), engage_target=target)))
    # 4CEBE0 capture reward.
    for kind, own_org, own, victim, m in [(0, False, 4.0, 2.0, mind()), (7, False, 4.0, 2.0, mind()),
                                          (0, False, 4.0, 0.0, None), (1, False, 1.0, 64.0, mind()),
                                          (0, False, 0.0, 2.0, mind())]:
        add(base(f'capture_reward_{kind}_{own}_{victim}_{m}', 'capture_reward', mind=m,
                 actor_values={'1': [f64(own)], '3': [f64(victim)]},
                 settings=dict(DEFAULT_SETTINGS, experience_capture=f32(3), experience_maximum=f32(5),
                               experience_minimum=f32(.25)),
                 args=dict(record_kind=kind)))
    return rows


def main():
    out = []
    visited = {}
    for precision in PRECISIONS:
        for c in cases():
            try:
                m = Run(c, precision)
                result = m.run()
            except Exception as exc:
                raise RuntimeError(f'{c["name"]} {precision}') from exc
            visited.update(m.visited)
            out.append({'input': {**c, 'precision': precision}, 'output': result})
    report('native-morale-fixtures.json', __doc__, out, visited)


if __name__ == '__main__':
    main()
