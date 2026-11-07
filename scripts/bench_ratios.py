#!/usr/bin/env python3
"""Median benchmark ratios for the retune-cost requirements.

Runs the 16-voice benchmarks N times (default 5) and reports, for each pair
the `realtime-safety` spec bounds, the ratio inside each run and the median
of those ratios. A ratio within one run compares a benchmark with its twin
under the same machine conditions; the median over runs damps the
run-to-run noise (h12-skip-redundant-mode-retunes).

Usage: python3 scripts/bench_ratios.py [runs]
Set CARGO_BUILD_TARGET for your host if needed (see the README).
"""

import re
import statistics
import subprocess
import sys

PAIRS = [
    # (name, measured, twin, bound)
    ("bent / plain", "voice_manager_sixteen_voices_bent", "voice_manager_sixteen_voices_render", 1.1),
    (
        "dynamics / static",
        "voice_manager_sixteen_voices_cable_dynamics",
        "voice_manager_sixteen_voices_cable_static",
        1.5,
    ),
]
FILTER = "voice_manager_sixteen_voices_(render|bent|cable)"
TIME = re.compile(r"time:\s+\[\S+ \S+ (\S+) (\S+) \S+ \S+\]")


def run_once():
    output = subprocess.run(
        ["cargo", "bench", "--no-default-features", "--bench", "performance", "--", FILTER],
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    medians = {}
    name = None
    for line in output.splitlines():
        stripped = line.strip()
        if stripped.startswith("voice_manager_") and "time:" not in stripped:
            name = stripped.split()[0]
        match = TIME.search(stripped)
        if match:
            if stripped.startswith("voice_manager_"):
                name = stripped.split()[0]
            value, unit = float(match.group(1)), match.group(2)
            scale = {"ns": 1e-6, "µs": 1e-3, "us": 1e-3, "ms": 1.0, "s": 1e3}[unit]
            medians[name] = value * scale
    return medians


def main():
    runs = int(sys.argv[1]) if len(sys.argv) > 1 else 5
    ratios = {label: [] for label, *_ in PAIRS}
    for run in range(runs):
        medians = run_once()
        cells = []
        for label, measured, twin, _ in PAIRS:
            ratio = medians[measured] / medians[twin]
            ratios[label].append(ratio)
            cells.append(f"{label} {ratio:.3f}x ({medians[measured]:.2f} / {medians[twin]:.2f} ms)")
        print(f"run {run + 1}: " + "; ".join(cells), flush=True)
    failed = False
    for label, _, _, bound in PAIRS:
        median = statistics.median(ratios[label])
        verdict = "ok" if median <= bound else "OVER"
        failed |= median > bound
        print(f"MEDIAN {label}: {median:.3f}x (bound {bound}x) {verdict}")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
