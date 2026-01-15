#include <benchmark/benchmark.h>

#include "bench_harness.h"

#ifdef BENCH_BEBOP
extern void RegisterBebopBenchmarks();
#endif
#ifdef BENCH_BEBOP_MT
extern void RegisterBebopMTBenchmarks();
#endif
#ifdef BENCH_PROTOBUF
extern void RegisterProtobufBenchmarks();
#endif
#ifdef BENCH_MSGPACK
extern void RegisterMsgpackBenchmarks();
#endif
#ifdef BENCH_SIMDJSON
extern void RegisterSimdjsonBenchmarks();
#endif

int main(int argc, char** argv)
{
#ifdef BENCH_BEBOP
  RegisterBebopBenchmarks();
#endif
#ifdef BENCH_BEBOP_MT
  RegisterBebopMTBenchmarks();
#endif
#ifdef BENCH_PROTOBUF
  RegisterProtobufBenchmarks();
#endif
#ifdef BENCH_MSGPACK
  RegisterMsgpackBenchmarks();
#endif
#ifdef BENCH_SIMDJSON
  RegisterSimdjsonBenchmarks();
#endif

  benchmark::Initialize(&argc, argv);
  if (benchmark::ReportUnrecognizedArguments(argc, argv)) {
    return 1;
  }
  benchmark::RunSpecifiedBenchmarks();
  benchmark::Shutdown();
  return 0;
}
