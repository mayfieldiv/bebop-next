#include "bebop.c"
#include "test_common.h"
#include "unity.h"

static bebop_context_t* ctx;

void test_service_basic_parsing(void);
void test_service_single_mixin(void);
void test_service_multiple_mixins(void);
void test_service_mixin_reference_tracking(void);
void test_service_mixin_not_service_error(void);
void test_service_mixin_method_conflict_error(void);
void test_service_mixin_mixin_conflict_error(void);
void test_service_chained_mixins(void);
void test_service_empty_body_with_mixin(void);
void test_service_streaming_with_mixin(void);
void test_service_mixin_at_bounds(void);
void test_service_mixin_count_non_service(void);

static const char* test_include_paths[] = {BEBOP_TEST_FIXTURES_DIR "/valid", BEBOP_STD_DIR};

void setUp(void)
{
  bebop_host_t host = test_host(test_include_paths, 2);
  ctx = bebop_context_create(&host);
  TEST_ASSERT_NOT_NULL(ctx);
}

void tearDown(void)
{
  if (ctx) {
    bebop_context_destroy(ctx);
    ctx = NULL;
  }
}

static void render_diagnostics(bebop_parse_result_t* result)
{
  uint32_t count = bebop_result_diagnostic_count(result);
  if (count == 0) {
    return;
  }

  fprintf(stderr, "\n=== %u Diagnostic(s) ===\n", count);
  for (uint32_t i = 0; i < count; i++) {
    const bebop_diagnostic_t* diag = bebop_result_diagnostic_at(result, i);
    bebop_diag_severity_t sev = bebop_diagnostic_severity(diag);
    const char* msg = bebop_diagnostic_message(diag);
    const char* path = bebop_diagnostic_path(diag);
    bebop_span_t span = bebop_diagnostic_span(diag);

    fprintf(
        stderr,
        "[%s] %s:%u:%u: %s\n",
        bebop_diag_severity_name(sev),
        path ? path : "<unknown>",
        span.start_line,
        span.start_col,
        msg ? msg : "<no message>"
    );
  }
  fprintf(stderr, "========================\n");
}

static bebop_parse_result_t* parse_expect_success(const char* source)
{
  bebop_parse_result_t* result = NULL;
  bebop_status_t status = bebop_parse_source(ctx, BEBOP_SOURCE(source, "test.bop"), &result);

  if (status != BEBOP_OK || bebop_result_error_count(result) > 0) {
    fprintf(stderr, "\nUnexpected parse failure (status=%d):\n", status);
    render_diagnostics(result);
    TEST_FAIL_MESSAGE("Parse failed unexpectedly - see diagnostics above");
  }

  TEST_ASSERT_NOT_NULL(result);
  TEST_ASSERT_TRUE(bebop_result_schema_count(result) >= 1);
  return result;
}

static bebop_parse_result_t* parse_expect_error(const char* source)
{
  bebop_parse_result_t* result = NULL;
  bebop_status_t status = bebop_parse_source(ctx, BEBOP_SOURCE(source, "test.bop"), &result);
  BEBOP_UNUSED(status);
  TEST_ASSERT_NOT_NULL(result);
  TEST_ASSERT_TRUE(bebop_result_error_count(result) > 0);
  return result;
}

static const bebop_def_t* find_def(bebop_parse_result_t* result, const char* name)
{
  const bebop_schema_t* schema = bebop_result_schema_at(result, 0);
  uint32_t count = bebop_schema_definition_count(schema);
  for (uint32_t i = 0; i < count; i++) {
    const bebop_def_t* def = bebop_schema_definition_at(schema, i);
    const char* def_name = bebop_def_name(def);
    if (def_name && strcmp(def_name, name) == 0) {
      return def;
    }
  }
  return NULL;
}

// Basic service parsing
void test_service_basic_parsing(void)
{
  const char* source =
      "struct Request { data: string; }\n"
      "struct Response { result: int32; }\n"
      "service TestService {\n"
      "  getData(Request): Response;\n"
      "  ping(Request): Response;\n"
      "}\n";

  bebop_parse_result_t* result = parse_expect_success(source);

  const bebop_def_t* service = find_def(result, "TestService");
  TEST_ASSERT_NOT_NULL(service);
  TEST_ASSERT_EQUAL(BEBOP_DEF_SERVICE, bebop_def_kind(service));
  TEST_ASSERT_EQUAL(2, bebop_def_method_count(service));

  const bebop_method_t* m0 = bebop_def_method_at(service, 0);
  TEST_ASSERT_NOT_NULL(m0);
  TEST_ASSERT_EQUAL_STRING("getData", bebop_method_name(m0));
  TEST_ASSERT_NOT_NULL(bebop_method_request_type(m0));
  TEST_ASSERT_NOT_NULL(bebop_method_response_type(m0));

  const bebop_method_t* m1 = bebop_def_method_at(service, 1);
  TEST_ASSERT_NOT_NULL(m1);
  TEST_ASSERT_EQUAL_STRING("ping", bebop_method_name(m1));

  BEBOP_UNUSED(result);
}

// Service with single mixin
void test_service_single_mixin(void)
{
  const char* source =
      "struct Req { data: string; }\n"
      "struct Res { ok: bool; }\n"
      "service BaseService {\n"
      "  base(Req): Res;\n"
      "}\n"
      "service DerivedService with BaseService {\n"
      "  derived(Req): Res;\n"
      "}\n";

  bebop_parse_result_t* result = parse_expect_success(source);

  const bebop_def_t* base = find_def(result, "BaseService");
  TEST_ASSERT_NOT_NULL(base);
  TEST_ASSERT_EQUAL(0, bebop_def_mixin_count(base));

  const bebop_def_t* derived = find_def(result, "DerivedService");
  TEST_ASSERT_NOT_NULL(derived);
  TEST_ASSERT_EQUAL(1, bebop_def_mixin_count(derived));

  const bebop_type_t* mixin_type = bebop_def_mixin_at(derived, 0);
  TEST_ASSERT_NOT_NULL(mixin_type);
  TEST_ASSERT_EQUAL(BEBOP_TYPE_DEFINED, bebop_type_kind(mixin_type));

  const bebop_def_t* resolved = bebop_type_resolved(mixin_type);
  TEST_ASSERT_NOT_NULL(resolved);
  TEST_ASSERT_EQUAL_STRING("BaseService", bebop_def_name(resolved));

  BEBOP_UNUSED(result);
}

// Service with multiple mixins
void test_service_multiple_mixins(void)
{
  const char* source =
      "struct Req { data: string; }\n"
      "struct Res { ok: bool; }\n"
      "service ServiceA {\n"
      "  methodA(Req): Res;\n"
      "}\n"
      "service ServiceB {\n"
      "  methodB(Req): Res;\n"
      "}\n"
      "service ServiceC {\n"
      "  methodC(Req): Res;\n"
      "}\n"
      "service Combined with ServiceA, ServiceB, ServiceC {\n"
      "  methodOwn(Req): Res;\n"
      "}\n";

  bebop_parse_result_t* result = parse_expect_success(source);

  const bebop_def_t* combined = find_def(result, "Combined");
  TEST_ASSERT_NOT_NULL(combined);
  TEST_ASSERT_EQUAL(3, bebop_def_mixin_count(combined));
  TEST_ASSERT_EQUAL(1, bebop_def_method_count(combined));

  const bebop_type_t* m0 = bebop_def_mixin_at(combined, 0);
  const bebop_type_t* m1 = bebop_def_mixin_at(combined, 1);
  const bebop_type_t* m2 = bebop_def_mixin_at(combined, 2);

  TEST_ASSERT_EQUAL_STRING("ServiceA", bebop_def_name(bebop_type_resolved(m0)));
  TEST_ASSERT_EQUAL_STRING("ServiceB", bebop_def_name(bebop_type_resolved(m1)));
  TEST_ASSERT_EQUAL_STRING("ServiceC", bebop_def_name(bebop_type_resolved(m2)));

  BEBOP_UNUSED(result);
}

// Mixin reference tracking - mixin types should have references
void test_service_mixin_reference_tracking(void)
{
  const char* source =
      "struct Req { x: int32; }\n"
      "struct Res { y: int32; }\n"
      "service Base {\n"
      "  op(Req): Res;\n"
      "}\n"
      "service Child with Base {\n"
      "  childOp(Req): Res;\n"
      "}\n";

  bebop_parse_result_t* result = parse_expect_success(source);

  const bebop_def_t* base = find_def(result, "Base");
  TEST_ASSERT_NOT_NULL(base);

  // Base should have at least one reference from the 'with Base' in Child
  uint32_t ref_count = bebop_def_references_count(base);
  TEST_ASSERT_TRUE(ref_count >= 1);

  BEBOP_UNUSED(result);
}

// Error: mixin is not a service
void test_service_mixin_not_service_error(void)
{
  const char* source =
      "struct NotAService { x: int32; }\n"
      "struct Req { x: int32; }\n"
      "struct Res { y: int32; }\n"
      "service Bad with NotAService {\n"
      "  op(Req): Res;\n"
      "}\n";

  bebop_parse_result_t* result = parse_expect_error(source);

  bool found_mixin_error = false;
  uint32_t count = bebop_result_diagnostic_count(result);
  for (uint32_t i = 0; i < count; i++) {
    const bebop_diagnostic_t* diag = bebop_result_diagnostic_at(result, i);
    const char* msg = bebop_diagnostic_message(diag);
    if (msg && strstr(msg, "not a service")) {
      found_mixin_error = true;
      break;
    }
  }
  TEST_ASSERT_TRUE_MESSAGE(found_mixin_error, "Expected 'not a service' error");

  BEBOP_UNUSED(result);
}

// Error: conflicting method from mixin
void test_service_mixin_method_conflict_error(void)
{
  const char* source =
      "struct Req { x: int32; }\n"
      "struct Res { y: int32; }\n"
      "service Base {\n"
      "  doThing(Req): Res;\n"
      "}\n"
      "service Child with Base {\n"
      "  doThing(Req): Res;\n"  // conflicts with Base.doThing
      "}\n";

  bebop_parse_result_t* result = parse_expect_error(source);

  bool found_conflict_error = false;
  uint32_t count = bebop_result_diagnostic_count(result);
  for (uint32_t i = 0; i < count; i++) {
    const bebop_diagnostic_t* diag = bebop_result_diagnostic_at(result, i);
    const char* msg = bebop_diagnostic_message(diag);
    if (msg && strstr(msg, "conflicts")) {
      found_conflict_error = true;
      break;
    }
  }
  TEST_ASSERT_TRUE_MESSAGE(found_conflict_error, "Expected method conflict error");

  BEBOP_UNUSED(result);
}

// Error: conflicting methods between two mixins
void test_service_mixin_mixin_conflict_error(void)
{
  const char* source =
      "struct Req { x: int32; }\n"
      "struct Res { y: int32; }\n"
      "service ServiceA {\n"
      "  sharedMethod(Req): Res;\n"
      "}\n"
      "service ServiceB {\n"
      "  sharedMethod(Req): Res;\n"
      "}\n"
      "service Bad with ServiceA, ServiceB {\n"
      "  uniqueMethod(Req): Res;\n"
      "}\n";

  bebop_parse_result_t* result = parse_expect_error(source);

  bool found_conflict_error = false;
  uint32_t count = bebop_result_diagnostic_count(result);
  for (uint32_t i = 0; i < count; i++) {
    const bebop_diagnostic_t* diag = bebop_result_diagnostic_at(result, i);
    const char* msg = bebop_diagnostic_message(diag);
    if (msg && strstr(msg, "conflicts")) {
      found_conflict_error = true;
      break;
    }
  }
  TEST_ASSERT_TRUE_MESSAGE(found_conflict_error, "Expected method conflict error between mixins");

  BEBOP_UNUSED(result);
}

// Chained mixins - if A uses B and B uses C, C's methods should be tracked
void test_service_chained_mixins(void)
{
  const char* source =
      "struct Req { x: int32; }\n"
      "struct Res { y: int32; }\n"
      "service Level0 {\n"
      "  level0Method(Req): Res;\n"
      "}\n"
      "service Level1 with Level0 {\n"
      "  level1Method(Req): Res;\n"
      "}\n"
      "service Level2 with Level1 {\n"
      "  level2Method(Req): Res;\n"
      "}\n";

  bebop_parse_result_t* result = parse_expect_success(source);

  const bebop_def_t* level2 = find_def(result, "Level2");
  TEST_ASSERT_NOT_NULL(level2);
  TEST_ASSERT_EQUAL(1, bebop_def_mixin_count(level2));
  TEST_ASSERT_EQUAL(1, bebop_def_method_count(level2));

  // Level1 is the direct mixin
  const bebop_type_t* mixin = bebop_def_mixin_at(level2, 0);
  const bebop_def_t* level1 = bebop_type_resolved(mixin);
  TEST_ASSERT_EQUAL_STRING("Level1", bebop_def_name(level1));

  // Level1 should have Level0 as its mixin
  TEST_ASSERT_EQUAL(1, bebop_def_mixin_count(level1));
  const bebop_type_t* level1_mixin = bebop_def_mixin_at(level1, 0);
  TEST_ASSERT_EQUAL_STRING("Level0", bebop_def_name(bebop_type_resolved(level1_mixin)));

  BEBOP_UNUSED(result);
}

// Service with empty body and mixin only
void test_service_empty_body_with_mixin(void)
{
  const char* source =
      "struct Req { x: int32; }\n"
      "struct Res { y: int32; }\n"
      "service Base {\n"
      "  op(Req): Res;\n"
      "}\n"
      "service Wrapper with Base { }\n";

  bebop_parse_result_t* result = parse_expect_success(source);

  const bebop_def_t* wrapper = find_def(result, "Wrapper");
  TEST_ASSERT_NOT_NULL(wrapper);
  TEST_ASSERT_EQUAL(1, bebop_def_mixin_count(wrapper));
  TEST_ASSERT_EQUAL(0, bebop_def_method_count(wrapper));

  BEBOP_UNUSED(result);
}

// Streaming methods with mixins
void test_service_streaming_with_mixin(void)
{
  const char* source =
      "struct Req { x: int32; }\n"
      "struct Res { y: int32; }\n"
      "service StreamBase {\n"
      "  streamOp(stream Req): stream Res;\n"
      "}\n"
      "service StreamChild with StreamBase {\n"
      "  clientStream(stream Req): Res;\n"
      "}\n";

  bebop_parse_result_t* result = parse_expect_success(source);

  const bebop_def_t* child = find_def(result, "StreamChild");
  TEST_ASSERT_NOT_NULL(child);
  TEST_ASSERT_EQUAL(1, bebop_def_mixin_count(child));

  BEBOP_UNUSED(result);
}

// Mixin at-index out of bounds returns NULL
void test_service_mixin_at_bounds(void)
{
  const char* source =
      "struct Req { x: int32; }\n"
      "struct Res { y: int32; }\n"
      "service Base {\n"
      "  op(Req): Res;\n"
      "}\n"
      "service Child with Base {\n"
      "  childOp(Req): Res;\n"
      "}\n";

  bebop_parse_result_t* result = parse_expect_success(source);

  const bebop_def_t* child = find_def(result, "Child");
  TEST_ASSERT_NOT_NULL(child);
  TEST_ASSERT_EQUAL(1, bebop_def_mixin_count(child));

  TEST_ASSERT_NOT_NULL(bebop_def_mixin_at(child, 0));
  TEST_ASSERT_NULL(bebop_def_mixin_at(child, 1));
  TEST_ASSERT_NULL(bebop_def_mixin_at(child, 100));

  BEBOP_UNUSED(result);
}

// Mixin count on non-service returns 0
void test_service_mixin_count_non_service(void)
{
  const char* source = "struct NotService { x: int32; }\n";

  bebop_parse_result_t* result = parse_expect_success(source);

  const bebop_def_t* def = find_def(result, "NotService");
  TEST_ASSERT_NOT_NULL(def);
  TEST_ASSERT_EQUAL(0, bebop_def_mixin_count(def));
  TEST_ASSERT_NULL(bebop_def_mixin_at(def, 0));

  BEBOP_UNUSED(result);
}

int main(void)
{
  UNITY_BEGIN();
  RUN_TEST(test_service_basic_parsing);
  RUN_TEST(test_service_single_mixin);
  RUN_TEST(test_service_multiple_mixins);
  RUN_TEST(test_service_mixin_reference_tracking);
  RUN_TEST(test_service_mixin_not_service_error);
  RUN_TEST(test_service_mixin_method_conflict_error);
  RUN_TEST(test_service_mixin_mixin_conflict_error);
  RUN_TEST(test_service_chained_mixins);
  RUN_TEST(test_service_empty_body_with_mixin);
  RUN_TEST(test_service_streaming_with_mixin);
  RUN_TEST(test_service_mixin_at_bounds);
  RUN_TEST(test_service_mixin_count_non_service);
  return UNITY_END();
}
