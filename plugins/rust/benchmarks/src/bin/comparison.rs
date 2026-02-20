use std::env;
use std::fs;
use std::hint::black_box;
use std::path::PathBuf;
use std::time::Instant;

use bebop_runtime::{BebopDecode, BebopEncode};
use bebop_rust_benchmarks::{benchmark_types as bt, fixtures};
use serde::Serialize;

#[derive(Serialize)]
struct Context {
  date: String,
  host_name: String,
  executable: String,
  num_cpus: usize,
  library_build_type: String,
}

#[derive(Serialize)]
struct BenchmarkRow {
  name: String,
  run_name: String,
  run_type: String,
  repetitions: usize,
  threads: usize,
  aggregate_name: String,
  aggregate_unit: String,
  iterations: usize,
  real_time: f64,
  cpu_time: f64,
  time_unit: String,
  bytes_per_second: f64,
  encoded_size_bytes: usize,
}

#[derive(Serialize)]
struct Output {
  context: Context,
  benchmarks: Vec<BenchmarkRow>,
}

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
  let encode_ns = measure_mean_ns(repetitions, iterations, || encode());
  let decode_ns = measure_mean_ns(repetitions, iterations, || decode());

  rows.push(BenchmarkRow {
    name: format!("BM_Bebop_Encode_{}_mean", scenario),
    run_name: format!("BM_Bebop_Encode_{}", scenario),
    run_type: "aggregate".to_string(),
    repetitions,
    threads: 1,
    aggregate_name: "mean".to_string(),
    aggregate_unit: "time".to_string(),
    iterations,
    real_time: encode_ns,
    cpu_time: encode_ns,
    time_unit: "ns".to_string(),
    bytes_per_second: bytes_per_second(encoded_size, encode_ns),
    encoded_size_bytes: encoded_size,
  });

  rows.push(BenchmarkRow {
    name: format!("BM_Bebop_Decode_{}_mean", scenario),
    run_name: format!("BM_Bebop_Decode_{}", scenario),
    run_type: "aggregate".to_string(),
    repetitions,
    threads: 1,
    aggregate_name: "mean".to_string(),
    aggregate_unit: "time".to_string(),
    iterations,
    real_time: decode_ns,
    cpu_time: decode_ns,
    time_unit: "ns".to_string(),
    bytes_per_second: bytes_per_second(encoded_size, decode_ns),
    encoded_size_bytes: encoded_size,
  });
}

fn parse_out_path() -> PathBuf {
  let mut args = env::args().skip(1);
  while let Some(arg) = args.next() {
    if arg == "--out" {
      if let Some(path) = args.next() {
        return PathBuf::from(path);
      }
    }
  }
  PathBuf::from("lab/benchmark/results/rust_benchmark.json")
}

fn main() {
  let out_path = parse_out_path();
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

  let output = Output {
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
    },
    benchmarks: rows,
  };

  if let Some(parent) = out_path.parent() {
    fs::create_dir_all(parent).unwrap();
  }
  fs::write(&out_path, serde_json::to_vec_pretty(&output).unwrap()).unwrap();
  println!("wrote {}", out_path.display());
}
