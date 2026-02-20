use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

use bebop_runtime::{
  bf16, f16, BebopDecode, BebopDuration, BebopEncode, BebopReader, BebopTimestamp, BebopWriter,
  Uuid,
};

use bebop_rust_benchmarks::{benchmark_types as bt, fixtures};

fn bench_scalar_decode_case<T>(
  group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
  name: &str,
  setup: impl Fn(&mut BebopWriter),
  read: impl Fn(&mut BebopReader<'_>) -> T,
) {
  let bytes = {
    let mut w = BebopWriter::new();
    setup(&mut w);
    w.into_bytes()
  };

  group.bench_function(name, |b| {
    b.iter(|| {
      let mut r = BebopReader::new(&bytes);
      black_box(read(&mut r))
    })
  });
}

fn bench_scalar_encode(c: &mut Criterion) {
  let mut g = c.benchmark_group("scalar_encode");

  g.bench_function("byte", |b| {
    b.iter(|| {
      let mut w = BebopWriter::new();
      w.write_byte(black_box(0x7f));
      black_box(w.into_bytes())
    })
  });
  g.bench_function("bool", |b| {
    b.iter(|| {
      let mut w = BebopWriter::new();
      w.write_bool(black_box(true));
      black_box(w.into_bytes())
    })
  });
  g.bench_function("i8", |b| {
    b.iter(|| {
      let mut w = BebopWriter::new();
      w.write_i8(black_box(-7));
      black_box(w.into_bytes())
    })
  });
  g.bench_function("u16", |b| {
    b.iter(|| {
      let mut w = BebopWriter::new();
      w.write_u16(black_box(42));
      black_box(w.into_bytes())
    })
  });
  g.bench_function("i16", |b| {
    b.iter(|| {
      let mut w = BebopWriter::new();
      w.write_i16(black_box(-42));
      black_box(w.into_bytes())
    })
  });
  g.bench_function("u32", |b| {
    b.iter(|| {
      let mut w = BebopWriter::new();
      w.write_u32(black_box(123_456));
      black_box(w.into_bytes())
    })
  });
  g.bench_function("i32", |b| {
    b.iter(|| {
      let mut w = BebopWriter::new();
      w.write_i32(black_box(-123_456));
      black_box(w.into_bytes())
    })
  });
  g.bench_function("u64", |b| {
    b.iter(|| {
      let mut w = BebopWriter::new();
      w.write_u64(black_box(123_456_789_000));
      black_box(w.into_bytes())
    })
  });
  g.bench_function("i64", |b| {
    b.iter(|| {
      let mut w = BebopWriter::new();
      w.write_i64(black_box(-123_456_789_000));
      black_box(w.into_bytes())
    })
  });
  g.bench_function("u128", |b| {
    b.iter(|| {
      let mut w = BebopWriter::new();
      w.write_u128(black_box(123_456_789_000_000_000_000u128));
      black_box(w.into_bytes())
    })
  });
  g.bench_function("i128", |b| {
    b.iter(|| {
      let mut w = BebopWriter::new();
      w.write_i128(black_box(-123_456_789_000_000_000_000i128));
      black_box(w.into_bytes())
    })
  });
  g.bench_function("f16", |b| {
    b.iter(|| {
      let mut w = BebopWriter::new();
      w.write_f16(black_box(f16::from_f32(1.234)));
      black_box(w.into_bytes())
    })
  });
  g.bench_function("bf16", |b| {
    b.iter(|| {
      let mut w = BebopWriter::new();
      w.write_bf16(black_box(bf16::from_f32(2.345)));
      black_box(w.into_bytes())
    })
  });
  g.bench_function("f32", |b| {
    b.iter(|| {
      let mut w = BebopWriter::new();
      w.write_f32(black_box(12.345_f32));
      black_box(w.into_bytes())
    })
  });
  g.bench_function("f64", |b| {
    b.iter(|| {
      let mut w = BebopWriter::new();
      w.write_f64(black_box(123.456_f64));
      black_box(w.into_bytes())
    })
  });
  g.bench_function("string", |b| {
    b.iter(|| {
      let mut w = BebopWriter::new();
      w.write_string(black_box("benchmark-scalar"));
      black_box(w.into_bytes())
    })
  });
  g.bench_function("uuid", |b| {
    let uuid = Uuid::from_bytes([
      0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x10, 0x32, 0x54, 0x76, 0x98, 0xba, 0xdc, 0xfe, 0x11,
      0x22,
    ]);
    b.iter(|| {
      let mut w = BebopWriter::new();
      w.write_uuid(black_box(uuid));
      black_box(w.into_bytes())
    })
  });
  g.bench_function("timestamp", |b| {
    let ts = BebopTimestamp {
      seconds: 1_700_000_000,
      nanos: 123_456_789,
    };
    b.iter(|| {
      let mut w = BebopWriter::new();
      w.write_timestamp(black_box(ts));
      black_box(w.into_bytes())
    })
  });
  g.bench_function("duration", |b| {
    let d = BebopDuration {
      seconds: 12,
      nanos: 345_678_901,
    };
    b.iter(|| {
      let mut w = BebopWriter::new();
      w.write_duration(black_box(d));
      black_box(w.into_bytes())
    })
  });

  g.finish();
}

fn bench_scalar_decode(c: &mut Criterion) {
  let mut g = c.benchmark_group("scalar_decode");

  bench_scalar_decode_case(
    &mut g,
    "byte",
    |w| w.write_byte(0x7f),
    |r| r.read_byte().unwrap(),
  );
  bench_scalar_decode_case(
    &mut g,
    "bool",
    |w| w.write_bool(true),
    |r| r.read_bool().unwrap(),
  );
  bench_scalar_decode_case(&mut g, "i8", |w| w.write_i8(-7), |r| r.read_i8().unwrap());
  bench_scalar_decode_case(
    &mut g,
    "u16",
    |w| w.write_u16(42),
    |r| r.read_u16().unwrap(),
  );
  bench_scalar_decode_case(
    &mut g,
    "i16",
    |w| w.write_i16(-42),
    |r| r.read_i16().unwrap(),
  );
  bench_scalar_decode_case(
    &mut g,
    "u32",
    |w| w.write_u32(123_456),
    |r| r.read_u32().unwrap(),
  );
  bench_scalar_decode_case(
    &mut g,
    "i32",
    |w| w.write_i32(-123_456),
    |r| r.read_i32().unwrap(),
  );
  bench_scalar_decode_case(
    &mut g,
    "u64",
    |w| w.write_u64(123_456_789_000),
    |r| r.read_u64().unwrap(),
  );
  bench_scalar_decode_case(
    &mut g,
    "i64",
    |w| w.write_i64(-123_456_789_000),
    |r| r.read_i64().unwrap(),
  );
  bench_scalar_decode_case(
    &mut g,
    "u128",
    |w| w.write_u128(123_456_789_000_000_000_000u128),
    |r| r.read_u128().unwrap(),
  );
  bench_scalar_decode_case(
    &mut g,
    "i128",
    |w| w.write_i128(-123_456_789_000_000_000_000i128),
    |r| r.read_i128().unwrap(),
  );
  bench_scalar_decode_case(
    &mut g,
    "f16",
    |w| w.write_f16(f16::from_f32(1.234)),
    |r| r.read_f16().unwrap(),
  );
  bench_scalar_decode_case(
    &mut g,
    "bf16",
    |w| w.write_bf16(bf16::from_f32(2.345)),
    |r| r.read_bf16().unwrap(),
  );
  bench_scalar_decode_case(
    &mut g,
    "f32",
    |w| w.write_f32(12.345_f32),
    |r| r.read_f32().unwrap(),
  );
  bench_scalar_decode_case(
    &mut g,
    "f64",
    |w| w.write_f64(123.456_f64),
    |r| r.read_f64().unwrap(),
  );
  bench_scalar_decode_case(
    &mut g,
    "string",
    |w| w.write_string("benchmark-scalar"),
    |r| r.read_string().unwrap(),
  );
  bench_scalar_decode_case(
    &mut g,
    "uuid",
    |w| {
      w.write_uuid(Uuid::from_bytes([
        0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x10, 0x32, 0x54, 0x76, 0x98, 0xba, 0xdc, 0xfe, 0x11,
        0x22,
      ]));
    },
    |r| r.read_uuid().unwrap(),
  );
  bench_scalar_decode_case(
    &mut g,
    "timestamp",
    |w| {
      w.write_timestamp(BebopTimestamp {
        seconds: 1_700_000_000,
        nanos: 123_456_789,
      });
    },
    |r| r.read_timestamp().unwrap(),
  );
  bench_scalar_decode_case(
    &mut g,
    "duration",
    |w| {
      w.write_duration(BebopDuration {
        seconds: 12,
        nanos: 345_678_901,
      });
    },
    |r| r.read_duration().unwrap(),
  );

  g.finish();
}

fn bench_structs_and_messages(c: &mut Criterion) {
  let person = fixtures::person_small();
  let person_bytes = person.to_bytes();
  let mut g = c.benchmark_group("Person");
  g.throughput(Throughput::Bytes(person.encoded_size() as u64));
  g.bench_function("encode", |b| {
    b.iter(|| black_box(person.clone()).to_bytes())
  });
  g.bench_function("decode", |b| {
    b.iter(|| black_box(bt::Person::from_bytes(&person_bytes).unwrap()))
  });
  g.bench_function("encoded_size", |b| {
    b.iter(|| black_box(person.encoded_size()))
  });
  g.bench_function("into_owned", |b| {
    b.iter(|| {
      let decoded = bt::Person::from_bytes(&person_bytes).unwrap();
      black_box(decoded.into_owned())
    })
  });
  g.finish();

  let span = fixtures::text_span();
  let span_bytes = span.to_bytes();
  let mut g = c.benchmark_group("TextSpan");
  g.throughput(Throughput::Bytes(span.encoded_size() as u64));
  g.bench_function("encode", |b| b.iter(|| black_box(span.clone()).to_bytes()));
  g.bench_function("decode", |b| {
    b.iter(|| black_box(bt::TextSpan::from_bytes(&span_bytes).unwrap()))
  });
  g.bench_function("encoded_size", |b| {
    b.iter(|| black_box(span.encoded_size()))
  });
  g.finish();

  let embedding = fixtures::embedding_bf16_1536();
  let embedding_bytes = embedding.to_bytes();
  let mut g = c.benchmark_group("EmbeddingBf16");
  g.throughput(Throughput::Bytes(embedding.encoded_size() as u64));
  g.bench_function("encode", |b| {
    b.iter(|| black_box(embedding.clone()).to_bytes())
  });
  g.bench_function("decode", |b| {
    b.iter(|| black_box(bt::EmbeddingBf16::from_bytes(&embedding_bytes).unwrap()))
  });
  g.bench_function("encoded_size", |b| {
    b.iter(|| black_box(embedding.encoded_size()))
  });
  g.finish();

  let tree = fixtures::tree_deep(32);
  let tree_bytes = tree.to_bytes();
  let mut g = c.benchmark_group("TreeNode");
  g.throughput(Throughput::Bytes(tree.encoded_size() as u64));
  g.bench_function("encode", |b| b.iter(|| black_box(tree.clone()).to_bytes()));
  g.bench_function("decode", |b| {
    b.iter(|| black_box(bt::TreeNode::from_bytes(&tree_bytes).unwrap()))
  });
  g.bench_function("encoded_size", |b| {
    b.iter(|| black_box(tree.encoded_size()))
  });
  g.finish();

  let json = fixtures::json_large(3, 3);
  let json_bytes = json.to_bytes();
  let mut g = c.benchmark_group("JsonValue");
  g.throughput(Throughput::Bytes(json.encoded_size() as u64));
  g.bench_function("encode", |b| b.iter(|| black_box(json.clone()).to_bytes()));
  g.bench_function("decode", |b| {
    b.iter(|| black_box(bt::JsonValue::from_bytes(&json_bytes).unwrap()))
  });
  g.bench_function("encoded_size", |b| {
    b.iter(|| black_box(json.encoded_size()))
  });
  g.bench_function("into_owned", |b| {
    b.iter(|| {
      let decoded = bt::JsonValue::from_bytes(&json_bytes).unwrap();
      black_box(decoded.into_owned())
    })
  });
  g.finish();

  let document = fixtures::document_large();
  let document_bytes = document.to_bytes();
  let mut g = c.benchmark_group("Document");
  g.throughput(Throughput::Bytes(document.encoded_size() as u64));
  g.bench_function("encode", |b| {
    b.iter(|| black_box(document.clone()).to_bytes())
  });
  g.bench_function("decode", |b| {
    b.iter(|| black_box(bt::Document::from_bytes(&document_bytes).unwrap()))
  });
  g.bench_function("encoded_size", |b| {
    b.iter(|| black_box(document.encoded_size()))
  });
  g.bench_function("into_owned", |b| {
    b.iter(|| {
      let decoded = bt::Document::from_bytes(&document_bytes).unwrap();
      black_box(decoded.into_owned())
    })
  });
  g.finish();

  let shard = fixtures::tensor_shard_large();
  let shard_bytes = shard.to_bytes();
  let mut g = c.benchmark_group("TensorShard");
  g.throughput(Throughput::Bytes(shard.encoded_size() as u64));
  g.bench_function("encode", |b| b.iter(|| black_box(shard.clone()).to_bytes()));
  g.bench_function("decode", |b| {
    b.iter(|| black_box(bt::TensorShard::from_bytes(&shard_bytes).unwrap()))
  });
  g.bench_function("encoded_size", |b| {
    b.iter(|| black_box(shard.encoded_size()))
  });
  g.bench_function("into_owned", |b| {
    b.iter(|| {
      let decoded = bt::TensorShard::from_bytes(&shard_bytes).unwrap();
      black_box(decoded.into_owned())
    })
  });
  g.finish();

  let response = fixtures::inference_response();
  let response_bytes = response.to_bytes();
  let mut g = c.benchmark_group("InferenceResponse");
  g.throughput(Throughput::Bytes(response.encoded_size() as u64));
  g.bench_function("encode", |b| {
    b.iter(|| black_box(response.clone()).to_bytes())
  });
  g.bench_function("decode", |b| {
    b.iter(|| black_box(bt::InferenceResponse::from_bytes(&response_bytes).unwrap()))
  });
  g.bench_function("encoded_size", |b| {
    b.iter(|| black_box(response.encoded_size()))
  });
  g.finish();
}

criterion_group!(
  benches,
  bench_scalar_encode,
  bench_scalar_decode,
  bench_structs_and_messages
);
criterion_main!(benches);
