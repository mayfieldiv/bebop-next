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

/// Register a benchmark scenario. Builds fixture once, then measures
/// encode and decode in `push_case`. Each new scenario is a single call.
macro_rules! bench {
  ($rows:expr, $reps:expr, $name:literal, $iters:expr, $ty:ty, $fixture:expr) => {{
    let val = $fixture;
    let bytes = val.to_bytes();
    push_case(
      $rows,
      $name,
      val.encoded_size(),
      $reps,
      $iters,
      || {
        black_box(val.to_bytes());
      },
      || {
        black_box(<$ty>::from_bytes(&bytes).unwrap());
      },
    );
  }};
}

fn run_benchmarks() -> Output {
  let reps = 10usize;
  let mut rows = Vec::new();

  bench!(
    &mut rows,
    reps,
    "PersonSmall",
    50_000,
    bt::Person,
    fixtures::person_small()
  );
  bench!(
    &mut rows,
    reps,
    "PersonMedium",
    50_000,
    bt::Person,
    fixtures::person_medium()
  );
  bench!(
    &mut rows,
    reps,
    "OrderSmall",
    50_000,
    bt::Order,
    fixtures::order_small()
  );
  bench!(
    &mut rows,
    reps,
    "OrderLarge",
    10_000,
    bt::Order,
    fixtures::order_large()
  );
  bench!(
    &mut rows,
    reps,
    "EventSmall",
    50_000,
    bt::Event,
    fixtures::event_small()
  );
  bench!(
    &mut rows,
    reps,
    "EventLarge",
    10_000,
    bt::Event,
    fixtures::event_large()
  );
  bench!(
    &mut rows,
    reps,
    "TreeWide",
    5_000,
    bt::TreeNode,
    fixtures::tree_wide(100)
  );
  bench!(
    &mut rows,
    reps,
    "TreeDeep",
    2_500,
    bt::TreeNode,
    fixtures::tree_deep(64)
  );
  bench!(
    &mut rows,
    reps,
    "JsonSmall",
    50_000,
    bt::JsonValue,
    fixtures::json_object_small()
  );
  bench!(
    &mut rows,
    reps,
    "JsonLarge",
    750,
    bt::JsonValue,
    fixtures::json_large(3, 3)
  );
  bench!(
    &mut rows,
    reps,
    "DocumentSmall",
    10_000,
    bt::Document,
    fixtures::document_small()
  );
  bench!(
    &mut rows,
    reps,
    "DocumentLarge",
    250,
    bt::Document,
    fixtures::document_large()
  );
  bench!(
    &mut rows,
    reps,
    "ChunkedText",
    2_500,
    bt::ChunkedText,
    fixtures::chunked_text()
  );
  bench!(
    &mut rows,
    reps,
    "Embedding384",
    20_000,
    bt::EmbeddingBf16,
    fixtures::embedding_bf16(384)
  );
  bench!(
    &mut rows,
    reps,
    "Embedding768",
    10_000,
    bt::EmbeddingBf16,
    fixtures::embedding_bf16(768)
  );
  bench!(
    &mut rows,
    reps,
    "Embedding1536",
    10_000,
    bt::EmbeddingBf16,
    fixtures::embedding_bf16(1536)
  );
  bench!(
    &mut rows,
    reps,
    "EmbeddingF32_768",
    10_000,
    bt::EmbeddingF32,
    fixtures::embedding_f32(768)
  );
  bench!(
    &mut rows,
    reps,
    "EmbeddingBatch",
    2_000,
    bt::EmbeddingBatch,
    fixtures::embedding_batch()
  );
  bench!(
    &mut rows,
    reps,
    "LLMChunkSmall",
    50_000,
    bt::LlmStreamChunk,
    fixtures::llm_chunk_small()
  );
  bench!(
    &mut rows,
    reps,
    "LLMChunkLarge",
    2_000,
    bt::LlmStreamChunk,
    fixtures::llm_chunk_large()
  );
  bench!(
    &mut rows,
    reps,
    "TensorShardSmall",
    10_000,
    bt::TensorShard,
    fixtures::tensor_shard_small()
  );
  bench!(
    &mut rows,
    reps,
    "TensorShardLarge",
    200,
    bt::TensorShard,
    fixtures::tensor_shard_large()
  );
  bench!(
    &mut rows,
    reps,
    "InferenceResponse",
    400,
    bt::InferenceResponse,
    fixtures::inference_response()
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

// ── Report generation ──

#[derive(Default)]
struct ScenarioMetrics {
  encode_ns: Option<f64>,
  decode_ns: Option<f64>,
  encode_throughput: Option<f64>,
  decode_throughput: Option<f64>,
  encoded_size: Option<usize>,
}

/// Parse benchmark rows into scenario -> metrics, filtering for mean aggregates
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

    // BM_Bebop_Encode_PersonSmall -> op="Encode", scenario="PersonSmall"
    let rest = &run_name["BM_Bebop_".len()..];
    let (op, scenario) = match rest.split_once('_') {
      Some((op, scenario)) => (op, scenario),
      None => continue,
    };

    let entry = map.entry(scenario.to_string()).or_default();

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

/// A named set of parsed benchmark results.
struct LangResults {
  name: String,
  path: String,
  metrics: BTreeMap<String, ScenarioMetrics>,
}

fn fmt_ns(v: Option<f64>) -> String {
  match v {
    Some(n) => format!("{n:.2}"),
    None => "-".to_string(),
  }
}

/// Format as multiple of baseline (1.0x = parity, 3.0x = takes 3x longer).
fn fmt_multiple(lang_ns: Option<f64>, baseline_ns: Option<f64>) -> String {
  match (lang_ns, baseline_ns) {
    (Some(l), Some(b)) if b > 0.0 => format!("{:.1}x", l / b),
    _ => "-".to_string(),
  }
}

fn generate_report(langs: &[LangResults], baseline: &str) -> String {
  // Collect all scenario names across all languages.
  let mut scenarios = BTreeMap::<&str, ()>::new();
  for lang in langs {
    for key in lang.metrics.keys() {
      scenarios.insert(key.as_str(), ());
    }
  }

  // Find the baseline index.
  let baseline_idx = langs
    .iter()
    .position(|l| l.name == baseline)
    .unwrap_or_else(|| {
      let names: Vec<_> = langs.iter().map(|l| l.name.as_str()).collect();
      panic!("baseline {baseline:?} not found in languages: {names:?}");
    });

  let empty = ScenarioMetrics::default();

  // Build all cell values first, then compute column widths for alignment.
  // Column layout: Scenario | Metric | <baseline> ns | [<lang> ns | vs <baseline>]... | Size
  let mut headers: Vec<String> = vec![
    "Scenario".to_string(),
    "Metric".to_string(),
    format!("{} ns", langs[baseline_idx].name),
  ];
  let mut right_align = vec![false, false, true]; // right-align numeric columns
  for (i, lang) in langs.iter().enumerate() {
    if i == baseline_idx {
      continue;
    }
    headers.push(format!("{} ns", lang.name));
    headers.push(format!("vs {}", baseline));
    right_align.push(true);
    right_align.push(true);
  }
  headers.push("Size".to_string());
  right_align.push(true);

  let num_cols = headers.len();

  // Build data rows.
  let mut data_rows: Vec<Vec<String>> = Vec::new();
  for scenario in scenarios.keys() {
    for metric in ["encode", "decode"] {
      let get_ns = |m: &ScenarioMetrics| -> Option<f64> {
        if metric == "encode" {
          m.encode_ns
        } else {
          m.decode_ns
        }
      };

      let baseline_m = langs[baseline_idx].metrics.get(*scenario).unwrap_or(&empty);
      let baseline_ns = get_ns(baseline_m);

      let size = langs
        .iter()
        .filter_map(|l| l.metrics.get(*scenario).and_then(|m| m.encoded_size))
        .next()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "-".to_string());

      let mut cells: Vec<String> = vec![
        scenario.to_string(),
        metric.to_string(),
        fmt_ns(baseline_ns),
      ];
      for (i, lang) in langs.iter().enumerate() {
        if i == baseline_idx {
          continue;
        }
        let m = lang.metrics.get(*scenario).unwrap_or(&empty);
        let ns = get_ns(m);
        cells.push(fmt_ns(ns));
        cells.push(fmt_multiple(ns, baseline_ns));
      }
      cells.push(size);
      data_rows.push(cells);
    }
  }

  // Compute column widths.
  let mut widths = vec![0usize; num_cols];
  for (i, h) in headers.iter().enumerate() {
    widths[i] = widths[i].max(h.len());
  }
  for row in &data_rows {
    for (i, cell) in row.iter().enumerate() {
      widths[i] = widths[i].max(cell.len());
    }
  }

  // Format a row with padding.
  let fmt_row = |cells: &[String]| -> String {
    let mut parts = Vec::with_capacity(cells.len());
    for (i, cell) in cells.iter().enumerate() {
      let w = widths[i];
      if right_align[i] {
        parts.push(format!("{cell:>w$}"));
      } else {
        parts.push(format!("{cell:<w$}"));
      }
    }
    format!("| {} |", parts.join(" | "))
  };

  let mut lines = Vec::new();

  // Title.
  let names: Vec<_> = langs.iter().map(|l| l.name.as_str()).collect();
  lines.push(format!("# Bebop Benchmarks: {}", names.join(" vs ")));
  lines.push(String::new());
  for lang in langs {
    lines.push(format!("- {}: `{}`", lang.name, lang.path));
  }
  lines.push(format!("- baseline: {baseline}"));
  lines.push(String::new());

  // Header row.
  lines.push(fmt_row(&headers));

  // Separator row.
  let sep_cells: Vec<String> = (0..num_cols)
    .map(|i| {
      let w = widths[i];
      if right_align[i] {
        format!("{:-<width$}:", "", width = w.saturating_sub(1))
      } else {
        format!("{:-<width$}", "", width = w)
      }
    })
    .collect();
  lines.push(format!("| {} |", sep_cells.join(" | ")));

  // Data rows.
  for row in &data_rows {
    lines.push(fmt_row(row));
  }

  lines.push(String::new());
  lines.join("\n")
}

// ── CLI ──

/// Repo root, anchored at compile time via CARGO_MANIFEST_DIR.
/// benchmarks/Cargo.toml is at plugins/rust/benchmarks/, so repo root is ../../..
const REPO_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../..");

struct Args {
  /// Run Rust benchmarks and write JSON here.
  out: Option<PathBuf>,
  /// Language result files: Vec<(name, path)>. Insertion order preserved.
  langs: Vec<(String, PathBuf)>,
  /// Baseline language name for "vs" column. Defaults to first --lang.
  baseline: Option<String>,
  /// Output path for markdown report. Defaults to stdout.
  report: Option<PathBuf>,
}

fn print_usage() -> ! {
  eprintln!("usage:");
  eprintln!("  comparison                          (run Rust benchmarks, compare vs C snapshot)");
  eprintln!("  comparison --out <path> [--lang name=path]... [--baseline name] [--report path]");
  eprintln!(
    "  comparison --lang name=path [--lang name=path]... [--baseline name] [--report path]"
  );
  eprintln!();
  eprintln!("  (no args)           Run Rust benchmarks, compare against checked-in C snapshot");
  eprintln!("  --out <path>        Run Rust benchmarks, write JSON (adds 'rust' to comparison)");
  eprintln!("  --lang name=path    Add a language's benchmark JSON to the comparison");
  eprintln!(
    "  --baseline name     Which language is the baseline for 'vs' column (default: first --lang)"
  );
  eprintln!("  --report <path>     Write markdown report to file (default: stdout)");
  std::process::exit(1);
}

fn parse_lang_arg(s: &str) -> (String, PathBuf) {
  match s.split_once('=') {
    Some((name, path)) if !name.is_empty() && !path.is_empty() => {
      (name.to_string(), PathBuf::from(path))
    }
    _ => {
      eprintln!("invalid --lang format: {s:?} (expected name=path)");
      std::process::exit(1);
    }
  }
}

fn parse_args() -> Args {
  let mut args = Args {
    out: None,
    langs: Vec::new(),
    baseline: None,
    report: None,
  };

  let mut iter = env::args().skip(1);
  while let Some(arg) = iter.next() {
    match arg.as_str() {
      "--out" => args.out = iter.next().map(PathBuf::from),
      "--lang" => {
        if let Some(val) = iter.next() {
          args.langs.push(parse_lang_arg(&val));
        }
      }
      "--baseline" => args.baseline = iter.next(),
      "--report" => args.report = iter.next().map(PathBuf::from),
      "--help" | "-h" => print_usage(),
      other => {
        eprintln!("unknown argument: {other}");
        print_usage();
      }
    }
  }

  args
}

fn load_output(path: &PathBuf) -> Output {
  let data = fs::read(path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
  serde_json::from_slice(&data)
    .unwrap_or_else(|e| panic!("failed to parse {}: {e}", path.display()))
}

/// Find the checked-in C benchmark snapshot.
fn find_c_snapshot() -> Option<PathBuf> {
  let candidates = [
    format!("{REPO_ROOT}/lab/benchmark/results/c_benchmark.json"),
    format!("{REPO_ROOT}/lab/benchmark/c/benchmark_results_st_3.json"),
  ];
  candidates
    .into_iter()
    .map(PathBuf::from)
    .find(|p| p.exists())
}

fn main() {
  let args = parse_args();

  // Default mode: no args → run benchmarks, compare against C snapshot, print to stdout.
  let default_mode = args.out.is_none() && args.langs.is_empty();

  let mut lang_results: Vec<LangResults> = Vec::new();

  // In default mode, load C snapshot first so it becomes the default baseline.
  if default_mode {
    if let Some(c_path) = find_c_snapshot() {
      let output = load_output(&c_path);
      lang_results.push(LangResults {
        name: "c".to_string(),
        path: "(snapshot)".to_string(),
        metrics: parse_means(&output.benchmarks),
      });
    } else {
      eprintln!("warning: no C benchmark snapshot found, showing Rust-only results");
    }
  }

  // Run Rust benchmarks if --out is given or in default mode.
  if args.out.is_some() || default_mode {
    let output = run_benchmarks();

    if let Some(ref out_path) = args.out {
      if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).unwrap();
      }
      fs::write(out_path, serde_json::to_vec_pretty(&output).unwrap()).unwrap();
      eprintln!("wrote {}", out_path.display());
    }

    let path_label = args
      .out
      .as_ref()
      .map(|p| p.display().to_string())
      .unwrap_or_else(|| "(live run)".to_string());

    lang_results.push(LangResults {
      name: "rust".to_string(),
      path: path_label,
      metrics: parse_means(&output.benchmarks),
    });
  }

  // Load each --lang file.
  for (name, path) in &args.langs {
    let output = load_output(path);
    lang_results.push(LangResults {
      name: name.clone(),
      path: path.display().to_string(),
      metrics: parse_means(&output.benchmarks),
    });
  }

  // Generate report if we have at least 2 languages.
  if lang_results.len() >= 2 {
    let baseline = args
      .baseline
      .as_deref()
      .unwrap_or_else(|| &lang_results[0].name);

    let report = generate_report(&lang_results, baseline);

    if let Some(ref report_path) = args.report {
      if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent).unwrap();
      }
      fs::write(report_path, &report).unwrap();
      eprintln!("wrote {}", report_path.display());
    } else {
      print!("{report}");
    }
  } else if lang_results.len() == 1 {
    eprintln!("only one language loaded — nothing to compare");
  }
}
