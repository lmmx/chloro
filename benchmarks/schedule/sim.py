#!/usr/bin/env python3
"""EXP-023: rebuild a run's schedule from on-CPU costs (load-free) and simulate
 A: today's scheduler (4 file workers, largest first; a split file spawns min(4, units) item
    threads while its worker blocks; runnable threads share the 4 cores equally),
 B: a unit queue (4 workers; units of opened split files before new files; the worker that
    completes a file's last unit stitches it),
 C: the work-conserving bound max(total/4, longest chain).
usage: sim.py TRACE... ; prints one line per trace and the medians.
"""
import sys, statistics, json
NC = 4
M = 1_000_000

def load(path):
    marks, files, items, cpu = {}, [], [], {}
    for line in open(path):
        f = line.rstrip('\n').split('\t')
        if f[0] == 'mark': marks[f[1]] = int(f[2])
        elif f[0] == 'file':
            files.append(dict(bytes=int(f[2]), split=f[3] == '1', start=int(f[4]), end=int(f[8]), cpu=0))
        elif f[0] == 'cpu':
            cpu[int(f[1])] = int(f[2])
        elif f[0] == 'item':
            items.append(dict(kind=int(f[1]), start=int(f[2]), end=int(f[3]), cpu=int(f[4]), n=int(f[5])))
    for f in files:
        f['cpu'] = cpu.get(f['start'], 0)
    smalls, splits = [], []
    for f in files:
        if not f['split']:
            smalls.append(f['cpu']); continue
        src = f['bytes']
        units = [(i['n'] % M, i['cpu']) for i in items if i['kind'] == 4 and i['n'] // M == src]
        stitch = sum(i['cpu'] for i in items if i['kind'] == 3 and i['n'] // M == src)
        parse = sum(i['cpu'] for i in items if i['kind'] == 1 and i['n'] == src)
        iw = sum(i['cpu'] for i in items if i['kind'] == 0 and i['n'] // M == src)
        setup = max(0, iw - sum(c for _, c in units))  # item workers' visitor set-up
        units.sort(key=lambda u: -u[0])
        splits.append(dict(bytes=src, p1=max(0, f['cpu'] - stitch), stitch=stitch, parse=parse,
                           units=[c for _, c in units], setup=setup, nworkers=min(NC, len(units))))
    pre = marks['spawn']; post = marks['finish'] - marks['joined']
    return dict(smalls=smalls, splits=splits, pre=pre, post=post, T=marks['finish'],
                F=marks['joined'] - marks['spawn'], sizes=[f['bytes'] for f in files if not f['split']])

def jobs_in_order(d):
    js = [('s', b, c) for b, c in zip(d['sizes'], d['smalls'])] + [('S', s['bytes'], s) for s in d['splits']]
    js.sort(key=lambda j: -j[1])
    return js

def sim_a(d):
    """Processor sharing: every runnable thread progresses at min(1, NC/n)."""
    queue = jobs_in_order(d)
    qi = 0
    t = 0.0
    # thread: dict(rem, kind, ...); workers in states
    threads = []
    idle_core_time = 0.0
    first_exit = None
    def new_worker():
        return dict(kind='w', rem=0.0, task=None, blocked=False, done=False)
    workers = [new_worker() for _ in range(min(NC, len(queue)))]
    items = []  # item threads
    last_job = [None]
    def pull(w):
        nonlocal qi, first_exit
        if qi >= len(queue):
            w['done'] = True
            if first_exit is None: first_exit = t
            return
        j = queue[qi]; qi += 1
        w['bytes'] = j[1]
        if j[0] == 's':
            w['task'] = ('s',); w['rem'] = j[2]
        else:
            s = j[2]
            w['task'] = ('p1', s); w['rem'] = s['p1']
    for w in workers: pull(w)
    while True:
        run = [w for w in workers if not w['done'] and not w['blocked']] + [x for x in items if not x['done']]
        if not run: break
        rate = min(1.0, NC / len(run))
        idle_core_time += 0  # accounted below
        dt = min(x['rem'] for x in run) / rate
        busy = min(NC, len(run))
        idle_core_time += (NC - busy) * dt
        t += dt
        for x in run: x['rem'] -= rate * dt
        for x in run:
            if x['rem'] > 1e-6: continue
            if x['kind'] == 'w':
                task = x['task']
                last_job[0] = ('split' if task[0] == 'p3' else 'small', x.get('bytes', 0), t)
                if task[0] == 's' or task[0] == 'p3':
                    pull(x)
                elif task[0] == 'p1':
                    s = task[1]
                    pending = list(s['units'])
                    grp = dict(pending=pending, live=0, owner=x, s=s)
                    k = s['nworkers'] if len(pending) > 1 else 0
                    if k <= 1:
                        # sequential: the worker does all units itself
                        x['task'] = ('p3', s); x['rem'] = sum(pending) + s['stitch']
                    else:
                        x['blocked'] = True
                        per_setup = s['setup'] / k
                        for _ in range(k):
                            it = dict(kind='i', grp=grp, rem=per_setup + pending.pop(0) if pending else per_setup, done=False)
                            grp['live'] += 1
                            items.append(it)
            else:
                g = x['grp']
                if g['pending']:
                    x['rem'] = g['pending'].pop(0)
                else:
                    x['done'] = True; g['live'] -= 1
                    if g['live'] == 0:
                        o = g['owner']; o['blocked'] = False
                        o['task'] = ('p3', g['s']); o['rem'] = g['s']['stitch']
    return t, (t - first_exit) if first_exit is not None else 0.0, idle_core_time, last_job[0]

def sim_b(d):
    """Unit queue on NC workers at rate 1."""
    queue = jobs_in_order(d)
    qi = 0
    t = 0.0
    open_files = []  # dicts with pending units, outstanding count
    workers = [dict(rem=0.0, task=None, done=False) for _ in range(NC)]
    first_exit = None
    idle = 0.0
    def pull(w):
        nonlocal qi, first_exit
        for f in open_files:
            if f['pending']:
                c = f['pending'].pop(0)
                # A worker's first unit of a file carries its visitor's set-up (as an item
                # thread's does in A).
                first = id(w) not in f['seen']
                f['seen'].add(id(w))
                w['task'] = ('u', f)
                w['rem'] = c + (f['s']['setup'] / max(1, f['s']['nworkers']) if first else 0)
                return
        if qi < len(queue):
            j = queue[qi]; qi += 1
            if j[0] == 's':
                w['task'] = ('s',); w['rem'] = j[2]
            else:
                w['task'] = ('p1', j[2]); w['rem'] = j[2]['p1']
            return
        w['done'] = True
        if first_exit is None: first_exit = t
    for w in workers: pull(w)
    while True:
        run = [w for w in workers if not w['done']]
        if not run: break
        dt = min(w['rem'] for w in run)
        idle += (NC - len(run)) * dt
        t += dt
        for w in run: w['rem'] -= dt
        for w in run:
            if w['rem'] > 1e-6: continue
            task = w['task']
            if task[0] == 'p1':
                s = task[1]
                f = dict(s=s, pending=list(s['units']), outstanding=len(s['units']), seen=set())
                open_files.append(f)
                pull(w)
            elif task[0] == 'u':
                f = task[1]; f['outstanding'] -= 1
                if f['outstanding'] == 0:
                    open_files.remove(f)
                    w['task'] = ('p3',); w['rem'] = f['s']['stitch']
                else:
                    pull(w)
            else:
                pull(w)
    # idle workers that exited early also count
    return t, (t - first_exit) if first_exit is not None else 0.0, idle

def bound(d):
    total = sum(d['smalls']) + sum(s['p1'] + sum(s['units']) + s['stitch'] for s in d['splits'])
    chains = [max(d['smalls'], default=0)] + [s['p1'] + max(s['units'], default=0) + s['stitch'] for s in d['splits']]
    return max(total / NC, max(chains)), total, max(chains)

def main():
    rows = []
    for p in sys.argv[1:]:
        d = load(p)
        a, a_tail, a_idle, last = sim_a(d)
        b, b_tail, b_idle = sim_b(d)
        c, total, chain = bound(d)
        ms = 1e-6
        rows.append(dict(total=total*ms, quarter=total/NC*ms, chain=chain*ms, A=a*ms, A_tail=a_tail*ms,
                         A_idle=a_idle*ms, B=b*ms, B_tail=b_tail*ms, C=c*ms, pre=d['pre']*ms, post=d['post']*ms,
                         A_over=a/(total/NC), B_over=b/(total/NC), gain=1 - b / a,
                         T_meas=d['T']*ms, F_meas=d['F']*ms, last_split=1.0 if last[0] == 'split' else 0.0,
                         last_bytes=last[1], nsplit=len(d['splits']), split_work=sum(s['p1'] + sum(s['units']) + s['stitch'] for s in d['splits'])*ms))
    med = {k: round(statistics.median(r[k] for r in rows), 3) for k in rows[0]}
    print(json.dumps(med))

if __name__ == '__main__':
    main()
