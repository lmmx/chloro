#!/usr/bin/env python3
"""EXP-023: markdown tables from a collect.sh results directory.
usage: report.py OUT [d1|d2|d4]...
  d1: the measured schedule at 4 threads (wall view); d2: the reconstruction from thread CPU
  clocks (sim.py; needs a trace_patch.py build); d4: the phases of a run per mode."""
import json, sys, csv, statistics, subprocess, os
HERE = os.path.dirname(os.path.abspath(__file__))
D = sys.argv[1]
P = ''
NAMES = 'fx-hir fx-hir-ty fx-ide-db fx-syntax fx-parser reg-tokio reg-regex-syntax reg-clap-builder reg-winnow'.split()

def medians(tag):
    out = {}
    for line in open(f'{D}/{P}{tag}.log'):
        if line.startswith('{'):
            out[NAMES[len(out)]] = json.loads(line)
    return out

def runs(tag, c):
    return list(csv.DictReader(open(f'{D}/{P}{tag}/{c}.runs.tsv'), delimiter='\t'))

def loads(tag):
    ls = []
    for line in open(f'{D}/{P}{tag}.log'):
        if not line.startswith('{') and 'load' in line:
            ls.append(float(line.split('load ')[1].split()[0]))
    return ls

def d1():
    m = medians('base4')
    print('| crate | wall | in-process | outside `main` | before workers | after join | tail span | tail / in-process | idle core-ms in tail | last file split (runs) | worker utilisation mean (min) | on-CPU share | CPU | load |')
    print('|---|---|---|---|---|---|---|---|---|---|---|---|---|---|')
    for c in NAMES:
        d = m[c]; rs = runs('base4', c)
        split_last = sum(1 for r in rs if r['owner_split'] == 'True')
        umin = statistics.median(min(float(u) for u in r['util'].split(',')) for r in rs)
        print(f"| {c} | {d['wall']:.1f} | {d['T']:.1f} | {d['wall']-d['T']:.1f} | {d['pre']:.2f} | {d['post']:.2f} | {d['tail']:.1f} | {100*d['tail_share']:.1f}% | {d['lost_tail']:.1f} | {split_last}/{len(rs)} | {d['util_mean']:.2f} ({umin:.2f}) | {d['oncpu_frac']:.2f} | {d['cpu']:.1f} | {d['load']:.1f} |")

def d2():
    print('| crate | split files | work (CPU ms) | of it split files | work / 4 | longest chain | today (A) | A / (work/4) | A idle core-ms | A tail | last to finish in A | unit queue (B) | B / (work/4) | B gain over A | bound C |')
    print('|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|')
    for c in NAMES:
        tr = sorted(f'{D}/{P}base4/{f}' for f in os.listdir(f'{D}/{P}base4') if f.startswith(c + '.trace'))
        j = json.loads(subprocess.check_output(['python3', '-I', f'{HERE}/sim.py'] + tr))
        last = f"split file ({int(j['last_bytes'])/1024:.0f} KiB)" if j['last_split'] >= 0.5 else 'a whole small file'
        print(f"| {c} | {int(j['nsplit'])} | {j['total']:.1f} | {j['split_work']:.1f} | {j['quarter']:.1f} | {j['chain']:.1f} | {j['A']:.1f} | {j['A_over']:.3f} | {j['A_idle']:.2f} | {j['A_tail']:.2f} | {last} | {j['B']:.1f} | {j['B_over']:.3f} | {100*j['gain']:.1f}% | {j['C']:.1f} |")

def d4():
    print('| crate | mode | threads | wall | outside `main` | args | cache open | walk | sort | read (sum) | hash (sum) | format (sum) | output (sum) | cache save | CPU | load |')
    print('|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|')
    for c in NAMES:
        for tag, mode, th in [('base1', 'check, no cache', 1), ('base4', 'check, no cache', 4), ('cold1', 'check, cold cache', 1), ('cold4', 'check, cold cache', 4), ('write1', 'write raw, no cache', 1), ('write4', 'write raw, no cache', 4)]:
            if not os.path.exists(f'{D}/{P}{tag}.log'):
                continue
            d = medians(tag).get(c)
            if not d: continue
            print(f"| {c} | {mode} | {th} | {d['wall']:.1f} | {d['wall']-d['T']:.1f} | {d['args']:.2f} | {d['cache']:.2f} | {d['walk']:.2f} | {d['sort']:.2f} | {d['read']:.2f} | {d['hash']:.2f} | {d['fmt']:.1f} | {d['out']:.2f} | {d['save']:.2f} | {d['cpu']:.1f} | {d['load']:.1f} |")

for name in sys.argv[2:] or ['d1', 'd2', 'd4']:
    print(f'\n### {name}\n')
    globals()[name]()
