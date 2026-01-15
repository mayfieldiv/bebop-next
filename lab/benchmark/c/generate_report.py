#!/usr/bin/env python3

import json
import sys
import subprocess
from collections import defaultdict
from pathlib import Path

def get_machine_info():
    try:
        result = subprocess.run(
            ['system_profiler', 'SPHardwareDataType'],
            capture_output=True, text=True
        )
        model = chip = None
        for line in result.stdout.splitlines():
            if 'Model Name:' in line:
                model = line.split(':')[1].strip()
            elif 'Chip:' in line:
                chip = line.split(':')[1].strip()
        if model and chip:
            return f"{model} ({chip})"
    except:
        pass
    import platform
    return platform.machine()

def format_time(ns):
    if ns is None:
        return "-"
    if ns >= 1_000_000:
        return f"{ns / 1_000_000:.2f}ms"
    if ns >= 1_000:
        return f"{ns / 1_000:.2f}us"
    return f"{ns:.2f}ns"

def format_throughput(bps):
    if bps is None or bps == 0:
        return "-"
    if bps >= 1e12:
        return f"{bps / 1e12:.2f} TB/s"
    if bps >= 1e9:
        return f"{bps / 1e9:.2f} GB/s"
    if bps >= 1e6:
        return f"{bps / 1e6:.2f} MB/s"
    if bps >= 1e3:
        return f"{bps / 1e3:.2f} KB/s"
    return f"{bps:.0f} B/s"

def format_cv(cv):
    if cv is None:
        return "-"
    return f"{cv * 100:.2f}%"

def parse_benchmark_name(name):
    # Handle MT format: BM_BebopMT_Decode_PersonSmall/threads:4_mean
    if '/threads:' in name:
        base, thread_part = name.split('/threads:')
        parts = base.split('_')
        lib = parts[1]  # BebopMT
        op = parts[2]   # Decode
        schema = '_'.join(parts[3:])
        # Extract thread count and aggregate type
        thread_agg = thread_part.split('_')
        threads = int(thread_agg[0])
        return lib, op, schema, threads

    parts = name.split('_')
    lib = parts[1]
    op = parts[2]
    schema = '_'.join(parts[3:])
    if schema.endswith('_mean'):
        schema = schema[:-5]
    elif schema.endswith('_median'):
        schema = schema[:-7]
    elif schema.endswith('_stddev'):
        schema = schema[:-7]
    elif schema.endswith('_cv'):
        schema = schema[:-3]
    return lib, op, schema, None

def load_benchmarks(path):
    with open(path) as f:
        return json.load(f)

def is_mt_benchmark(data):
    for b in data['benchmarks']:
        if '/threads:' in b['name']:
            return True
    return False

def group_benchmarks(data):
    grouped = defaultdict(lambda: defaultdict(lambda: defaultdict(dict)))

    for b in data['benchmarks']:
        lib, op, schema, threads = parse_benchmark_name(b['name'])
        agg = b.get('aggregate_name', 'raw')
        grouped[lib][op][schema][agg] = {
            'time': b.get('real_time'),
            'throughput': b.get('bytes_per_second'),
        }

    return grouped

def group_mt_benchmarks(data):
    # Group by lib -> op -> schema -> threads -> agg
    grouped = defaultdict(lambda: defaultdict(lambda: defaultdict(lambda: defaultdict(dict))))

    for b in data['benchmarks']:
        lib, op, schema, threads = parse_benchmark_name(b['name'])
        if threads is None:
            continue
        agg = b.get('aggregate_name', 'raw')
        grouped[lib][op][schema][threads][agg] = {
            'time': b.get('real_time'),
            'throughput': b.get('bytes_per_second'),
        }

    return grouped

def generate_mt_report(input_file, output_file):
    data = load_benchmarks(input_file)
    ctx = data['context']
    grouped = group_mt_benchmarks(data)

    lines = []
    w = lines.append

    w("# Multi-Threaded Serialization Benchmark Report")
    w("")

    w("## Experimental Setup")
    w("")
    w("### Hardware Configuration")
    w("")
    w("| Parameter | Value |")
    w("|-----------|-------|")
    w(f"| Machine | {get_machine_info()} |")
    w(f"| Host | {ctx['host_name']} |")
    w(f"| CPU Count | {ctx['num_cpus']} |")
    w(f"| MHz per CPU | {ctx['mhz_per_cpu']} |")
    w(f"| CPU Scaling | {'Enabled' if ctx.get('cpu_scaling_enabled') else 'Disabled'} |")
    w(f"| Build Type | {ctx['library_build_type']} |")
    w(f"| Date | {ctx['date']} |")
    load_avg = ', '.join(f"{x:.2f}" for x in ctx['load_avg'])
    w(f"| Load Average | {load_avg} |")
    w("")

    w("### Cache Hierarchy")
    w("")
    w("| Level | Type | Size |")
    w("|-------|------|------|")
    for cache in ctx.get('caches', []):
        size_kb = cache['size'] // 1024
        w(f"| L{cache['level']} | {cache['type']} | {size_kb}KB |")
    w("")

    reps = data['benchmarks'][0].get('repetitions', 'N/A')
    w("### Methodology")
    w("")
    w(f"Each benchmark ran {reps} times per thread configuration. Statistics: mean, median, stddev, CV.")
    w("")

    # Collect all thread counts used
    all_threads = set()
    for lib in grouped:
        for op in grouped[lib]:
            for schema in grouped[lib][op]:
                all_threads.update(grouped[lib][op][schema].keys())
    thread_counts = sorted(all_threads)

    w("## Results")
    w("")

    for lib in sorted(grouped.keys()):
        w(f"### {lib}")
        w("")

        for op in sorted(grouped[lib].keys()):
            w(f"#### {op} Performance")
            w("")

            # Latency table
            header = "| Schema |"
            sep = "|--------|"
            for t in thread_counts:
                header += f" {t}T |"
                sep += "-------:|"
            header += " Speedup (1→max) |"
            sep += "----------------:|"
            w(header)
            w(sep)

            for schema in sorted(grouped[lib][op].keys()):
                thread_data = grouped[lib][op][schema]
                row = f"| {schema} |"
                t1 = thread_data.get(1, {}).get('mean', {}).get('time')
                t_max = None
                max_threads = max(thread_counts)

                for t in thread_counts:
                    time = thread_data.get(t, {}).get('mean', {}).get('time')
                    row += f" {format_time(time)} |"
                    if t == max_threads:
                        t_max = time

                if t1 and t_max and t_max > 0:
                    speedup = t1 / t_max
                    row += f" {speedup:.2f}x |"
                else:
                    row += " - |"
                w(row)
            w("")

            # Throughput table
            w("**Throughput:**")
            w("")
            header = "| Schema |"
            sep = "|--------|"
            for t in thread_counts:
                header += f" {t}T |"
                sep += "-------:|"
            w(header)
            w(sep)

            for schema in sorted(grouped[lib][op].keys()):
                thread_data = grouped[lib][op][schema]
                row = f"| {schema} |"
                for t in thread_counts:
                    tput = thread_data.get(t, {}).get('mean', {}).get('throughput')
                    row += f" {format_throughput(tput)} |"
                w(row)
            w("")

    # Scaling analysis
    w("## Scaling Analysis")
    w("")

    for lib in sorted(grouped.keys()):
        for op in sorted(grouped[lib].keys()):
            w(f"### {lib} {op}")
            w("")
            w("| Schema | 1→2T | 1→4T | 1→maxT | Efficiency @ max |")
            w("|--------|-----:|-----:|-------:|-----------------:|")

            max_threads = max(thread_counts)
            for schema in sorted(grouped[lib][op].keys()):
                thread_data = grouped[lib][op][schema]
                t1 = thread_data.get(1, {}).get('mean', {}).get('time')
                t2 = thread_data.get(2, {}).get('mean', {}).get('time')
                t4 = thread_data.get(4, {}).get('mean', {}).get('time')
                t_max = thread_data.get(max_threads, {}).get('mean', {}).get('time')

                s2 = f"{t1/t2:.2f}x" if t1 and t2 else "-"
                s4 = f"{t1/t4:.2f}x" if t1 and t4 else "-"
                s_max = f"{t1/t_max:.2f}x" if t1 and t_max else "-"

                # Efficiency = actual_speedup / ideal_speedup
                if t1 and t_max and t_max > 0:
                    actual = t1 / t_max
                    ideal = max_threads
                    eff = (actual / ideal) * 100
                    eff_str = f"{eff:.1f}%"
                else:
                    eff_str = "-"

                w(f"| {schema} | {s2} | {s4} | {s_max} | {eff_str} |")
            w("")

    # Conclusions
    w("## Conclusions")
    w("")

    conclusion_num = 1
    max_threads = max(thread_counts)

    for lib in sorted(grouped.keys()):
        for op in sorted(grouped[lib].keys()):
            good_scaling = []
            poor_scaling = []

            for schema in grouped[lib][op]:
                thread_data = grouped[lib][op][schema]
                t1 = thread_data.get(1, {}).get('mean', {}).get('time')
                t_max = thread_data.get(max_threads, {}).get('mean', {}).get('time')

                if t1 and t_max and t_max > 0:
                    speedup = t1 / t_max
                    efficiency = speedup / max_threads
                    if efficiency >= 0.5:
                        good_scaling.append((schema, speedup, efficiency))
                    else:
                        poor_scaling.append((schema, speedup, efficiency))

            if good_scaling:
                good_scaling.sort(key=lambda x: -x[1])
                schemas = ', '.join(f"{s} ({sp:.1f}x)" for s, sp, _ in good_scaling[:3])
                w(f"{conclusion_num}. **{lib} {op} scales well**: {schemas}")
                w("")
                conclusion_num += 1

            if poor_scaling:
                poor_scaling.sort(key=lambda x: x[2])
                schemas = ', '.join(f"{s} ({sp:.1f}x, {e*100:.0f}% eff)" for s, sp, e in poor_scaling[:3])
                w(f"{conclusion_num}. **{lib} {op} limited scaling**: {schemas}")
                w("")
                conclusion_num += 1

    # Peak throughput
    peak_throughput = []
    for lib in grouped:
        for op in grouped[lib]:
            for schema in grouped[lib][op]:
                for threads in grouped[lib][op][schema]:
                    tput = grouped[lib][op][schema][threads].get('mean', {}).get('throughput')
                    if tput and tput > 0:
                        peak_throughput.append((lib, op, schema, threads, tput))

    peak_throughput.sort(key=lambda x: -x[4])
    if peak_throughput:
        w(f"{conclusion_num}. **Peak Throughput**:")
        w("")
        for lib, op, schema, threads, tput in peak_throughput[:5]:
            w(f"   - {lib} {op} {schema} @ {threads}T: {format_throughput(tput)}")
        w("")

    w("---")
    w("")
    w("## Appendix: Raw Data")
    w("")
    w("<!--")
    w("RAW_BENCHMARK_DATA_START")
    with open(input_file) as f:
        w(f.read())
    w("RAW_BENCHMARK_DATA_END")
    w("-->")

    with open(output_file, 'w') as f:
        f.write('\n'.join(lines))

    print(f"MT Report generated: {output_file}", file=sys.stderr)


def generate_report(input_file, output_file):
    data = load_benchmarks(input_file)

    if is_mt_benchmark(data):
        return generate_mt_report(input_file, output_file)

    ctx = data['context']
    grouped = group_benchmarks(data)

    libs = sorted(grouped.keys())
    baseline = 'Bebop' if 'Bebop' in libs else libs[0]
    competitors = [l for l in libs if l != baseline]

    lines = []
    w = lines.append

    w("# Serialization Performance Benchmark Report")
    w("")
    w("## Abstract")
    w("")
    w(f"Benchmark comparison of {len(libs)} serialization frameworks: {', '.join(libs)}.")
    w("")

    w("## Experimental Setup")
    w("")
    w("### Hardware Configuration")
    w("")
    w("| Parameter | Value |")
    w("|-----------|-------|")
    w(f"| Machine | {get_machine_info()} |")
    w(f"| Host | {ctx['host_name']} |")
    w(f"| CPU Count | {ctx['num_cpus']} |")
    w(f"| MHz per CPU | {ctx['mhz_per_cpu']} |")
    w(f"| CPU Scaling | {'Enabled' if ctx.get('cpu_scaling_enabled') else 'Disabled'} |")
    w(f"| Build Type | {ctx['library_build_type']} |")
    w(f"| Date | {ctx['date']} |")
    load_avg = ', '.join(f"{x:.2f}" for x in ctx['load_avg'])
    w(f"| Load Average | {load_avg} |")
    w("")

    w("### Cache Hierarchy")
    w("")
    w("| Level | Type | Size |")
    w("|-------|------|------|")
    for cache in ctx.get('caches', []):
        size_kb = cache['size'] // 1024
        w(f"| L{cache['level']} | {cache['type']} | {size_kb}KB |")
    w("")

    reps = data['benchmarks'][0].get('repetitions', 'N/A')
    w("### Methodology")
    w("")
    w(f"Each benchmark ran {reps} times. Statistics: mean, median, stddev, CV. Throughput in bytes/second.")
    w("")

    ops = set()
    for lib in grouped.values():
        ops.update(lib.keys())
    ops = sorted(ops)

    w("## Results")
    w("")

    for op in ops:
        w(f"### {op} Performance")
        w("")

        for lib in libs:
            if op not in grouped[lib]:
                continue

            schemas = sorted(grouped[lib][op].keys())
            if not schemas:
                continue

            w(f"#### {lib}")
            w("")
            w("| Schema | Latency | Throughput |")
            w("|--------|--------:|-----------:|")

            for schema in schemas:
                stats = grouped[lib][op][schema].get('mean', {})
                time = format_time(stats.get('time'))
                tput = format_throughput(stats.get('throughput'))
                w(f"| {schema} | {time} | {tput} |")
            w("")

    w("### Comparative Analysis")
    w("")

    for op in ['Encode', 'Decode']:
        if op not in grouped.get(baseline, {}):
            continue

        active_comps = []
        comp_ops = {}
        for c in competitors:
            if op in grouped.get(c, {}):
                active_comps.append(c)
                comp_ops[c] = op
            elif op == 'Decode' and 'Parse' in grouped.get(c, {}):
                active_comps.append(c)
                comp_ops[c] = 'Parse'

        if not active_comps:
            continue

        w(f"#### {op}: {baseline} vs Others")
        w("")

        header = f"| Schema | {baseline} |"
        sep = "|--------|--------:|"
        for comp in active_comps:
            comp_op = comp_ops[comp]
            label = f"{comp}" if comp_op == op else f"{comp} ({comp_op})"
            header += f" {label} | ratio |"
            sep += "--------:|------:|"
        w(header)
        w(sep)

        all_schemas = set(grouped[baseline][op].keys())
        for comp in active_comps:
            comp_op = comp_ops[comp]
            all_schemas.update(grouped[comp].get(comp_op, {}).keys())

        for schema in sorted(all_schemas):
            base_time = grouped[baseline][op].get(schema, {}).get('mean', {}).get('time')
            row = f"| {schema} | {format_time(base_time)} |"

            for comp in active_comps:
                comp_op = comp_ops[comp]
                comp_time = grouped[comp].get(comp_op, {}).get(schema, {}).get('mean', {}).get('time')
                row += f" {format_time(comp_time)} |"

                if base_time and comp_time and base_time > 0:
                    ratio = comp_time / base_time
                    if ratio >= 1:
                        row += f" {ratio:.1f}x |"
                    else:
                        row += f" **{ratio:.2f}x** |"
                else:
                    row += " - |"

            w(row)
        w("")

    w("### Full Statistics")
    w("")

    for lib in libs:
        w(f"#### {lib}")
        w("")
        w("| Benchmark | Mean | Median | Std Dev | CV | Throughput |")
        w("|-----------|-----:|-------:|--------:|---:|-----------:|")

        for op in sorted(grouped[lib].keys()):
            for schema in sorted(grouped[lib][op].keys()):
                stats = grouped[lib][op][schema]
                mean_t = format_time(stats.get('mean', {}).get('time'))
                median_t = format_time(stats.get('median', {}).get('time'))
                stddev_t = format_time(stats.get('stddev', {}).get('time'))
                cv = format_cv(stats.get('cv', {}).get('time'))
                tput = format_throughput(stats.get('mean', {}).get('throughput'))
                w(f"| {op}_{schema} | {mean_t} | {median_t} | {stddev_t} | {cv} | {tput} |")
        w("")

    w("## Conclusions")
    w("")

    conclusion_num = 1

    for op in ['Encode', 'Decode']:
        if op not in grouped.get(baseline, {}):
            continue

        for comp in competitors:
            comp_op = op
            if op not in grouped.get(comp, {}):
                if op == 'Decode' and 'Parse' in grouped.get(comp, {}):
                    comp_op = 'Parse'
                else:
                    continue

            wins = []
            losses = []

            common_schemas = set(grouped[baseline][op].keys()) & set(grouped[comp][comp_op].keys())

            for schema in common_schemas:
                base_time = grouped[baseline][op][schema].get('mean', {}).get('time')
                comp_time = grouped[comp][comp_op][schema].get('mean', {}).get('time')

                if base_time and comp_time and base_time > 0 and comp_time > 0:
                    ratio = comp_time / base_time
                    if ratio > 1:
                        wins.append((schema, ratio))
                    else:
                        losses.append((schema, ratio))

            total = len(wins) + len(losses)
            if total == 0:
                continue

            wins.sort(key=lambda x: -x[1])
            losses.sort(key=lambda x: x[1])

            vs_label = f"{comp}" if comp_op == op else f"{comp} {comp_op}"
            line = f"{conclusion_num}. **{baseline} {op} vs {vs_label}**: "

            if wins:
                speedups = [r for _, r in wins]
                min_s, max_s = min(speedups), max(speedups)
                line += f"{baseline} faster in {len(wins)}/{total} ({min_s:.1f}x-{max_s:.1f}x). "

                top3 = wins[:3]
                line += f"Best: {', '.join(f'{s} ({r:.1f}x)' for s, r in top3)}. "

            if losses:
                slowdowns = [1/r for _, r in losses]
                line += f"{comp} faster in {len(losses)}/{total}. "
                line += f"Losses: {', '.join(f'{s} ({1/r:.2f}x slower)' for s, r in losses)}."

            w(line)
            w("")
            conclusion_num += 1

    all_cvs = []
    for lib in grouped:
        for op in grouped[lib]:
            for schema in grouped[lib][op]:
                cv = grouped[lib][op][schema].get('cv', {}).get('time')
                if cv is not None:
                    all_cvs.append(cv)

    if all_cvs:
        avg_cv = sum(all_cvs) / len(all_cvs) * 100
        w(f"{conclusion_num}. **Stability**: Mean CV across {len(all_cvs)} benchmarks: {avg_cv:.2f}%.")
        w("")
        conclusion_num += 1

    peak_throughput = []
    for lib in grouped:
        for op in grouped[lib]:
            for schema in grouped[lib][op]:
                tput = grouped[lib][op][schema].get('mean', {}).get('throughput')
                if tput and tput > 0:
                    peak_throughput.append((lib, op, schema, tput))

    peak_throughput.sort(key=lambda x: -x[3])
    if peak_throughput:
        w(f"{conclusion_num}. **Peak Throughput**:")
        w("")
        for lib, op, schema, tput in peak_throughput[:5]:
            w(f"   - {lib} {op} {schema}: {format_throughput(tput)}")
        w("")

    w("---")
    w("")
    w("## Appendix: Raw Data")
    w("")
    w("<!--")
    w("RAW_BENCHMARK_DATA_START")
    with open(input_file) as f:
        w(f.read())
    w("RAW_BENCHMARK_DATA_END")
    w("-->")

    with open(output_file, 'w') as f:
        f.write('\n'.join(lines))

    print(f"Report generated: {output_file}", file=sys.stderr)

if __name__ == '__main__':
    input_file = sys.argv[1] if len(sys.argv) > 1 else 'benchmark_results.json'
    output_file = sys.argv[2] if len(sys.argv) > 2 else 'benchmark_report.md'
    generate_report(input_file, output_file)
