#!/usr/bin/env python3
"""Turn a compare_rustfmt.sh results directory into markdown tables.

    python3 benchmarks/report.py RESULTS_DIR [--out FILE]

Reads every `<workload>.<variant>.r<k>.json` (hyperfine 2.0 `--export-json`), merges the
runs of each command across rounds, and prints one table per workload kind. Ratios are
chloro / rustfmt (and chloro / chloro-base when a baseline build ran). "ranges" compares
the min..max wall-time ranges of the two commands: `below` (every chloro run faster than
every rustfmt run), `above`, or `overlap`. Also reads `<workload>.1c.bench.txt` from
examples/bench. Standard library only.
"""

import json
import re
import statistics
import sys
from collections import defaultdict
from pathlib import Path

KIND_ORDER = ["crate", "files", "stdin", "inproc"]
KIND_TITLE = {
    "crate": "Crate level (rustfmt on the crate root, chloro on the same file set)",
    "files": "Many independent files",
    "stdin": "One file on stdin (rustfmt vs examples/fmt_stdin)",
    "inproc": "In process (examples/conform -i, wall time of the process)",
}


def read_meta(path):
    meta = {}
    if path.exists():
        for line in path.read_text().splitlines():
            if "=" in line:
                k, v = line.split("=", 1)
                meta[k] = v
    return meta


def collect(results_dir):
    runs = defaultdict(lambda: defaultdict(list))  # id -> label -> [measurement]
    for path in sorted(results_dir.glob("*.r*.json")):
        m = re.match(r"(.+)\.r(\d+)\.json$", path.name)
        if not m:
            continue
        data = json.loads(path.read_text())
        for result in data["results"]:
            runs[m.group(1)][result.get("name") or result["command"]].extend(result["measurements"])
    return runs


def stats(measurements):
    wall = [m["time_wall_clock"]["value"] for m in measurements]
    cpu = [
        m.get("time_user", {}).get("value", 0.0) + m.get("time_system", {}).get("value", 0.0)
        for m in measurements
    ]
    rss = [m["memory_peak_resident"]["value"] for m in measurements if "memory_peak_resident" in m]
    return {
        "n": len(wall),
        "mean": statistics.fmean(wall),
        "sd": statistics.stdev(wall) if len(wall) > 1 else 0.0,
        "min": min(wall),
        "max": max(wall),
        "cpu": statistics.fmean(cpu),
        "rss": max(rss) if rss else None,
        "exits": sorted({m.get("exit_code") for m in measurements}),
    }


def ms(x):
    return f"{x * 1e3:.1f}" if x < 10 else f"{x:.2f} s"


def fmt_time(s):
    return f"{ms(s['mean'])} ± {ms(s['sd'])} ({ms(s['min'])}–{ms(s['max'])})"


def mib(b):
    return "–" if b is None else f"{b / 2**20:.0f}"


def ranges(a, b):
    if a["max"] < b["min"]:
        return "below"
    if a["min"] > b["max"]:
        return "above"
    return "overlap"


def bench_table(results_dir):
    lines = []
    for path in sorted(results_dir.glob("*.bench.txt")):
        rows = []
        for line in path.read_text().splitlines():
            label, _, text = line.partition("\t")
            m = re.search(r"(\d+) files, ([\d.]+) MB: ([\d.]+)s, ([\d.]+) MB/s \(best of (\d+)\)", text)
            if m:
                rows.append((label, m))
        if not rows:
            continue
        name = path.name.split(".")[0]
        lines.append(f"### In process: {name} (examples/bench, best of -n per run)\n")
        lines.append("| run | binary | files | MB | best time | MB/s |")
        lines.append("|---|---|---|---|---|---|")
        for i, (label, m) in enumerate(rows, 1):
            lines.append(f"| {i} | {label} | {m.group(1)} | {m.group(2)} | {m.group(3)} s | {m.group(4)} |")
        by = defaultdict(list)
        for label, m in rows:
            by[label].append(float(m.group(4)))
        lines.append("")
        for label, v in by.items():
            lines.append(f"- {label}: {len(v)} runs, {min(v):.2f}–{max(v):.2f} MB/s")
        lines.append("")
    return lines


def main():
    args = sys.argv[1:]
    out = None
    if "--out" in args:
        i = args.index("--out")
        out = Path(args[i + 1])
        del args[i : i + 2]
    if len(args) != 1:
        sys.exit(__doc__)
    results_dir = Path(args[0])
    runs = collect(results_dir)
    lines = []
    env = results_dir / "env.txt"
    if env.exists():
        lines.append("```text")
        lines.extend(env.read_text().splitlines())
        lines.append("```\n")

    by_kind = defaultdict(list)
    for wid in sorted(runs):
        meta = read_meta(results_dir / f"{wid}.meta")
        by_kind[meta.get("kind", "inproc")].append((wid, meta))

    for kind in KIND_ORDER:
        if kind not in by_kind:
            continue
        lines.append(f"### {KIND_TITLE[kind]}\n")
        lines.append(
            "Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; "
            "CPU: mean user+sys ms.\n"
        )
        has_base = any("chloro-base" in runs[w] for w, _ in by_kind[kind])
        head = ["workload", "variant", "files", "KB", "n", "rustfmt", "chloro",
                "chloro/rustfmt mean", "chloro/rustfmt min", "ranges",
                "RSS rustfmt", "RSS chloro", "CPU rustfmt", "CPU chloro", "identical outputs", "exit codes"]
        if has_base:
            head[7:7] = ["chloro-base", "chloro/base mean", "ranges vs base"]
        if kind == "inproc":
            head = ["workload", "variant", "n", "command", "wall", "RSS", "CPU", "exit codes"]
        lines.append("| " + " | ".join(head) + " |")
        lines.append("|" + "---|" * len(head))
        for wid, meta in by_kind[kind]:
            name, _, variant = wid.rpartition(".")
            st = {label: stats(ms_) for label, ms_ in runs[wid].items()}
            if kind == "inproc":
                for label, s in st.items():
                    lines.append(
                        f"| {name} | {variant} | {s['n']} | {label} | {fmt_time(s)} | {mib(s['rss'])} | "
                        f"{ms(s['cpu'])} | {s['exits']} |"
                    )
                continue
            r, c = st.get("rustfmt"), st.get("chloro")
            if r is None or c is None:
                continue
            kb = int(meta.get("bytes", 0)) / 1000
            row = [name, f"{variant} ({meta.get('pin', '?')})", meta.get("files", "?"), f"{kb:.0f}",
                   f"{c['n']}", fmt_time(r), fmt_time(c),
                   f"{c['mean'] / r['mean']:.3f}", f"{c['min'] / r['min']:.3f}", ranges(c, r),
                   mib(r["rss"]), mib(c["rss"]), ms(r["cpu"]), ms(c["cpu"]),
                   meta.get("identical_outputs", "?"),
                   f"r{r['exits']} c{c['exits']}"]
            if has_base:
                b = st.get("chloro-base")
                row[7:7] = ([fmt_time(b), f"{c['mean'] / b['mean']:.3f}", ranges(c, b)]
                            if b else ["–", "–", "–"])
            lines.append("| " + " | ".join(row) + " |")
        lines.append("")
    lines.extend(bench_table(results_dir))
    text = "\n".join(lines) + "\n"
    (out or results_dir / "report.md").write_text(text)
    sys.stdout.write(text)


if __name__ == "__main__":
    main()
