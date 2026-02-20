#!/usr/bin/env python3
import argparse
import json
from pathlib import Path


def parse_bebop_means(rows):
    out = {}
    for row in rows:
        if row.get("aggregate_name") != "mean":
            continue
        run_name = row.get("run_name", "")
        if not run_name.startswith("BM_Bebop_"):
            continue
        parts = run_name.split("_", 3)
        if len(parts) != 4:
            continue
        op = parts[2].lower()  # encode|decode
        scenario = parts[3]
        entry = out.setdefault(scenario, {})
        entry[f"{op}_ns"] = float(row.get("real_time", 0.0))
        entry[f"{op}_throughput"] = float(row.get("bytes_per_second", 0.0))
        if "encoded_size_bytes" in row:
            entry["encoded_size_bytes"] = int(row["encoded_size_bytes"])
    return out


def fmt_ns(v):
    return f"{v:.2f}" if v is not None else "-"


def fmt_mibs(v):
    return f"{(v / (1024 * 1024)):.2f}" if v is not None else "-"


def fmt_speedup(rust_ns, c_ns):
    if not rust_ns or not c_ns:
        return "-"
    return f"{(c_ns / rust_ns):.2f}x"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--c-json", required=True)
    parser.add_argument("--rust-json", required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()

    c_data = json.loads(Path(args.c_json).read_text())
    rust_data = json.loads(Path(args.rust_json).read_text())

    c_map = parse_bebop_means(c_data.get("benchmarks", []))
    rust_map = parse_bebop_means(rust_data.get("benchmarks", []))

    scenarios = sorted(set(c_map.keys()) | set(rust_map.keys()))

    lines = []
    lines.append("# Rust vs C Bebop Benchmarks")
    lines.append("")
    lines.append(f"- Rust source: `{args.rust_json}`")
    lines.append(f"- C source: `{args.c_json}`")
    lines.append("")
    lines.append("| Scenario | Metric | Rust ns | C ns | Rust/C Speedup | Rust MiB/s | C MiB/s | Encoded Size (bytes) |")
    lines.append("|---|---:|---:|---:|---:|---:|---:|---:|")

    for scenario in scenarios:
        r = rust_map.get(scenario, {})
        c = c_map.get(scenario, {})
        size = r.get("encoded_size_bytes", "-")

        lines.append(
            "| {s} | encode | {r_ns} | {c_ns} | {sp} | {r_tp} | {c_tp} | {sz} |".format(
                s=scenario,
                r_ns=fmt_ns(r.get("encode_ns")),
                c_ns=fmt_ns(c.get("encode_ns")),
                sp=fmt_speedup(r.get("encode_ns"), c.get("encode_ns")),
                r_tp=fmt_mibs(r.get("encode_throughput")),
                c_tp=fmt_mibs(c.get("encode_throughput")),
                sz=size,
            )
        )
        lines.append(
            "| {s} | decode | {r_ns} | {c_ns} | {sp} | {r_tp} | {c_tp} | {sz} |".format(
                s=scenario,
                r_ns=fmt_ns(r.get("decode_ns")),
                c_ns=fmt_ns(c.get("decode_ns")),
                sp=fmt_speedup(r.get("decode_ns"), c.get("decode_ns")),
                r_tp=fmt_mibs(r.get("decode_throughput")),
                c_tp=fmt_mibs(c.get("decode_throughput")),
                sz=size,
            )
        )

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
