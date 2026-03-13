use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::hint::black_box;
use std::path::PathBuf;
use std::time::Instant;

use bebop_runtime::{BebopDecode, BebopEncode};
use bebop_rust_benchmarks::{benchmark_types as bt, fixtures};
use serde::{Deserialize, Serialize};

// ── JSON schema (matches Google Benchmark output format) ──

#[derive(Serialize, Deserialize)]
struct Context {
  date: String,
  host_name: String,
  executable: String,
  num_cpus: usize,
  library_build_type: String,
  #[serde(flatten)]
  extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
struct BenchmarkRow {
  name: String,
  run_name: String,
  run_type: String,
  repetitions: usize,
  threads: usize,
  aggregate_name: String,
  #[serde(default)]
  aggregate_unit: String,
  iterations: usize,
  real_time: f64,
  cpu_time: f64,
  time_unit: String,
  #[serde(default)]
  bytes_per_second: f64,
  #[serde(default)]
  encoded_size_bytes: usize,
  #[serde(flatten)]
  extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
struct Output {
  context: Context,
  benchmarks: Vec<BenchmarkRow>,
}

// ── Benchmark runner ──

fn bytes_per_second(encoded_size: usize, ns: f64) -> f64 {
  if ns <= 0.0 {
    return 0.0;
  }
  (encoded_size as f64) * 1_000_000_000.0 / ns
}

fn measure_mean_ns(repetitions: usize, iterations: usize, mut f: impl FnMut()) -> f64 {
  let mut samples = Vec::with_capacity(repetitions);
  for _ in 0..repetitions {
    let start = Instant::now();
    for _ in 0..iterations {
      f();
    }
    let elapsed_ns = start.elapsed().as_nanos() as f64;
    samples.push(elapsed_ns / (iterations as f64));
  }
  samples.iter().sum::<f64>() / (samples.len() as f64)
}

fn push_case(
  rows: &mut Vec<BenchmarkRow>,
  scenario: &str,
  encoded_size: usize,
  repetitions: usize,
  iterations: usize,
  mut encode: impl FnMut(),
  mut decode: impl FnMut(),
) {
  let encode_ns = measure_mean_ns(repetitions, iterations, &mut encode);
  let decode_ns = measure_mean_ns(repetitions, iterations, &mut decode);

  for (op, ns) in [("Encode", encode_ns), ("Decode", decode_ns)] {
    rows.push(BenchmarkRow {
      name: format!("BM_Bebop_{op}_{scenario}_mean"),
      run_name: format!("BM_Bebop_{op}_{scenario}"),
      run_type: "aggregate".to_string(),
      repetitions,
      threads: 1,
      aggregate_name: "mean".to_string(),
      aggregate_unit: "time".to_string(),
      iterations,
      real_time: ns,
      cpu_time: ns,
      time_unit: "ns".to_string(),
      bytes_per_second: bytes_per_second(encoded_size, ns),
      encoded_size_bytes: encoded_size,
      extra: serde_json::Map::new(),
    });
  }
}

fn run_benchmarks() -> Output {
  let repetitions = 10usize;
  let mut rows = Vec::new();

  let person_small = fixtures::person_small();
  let person_small_bytes = person_small.to_bytes();
  push_case(
    &mut rows,
    "PersonSmall",
    person_small.encoded_size(),
    repetitions,
    50_000,
    || {
      black_box(person_small.clone().to_bytes());
    },
    || {
      black_box(bt::Person::from_bytes(&person_small_bytes).unwrap());
    },
  );

  let person_medium = fixtures::person_medium();
  let person_medium_bytes = person_medium.to_bytes();
  push_case(
    &mut rows,
    "PersonMedium",
    person_medium.encoded_size(),
    repetitions,
    50_000,
    || {
      black_box(person_medium.clone().to_bytes());
    },
    || {
      black_box(bt::Person::from_bytes(&person_medium_bytes).unwrap());
    },
  );

  let tree = fixtures::tree_deep(64);
  let tree_bytes = tree.to_bytes();
  push_case(
    &mut rows,
    "TreeDeep",
    tree.encoded_size(),
    repetitions,
    2_500,
    || {
      black_box(tree.clone().to_bytes());
    },
    || {
      black_box(bt::TreeNode::from_bytes(&tree_bytes).unwrap());
    },
  );

  let json = fixtures::json_large(3, 3);
  let json_bytes = json.to_bytes();
  push_case(
    &mut rows,
    "JsonLarge",
    json.encoded_size(),
    repetitions,
    750,
    || {
      black_box(json.clone().to_bytes());
    },
    || {
      black_box(bt::JsonValue::from_bytes(&json_bytes).unwrap());
    },
  );

  let doc_small = fixtures::document_small();
  let doc_small_bytes = doc_small.to_bytes();
  push_case(
    &mut rows,
    "DocumentSmall",
    doc_small.encoded_size(),
    repetitions,
    10_000,
    || {
      black_box(doc_small.clone().to_bytes());
    },
    || {
      black_box(bt::Document::from_bytes(&doc_small_bytes).unwrap());
    },
  );

  let doc_large = fixtures::document_large();
  let doc_large_bytes = doc_large.to_bytes();
  push_case(
    &mut rows,
    "DocumentLarge",
    doc_large.encoded_size(),
    repetitions,
    250,
    || {
      black_box(doc_large.clone().to_bytes());
    },
    || {
      black_box(bt::Document::from_bytes(&doc_large_bytes).unwrap());
    },
  );

  let shard = fixtures::tensor_shard_large();
  let shard_bytes = shard.to_bytes();
  push_case(
    &mut rows,
    "TensorShardLarge",
    shard.encoded_size(),
    repetitions,
    200,
    || {
      black_box(shard.clone().to_bytes());
    },
    || {
      black_box(bt::TensorShard::from_bytes(&shard_bytes).unwrap());
    },
  );

  let response = fixtures::inference_response();
  let response_bytes = response.to_bytes();
  push_case(
    &mut rows,
    "InferenceResponse",
    response.encoded_size(),
    repetitions,
    400,
    || {
      black_box(response.clone().to_bytes());
    },
    || {
      black_box(bt::InferenceResponse::from_bytes(&response_bytes).unwrap());
    },
  );

  Output {
    context: Context {
      date: format!(
        "unix:{}",
        std::time::SystemTime::now()
          .duration_since(std::time::UNIX_EPOCH)
          .unwrap()
          .as_secs()
      ),
      host_name: env::var("HOSTNAME").unwrap_or_else(|_| "unknown".to_string()),
      executable: "cargo run --release --bin comparison".to_string(),
      num_cpus: std::thread::available_parallelism()
        .map(|v| v.get())
        .unwrap_or(1),
      library_build_type: "release".to_string(),
      extra: serde_json::Map::new(),
    },
    benchmarks: rows,
  }
}

// ── Report generation (replaces compare.py) ──

struct ScenarioMetrics {
  encode_ns: Option<f64>,
  decode_ns: Option<f64>,
  encode_throughput: Option<f64>,
  decode_throughput: Option<f64>,
  encoded_size: Option<usize>,
}

/// Parse benchmark rows into scenario → metrics, filtering for mean aggregates
/// with the `BM_Bebop_{Encode|Decode}_{Scenario}` naming convention.
fn parse_means(rows: &[BenchmarkRow]) -> BTreeMap<String, ScenarioMetrics> {
  let mut map = BTreeMap::<String, ScenarioMetrics>::new();

  for row in rows {
    if row.aggregate_name != "mean" {
      continue;
    }
    let run_name = &row.run_name;
    if !run_name.starts_with("BM_Bebop_") {
      continue;
    }

    // BM_Bebop_Encode_PersonSmall → op="Encode", scenario="PersonSmall"
    let rest = &run_name["BM_Bebop_".len()..];
    let (op, scenario) = match rest.split_once('_') {
      Some((op, scenario)) => (op, scenario),
      None => continue,
    };

    let entry = map.entry(scenario.to_string()).or_insert(ScenarioMetrics {
      encode_ns: None,
      decode_ns: None,
      encode_throughput: None,
      decode_throughput: None,
      encoded_size: None,
    });

    match op {
      "Encode" => {
        entry.encode_ns = Some(row.real_time);
        entry.encode_throughput = Some(row.bytes_per_second);
        if row.encoded_size_bytes > 0 {
          entry.encoded_size = Some(row.encoded_size_bytes);
        }
      }
      "Decode" => {
        entry.decode_ns = Some(row.real_time);
        entry.decode_throughput = Some(row.bytes_per_second);
        if row.encoded_size_bytes > 0 {
          entry.encoded_size = Some(row.encoded_size_bytes);
        }
      }
      _ => {}
    }
  }

  map
}

fn fmt_ns(v: Option<f64>) -> String {
  match v {
    Some(n) => format!("{n:.2}"),
    None => "-".to_string(),
  }
}

fn fmt_mibs(v: Option<f64>) -> String {
  match v {
    Some(n) => format!("{:.2}", n / (1024.0 * 1024.0)),
    None => "-".to_string(),
  }
}

fn fmt_speedup(rust_ns: Option<f64>, c_ns: Option<f64>) -> String {
  match (rust_ns, c_ns) {
    (Some(r), Some(c)) if r > 0.0 => format!("{:.2}x", c / r),
    _ => "-".to_string(),
  }
}

fn generate_report(rust_json_path: &str, c_json_path: &str, rust: &Output, c: &Output) -> String {
  let rust_map = parse_means(&rust.benchmarks);
  let c_map = parse_means(&c.benchmarks);

  let mut scenarios: Vec<&str> = rust_map
    .keys()
    .chain(c_map.keys())
    .map(|s| s.as_str())
    .collect();
  scenarios.sort_unstable();
  scenarios.dedup();

  let mut lines = Vec::new();
  lines.push("# Rust vs C Bebop Benchmarks".to_string());
  lines.push(String::new());
  lines.push(format!("- Rust source: `{rust_json_path}`"));
  lines.push(format!("- C source: `{c_json_path}`"));
  lines.push(String::new());
  lines.push(
    "| Scenario | Metric | Rust ns | C ns | Rust/C Speedup | Rust MiB/s | C MiB/s | Encoded Size (bytes) |"
      .to_string(),
  );
  lines.push("|---|---:|---:|---:|---:|---:|---:|---:|".to_string());

  let empty = ScenarioMetrics {
    encode_ns: None,
    decode_ns: None,
    encode_throughput: None,
    decode_throughput: None,
    encoded_size: None,
  };

  for scenario in &scenarios {
    let r = rust_map.get(*scenario).unwrap_or(&empty);
    let c = c_map.get(*scenario).unwrap_or(&empty);
    let size = r
      .encoded_size
      .or(c.encoded_size)
      .map(|s| s.to_string())
      .unwrap_or_else(|| "-".to_string());

    lines.push(format!(
      "| {scenario} | encode | {} | {} | {} | {} | {} | {size} |",
      fmt_ns(r.encode_ns),
      fmt_ns(c.encode_ns),
      fmt_speedup(r.encode_ns, c.encode_ns),
      fmt_mibs(r.encode_throughput),
      fmt_mibs(c.encode_throughput),
    ));
    lines.push(format!(
      "| {scenario} | decode | {} | {} | {} | {} | {} | {size} |",
      fmt_ns(r.decode_ns),
      fmt_ns(c.decode_ns),
      fmt_speedup(r.decode_ns, c.decode_ns),
      fmt_mibs(r.decode_throughput),
      fmt_mibs(c.decode_throughput),
    ));
  }

  lines.push(String::new());
  lines.join("\n")
}

// ── CLI ──

struct Args {
  /// Write Rust benchmark JSON here (runs benchmarks).
  out: Option<PathBuf>,
  /// Load pre-existing Rust benchmark JSON (skip benchmark run).
  rust_json: Option<PathBuf>,
  /// Path to C benchmark JSON for comparison.
  c_json: Option<PathBuf>,
  /// Output path for markdown report (requires --c-json). Defaults to stdout.
  report: Option<PathBuf>,
}

fn parse_args() -> Args {
  let mut args = Args {
    out: None,
    rust_json: None,
    c_json: None,
    report: None,
  };

  let mut iter = env::args().skip(1);
  while let Some(arg) = iter.next() {
    match arg.as_str() {
      "--out" => args.out = iter.next().map(PathBuf::from),
      "--rust-json" => args.rust_json = iter.next().map(PathBuf::from),
      "--c-json" => args.c_json = iter.next().map(PathBuf::from),
      "--report" => args.report = iter.next().map(PathBuf::from),
      other => {
        eprintln!("unknown argument: {other}");
        std::process::exit(1);
      }
    }
  }

  if args.out.is_none() && args.rust_json.is_none() {
    eprintln!("usage: comparison --out <path>  (run benchmarks)");
    eprintln!(
      "       comparison --rust-json <path> --c-json <path> --report <path>  (compare only)"
    );
    eprintln!("       comparison --out <path> --c-json <path> --report <path>  (run + compare)");
    std::process::exit(1);
  }

  args
}

fn main() {
  let args = parse_args();

  // Either run benchmarks or load existing results.
  let rust_output = if let Some(ref out_path) = args.out {
    let output = run_benchmarks();
    if let Some(parent) = out_path.parent() {
      fs::create_dir_all(parent).unwrap();
    }
    fs::write(out_path, serde_json::to_vec_pretty(&output).unwrap()).unwrap();
    eprintln!("wrote {}", out_path.display());
    output
  } else if let Some(ref rust_path) = args.rust_json {
    let data =
      fs::read(rust_path).unwrap_or_else(|e| panic!("failed to read {}: {e}", rust_path.display()));
    serde_json::from_slice(&data)
      .unwrap_or_else(|e| panic!("failed to parse {}: {e}", rust_path.display()))
  } else {
    unreachable!("parse_args ensures one of --out or --rust-json is set");
  };

  // Generate comparison report if C results are provided.
  if let Some(ref c_path) = args.c_json {
    let c_data =
      fs::read(c_path).unwrap_or_else(|e| panic!("failed to read {}: {e}", c_path.display()));
    let c_output: Output = serde_json::from_slice(&c_data)
      .unwrap_or_else(|e| panic!("failed to parse {}: {e}", c_path.display()));

    let rust_json_label = args
      .out
      .as_ref()
      .or(args.rust_json.as_ref())
      .map(|p| p.display().to_string())
      .unwrap_or_else(|| "(stdin)".to_string());

    let report = generate_report(
      &rust_json_label,
      &c_path.display().to_string(),
      &rust_output,
      &c_output,
    );

    if let Some(ref report_path) = args.report {
      if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent).unwrap();
      }
      fs::write(report_path, &report).unwrap();
      eprintln!("wrote {}", report_path.display());
    } else {
      print!("{report}");
    }
  }
}
