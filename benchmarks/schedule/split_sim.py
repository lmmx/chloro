#!/usr/bin/env python3
"""EXP-029: a split file's own schedule, rebuilt from on-CPU costs (trace_patch.py traces).

usage: split_sim.py [--runs] TRACE...                 replay every policy below; medians (and per run)
       split_sim.py --blind TRACE...                  diag.py's wall view with and without the item
                                                      workers' intervals (an unpatched trace has none)
       split_sim.py --compare DIR NAME:POLICY NAME:POLICY [CRATE...]
                                                      collect_ab.sh's traces: each build replayed under
                                                      its own policy, with its wall view; per run and medians

Policies, all replayed on four cores under processor sharing (every runnable thread progresses at
min(1, 4/runnable)), from the measured costs of every small file and of the split file's parts:
 A    today: the split file's worker spawns min(4, units) item threads and waits in join;
 a    lever (a): the split file's worker formats units itself beside min(4, units) - 1 threads;
 bs   lever (b), static: as a, the threads capped when the units phase starts at the number of
      workers that have found the file queue empty;
 bd   lever (b), dynamic: as a with no item threads; a worker that finds the file queue empty
      takes units of an open split file (never more runnable threads than cores);
 ad   as a, and a worker that finds the file queue empty also takes an open split file's units;
 +c8  lever (c)'s bound: every unit of at least 100 KB (lints.rs's two const arrays) cut into 8
      equal pieces at no cost;
 +d   lever (d)'s bound: the split file's parse on four threads (parse/4 each) at no cost, the
      units phase starting when all four are done;
 C    the work-conserving bound max(work/4, longest chain) (C_c8, C_d: with the cut, the parse).
Tail: from the first worker finding nothing left to take (no file; for bd and ad no unit either)
to the end; idle: core-time with fewer runnable threads than cores. A trace of the lever (a)
build (trace_patch.py ... owner) carries `item 5` lines: the calling thread's units, taken out of
its file's sequential part here.
"""
import os, sys, glob, statistics, json
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from sim import load as _load, jobs_in_order

def load(path):
    """sim.load, with each split file's units kept as (bytes, cost) pairs, largest bytes first."""
    d = _load(path)
    M = 1_000_000
    items, own = [], []
    for line in open(path):
        f = line.rstrip('\n').split('\t')
        if f[0] == 'item' and f[1] == '4':
            items.append((int(f[5]), int(f[4])))
        elif f[0] == 'item' and f[1] == '5':
            own.append((int(f[5]), int(f[4])))
    for s in d['splits']:
        us = sorted(((n % M, c) for n, c in items if n // M == s['bytes']), key=lambda u: -u[0])
        s['ubytes'] = us
        # kind 5 (lever a's diagnostic build): the calling thread's units, which its file CPU
        # also holds; they are units, not the sequential part.
        s['p1'] = max(0, s['p1'] - sum(c for n, c in own if n // M == s['bytes']))
    return d
NC = 4

def split_parts(path):
    """The split file's on-CPU breakdown, ms."""
    items, files, cpu = [], [], {}
    for line in open(path):
        f = line.rstrip('\n').split('\t')
        if f[0] == 'file':
            files.append(dict(bytes=int(f[2]), split=f[3] == '1', start=int(f[4]), read=int(f[5]),
                              formatted=int(f[7]), end=int(f[8])))
        elif f[0] == 'cpu':
            cpu[int(f[1])] = int(f[2])
        elif f[0] == 'item':
            items.append(dict(kind=int(f[1]), start=int(f[2]), end=int(f[3]), cpu=int(f[4]), n=int(f[5])))
    out = []
    M = 1_000_000
    for f in files:
        if not f['split']:
            continue
        src = f['bytes']
        fc = cpu.get(f['start'], 0)
        mine = lambda k: [i for i in items if i['kind'] == k and (i['n'] // M == src if k != 1 else i['n'] == src)]
        parse = sum(i['cpu'] for i in mine(1))
        own = sum(i['cpu'] for i in mine(5))  # lever (a)'s build: the worker's own units
        ph2 = sum(i['cpu'] for i in mine(2))  # the units phase on the worker (its own units included)
        stitch = sum(i['cpu'] for i in mine(3))
        units = sorted((i['cpu'] for i in mine(4)), reverse=True)
        iw = sum(i['cpu'] for i in mine(0))
        fc -= own
        ph2 -= own
        out.append(dict(bytes=src, file_cpu=fc / 1e6, parse=parse / 1e6, rest=(fc - parse - ph2 - stitch) / 1e6,
                        join_phase=ph2 / 1e6, stitch=stitch / 1e6, nunits=len(units), units=sum(units) / 1e6,
                        u1=units[0] / 1e6 if units else 0, u2=units[1] / 1e6 if len(units) > 1 else 0,
                        setup=(iw - sum(units)) / 1e6, nworkers=len(mine(0)),
                        read_wall=(f['read'] - f['start']) / 1e6))
    return out

def simulate(d, policy, cut=1):
    """Processor-sharing replay; returns (span, tail, idle core-time) in ns."""
    pparse = policy.endswith('+d')
    policy = policy[:-2] if pparse else policy
    queue = jobs_in_order(d)
    qi = 0
    t = 0.0
    idle = 0.0
    first_out = None
    dyn = policy in ('bd', 'ad')
    threads = []  # every thread: dict(rem, role, ...)
    open_groups = []

    def units_of(s):
        return cut_units(s, cut)

    def pull_file(w):
        nonlocal qi
        if qi < len(queue):
            j = queue[qi]; qi += 1
            if j[0] == 's':
                w.update(task='s', rem=j[2])
            else:
                s = j[2]
                if pparse and s.get('parse', 0) > 0:
                    q = s['parse'] / NC
                    g = dict(live=NC - 1, owner=w, s=s)
                    for _ in range(NC - 1):
                        threads.append(dict(role='i', done=False, blocked=False, task='pp', grp=g, rem=q))
                    w.update(task='p1', s=s, rem=s['p1'] - (NC - 1) * q, pgrp=g)
                else:
                    w.update(task='p1', s=s, rem=s['p1'], pgrp=None)
            return True
        return False

    def out_of_work(w):
        nonlocal first_out
        w['done'] = True
        if first_out is None:
            first_out = t

    def take_unit(x, g):
        c = g['pending'].pop(0)
        first = id(x) not in g['seen']
        g['seen'].add(id(x))
        x.update(task='u', grp=g, rem=c + (g['per_setup'] if first else 0))
        g['live'] += 1

    def next_for_worker(w):
        # A worker: a new file; with the dynamic policy, otherwise a unit of an open split file.
        if pull_file(w):
            return
        if dyn:
            for g in open_groups:
                if g['pending']:
                    take_unit(w, g); return
        out_of_work(w)

    def idle_workers():
        return sum(1 for x in threads if x['role'] == 'w' and x['done'])

    def start_units(w, s):
        pending = units_of(s)
        k = min(NC, len(pending))
        g = dict(pending=pending, live=0, owner=w, s=s, seen=set(), per_setup=s['setup'] / max(1, s['nworkers']))
        if len(pending) <= 1:
            w.update(task='p3', rem=sum(pending) + s['stitch'])
            return
        open_groups.append(g)
        if policy == 'A':
            w['blocked'] = True
            for _ in range(k):
                it = dict(role='i', done=False, blocked=False)
                take_unit(it, g)
                threads.append(it)
            return
        helpers = k - 1
        if policy == 'bs':
            helpers = min(helpers, idle_workers())
        elif policy == 'bd':
            helpers = 0
        for _ in range(helpers):
            it = dict(role='i', done=False, blocked=False)
            take_unit(it, g)
            threads.append(it)
        take_unit(w, g)

    def unit_done(x):
        g = x['grp']; g['live'] -= 1
        o = g['owner']
        if g['pending']:
            # keep taking this file's units (helpers and the owner alike)
            take_unit(x, g)
            return
        # nothing left to take
        if x['role'] == 'i':
            x['done'] = True
        elif x is not o:
            next_for_worker(x)
        else:
            x['blocked'] = True  # the owner waits for the others' units
        if g['live'] == 0:
            open_groups.remove(g)
            o['blocked'] = False
            o.update(task='p3', rem=g['s']['stitch'])

    for _ in range(min(NC, len(queue))):
        w = dict(role='w', done=False, blocked=False)
        threads.append(w)
        if not pull_file(w):
            out_of_work(w)
    while True:
        run = [x for x in threads if not x['done'] and not x['blocked']]
        if not run:
            break
        rate = min(1.0, NC / len(run))
        dt = min(x['rem'] for x in run) / rate
        idle += (NC - min(NC, len(run))) * dt
        t += dt
        for x in run:
            x['rem'] -= rate * dt
        for x in run:
            if x['rem'] > 1e-6:
                continue
            task = x['task']
            if task == 'u':
                unit_done(x)
            elif task == 'pp':
                x['done'] = True
                g = x['grp']; g['live'] -= 1
                o = g['owner']
                if g['live'] == 0 and o.get('pwait'):
                    o['blocked'] = False; o['pwait'] = False
                    start_units(o, o['s'])
            elif task == 'p1':
                g = x.get('pgrp')
                if g is not None and g['live'] > 0:
                    x['blocked'] = True; x['pwait'] = True
                else:
                    start_units(x, x['s'])
            else:  # 's' or 'p3'
                next_for_worker(x)
    return t, (t - first_out) if first_out is not None else 0.0, idle

CUT_MIN = 100_000  # bytes: the units (c) cuts below the item level (lints.rs's two const arrays)

def cut_units(s, cut):
    """The split file's unit costs, longest first; with cut > 1 every unit of at least CUT_MIN
    bytes becomes `cut` equal pieces (no overhead: the bound)."""
    us = []
    for b, c in s['ubytes']:
        if cut > 1 and b >= CUT_MIN:
            us += [(b / cut, c / cut)] * cut
        else:
            us.append((b, c))
    # the scheduler's order: longest in bytes first (format_units)
    return [c for _, c in sorted(us, key=lambda u: -u[0])]

def bound(d, cut=1, pparse=False):
    total = sum(d['smalls']) + sum(s['p1'] + sum(s['units']) + s['stitch'] for s in d['splits'])
    chains = [max(d['smalls'], default=0)]
    for s in d['splits']:
        us = cut_units(s, cut)
        p1 = s['p1'] - (s['parse'] * (NC - 1) / NC if pparse else 0)
        chains.append(p1 + max(us, default=0) + s['stitch'])
    return max(total / NC, max(chains)), total, max(chains)

POLICIES = [('A', 'A', 1), ('a', 'a', 1), ('bs', 'bs', 1), ('bd', 'bd', 1), ('ad', 'ad', 1),
            ('A+c8', 'A', 8), ('ad+c8', 'ad', 8), ('A+d', 'A+d', 1), ('A+c8+d', 'A+d', 8)]

def replay(p):
    d = load(p)
    ms = 1e-6
    c, total, chain = bound(d)
    r = dict(work=total * ms, quarter=total / NC * ms, chain=chain * ms, C=c * ms,
             C_c8=bound(d, 8)[0] * ms, C_d=bound(d, 1, True)[0] * ms)
    for name, pol, cut in POLICIES:
        span, tail, idle = simulate(d, pol, cut)
        r[name] = span * ms; r[name + '_tail'] = tail * ms; r[name + '_idle'] = idle * ms
    parts = split_parts(p)
    if parts:
        big = max(parts, key=lambda s: s['bytes'])
        for k in ('bytes', 'file_cpu', 'parse', 'rest', 'join_phase', 'stitch', 'units', 'nunits', 'u1', 'u2',
                  'setup', 'nworkers', 'read_wall'):
            r['s_' + k] = big[k]
    return r

def medians(rows):
    return {k: round(statistics.median(r[k] for r in rows), 3) for k in rows[0]}

def blind(paths):
    from diag import parse, analyse
    rows = []
    for p in paths:
        marks, workers, files, items = parse(p)
        seen = analyse(marks, workers, files, items, NC)
        unseen = analyse(marks, workers, files, [], NC)
        rows.append(dict(T=seen['T'], tail=seen['tail'], lost_tail_seen=seen['lost_tail'],
                         lost_tail_blind=unseen['lost_tail'], busy_seen=seen['busy'], busy_blind=unseen['busy']))
    for p, r in zip(paths, rows):
        print(os.path.basename(p), json.dumps({k: round(v, 3) for k, v in r.items()}))
    print('MEDIAN', json.dumps(medians(rows)))

def runs_tsv(d, name, crate):
    rows = []
    for p in sorted(glob.glob(f'{d}/{name}/{crate}.b*.runs.tsv')):
        lines = open(p).read().split('\n')
        keys = lines[0].split('\t')
        rows += [dict(zip(keys, l.split('\t'))) for l in lines[1:] if l]
    return rows

def compare(d, specs, crates):
    crates = crates or ['fx-hir', 'fx-ide-db', 'fx-hir-ty', 'fx-syntax', 'fx-parser', 'reg-tokio',
                        'reg-regex-syntax', 'reg-clap-builder', 'reg-winnow']
    out = {}
    for c in crates:
        res = {}
        for name, pol in specs:
            runs = []
            for p in sorted(glob.glob(f'{d}/{name}/{c}.b*.trace*.tsv')):
                r = replay(p)
                runs.append(dict(span=r[pol], tail=r[pol + '_tail'], idle=r[pol + '_idle'], quarter=r['quarter'],
                                 C=r['C']))
            wall = runs_tsv(d, name, c)
            for r, w in zip(runs, wall):
                r.update(wall=float(w['wall']), cpu=float(w['cpu']), wall_tail=float(w['tail']),
                         wall_lost_tail=float(w['lost_tail']), load=float(w['load']))
            res[name] = dict(median=medians(runs), runs=[{k: round(v, 3) for k, v in r.items()} for r in runs])
        out[c] = res
        (a, ma), (b, mb) = ((n, res[n]['median']) for n, _ in specs)
        print(f"{c}: span {ma['span']} -> {mb['span']}, tail {ma['tail']} -> {mb['tail']}, idle {ma['idle']} -> {mb['idle']}"
              f" | wall {ma['wall']} -> {mb['wall']}, cpu {ma['cpu']} -> {mb['cpu']},"
              f" wall tail {ma['wall_tail']} -> {mb['wall_tail']}, load {ma['load']}/{mb['load']}")
    json.dump(out, open(f'{d}/compare.json', 'w'), indent=0)

def main():
    args = sys.argv[1:]
    if args and args[0] == '--blind':
        return blind(args[1:])
    if args and args[0] == '--compare':
        return compare(args[1], [s.split(':') for s in args[2:4]], args[4:])
    show = bool(args) and args[0] == '--runs'
    paths = args[1:] if show else args
    rows = []
    for p in paths:
        r = replay(p)
        rows.append(r)
        if show:
            print(os.path.basename(p), json.dumps({k: round(v, 3) for k, v in r.items()}))
    print('MEDIAN', json.dumps(medians(rows)))

if __name__ == '__main__':
    main()
