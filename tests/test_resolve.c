#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "bebop.h"
#include "test_common.h"
#include "unity.h"

static bebop_context_t* ctx;

void setUp(void);
void tearDown(void);

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

static bebop_status_t parse_and_validate_multi(
    const char** sources, const char** paths, uint32_t count, bebop_parse_result_t** out_result
)
{
  bebop_source_t* srcs = malloc(count * sizeof(bebop_source_t));
  TEST_ASSERT_NOT_NULL(srcs);

  for (uint32_t i = 0; i < count; i++) {
    srcs[i] = (bebop_source_t) {sources[i], strlen(sources[i]), paths[i]};
  }

  bebop_status_t status = bebop_parse_sources(ctx, srcs, count, out_result);
  free(srcs);
  return status;
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

void test_package_basic(void);

void test_package_basic(void)
{
  const char* source = "package foo.bar;\n" "struct Point { x: int32; y: int32; }\n";

  bebop_parse_result_t* result = NULL;
  bebop_status_t status = bebop_parse_source(ctx, BEBOP_SOURCE(source, "test.bop"), &result);
  TEST_ASSERT_EQUAL(BEBOP_OK, status);

  const bebop_schema_t* schema = bebop_result_schema_at(result, 0);
  TEST_ASSERT_NOT_NULL(schema);

  const char* pkg = bebop_schema_package(schema);
  TEST_ASSERT_NOT_NULL(pkg);
  TEST_ASSERT_EQUAL_STRING("foo.bar", pkg);

  const bebop_def_t* def = bebop_result_find(result, "foo.bar.Point");
  TEST_ASSERT_NOT_NULL(def);
  TEST_ASSERT_EQUAL_STRING("Point", bebop_def_name(def));
}

void test_same_name_different_packages(void);

void test_same_name_different_packages(void)
{
  const char* sources[] = {
      "package a;\n" "struct Foo { x: int32; }\n",

      "package b;\n" "struct Foo { y: string; }\n"
  };
  const char* paths[] = {"a.bop", "b.bop"};

  bebop_parse_result_t* result = NULL;
  bebop_status_t status = parse_and_validate_multi(sources, paths, 2, &result);

  if (status != BEBOP_OK && status != BEBOP_OK_WITH_WARNINGS) {
    render_diagnostics(result);
  }
  TEST_ASSERT_TRUE(status == BEBOP_OK || status == BEBOP_OK_WITH_WARNINGS);
  TEST_ASSERT_EQUAL_UINT32(0, bebop_result_error_count(result));

  const bebop_def_t* a_foo = bebop_result_find(result, "a.Foo");
  TEST_ASSERT_NOT_NULL(a_foo);

  const bebop_def_t* b_foo = bebop_result_find(result, "b.Foo");
  TEST_ASSERT_NOT_NULL(b_foo);

  TEST_ASSERT_TRUE(a_foo != b_foo);
}

void test_same_name_no_package_conflict(void);

void test_same_name_no_package_conflict(void)
{
  const char* sources[] = {"struct Foo { x: int32; }\n", "struct Foo { y: string; }\n"};
  const char* paths[] = {"a.bop", "b.bop"};

  bebop_parse_result_t* result = NULL;
  bebop_status_t status = parse_and_validate_multi(sources, paths, 2, &result);

  TEST_ASSERT_EQUAL(BEBOP_ERROR, status);
  TEST_ASSERT_TRUE(bebop_result_error_count(result) > 0);

  const bebop_diagnostic_t* diag = bebop_result_diagnostic_at(result, 0);
  TEST_ASSERT_NOT_NULL(diag);
  const char* msg = bebop_diagnostic_message(diag);
  TEST_ASSERT_NOT_NULL(strstr(msg, "multiple schemas"));
}

void test_cross_package_requires_import(void);

void test_cross_package_requires_import(void)
{
  const char* sources[] = {
      "package types;\n" "struct Point { x: int32; y: int32; }\n",

      "package app;\n" "struct Location { position: types.Point; }\n"
  };
  const char* paths[] = {"types.bop", "app.bop"};

  bebop_parse_result_t* result = NULL;
  bebop_status_t status = parse_and_validate_multi(sources, paths, 2, &result);

  TEST_ASSERT_EQUAL(BEBOP_ERROR, status);
  TEST_ASSERT_TRUE(bebop_result_error_count(result) > 0);

  const bebop_diagnostic_t* diag = bebop_result_diagnostic_at(result, 0);
  const char* msg = bebop_diagnostic_message(diag);
  TEST_ASSERT_NOT_NULL(strstr(msg, "Unknown type"));
}

void test_same_package_simple_name(void);

void test_same_package_simple_name(void)
{
  const char* source = "package myapp;\n"
                         "struct Point { x: int32; y: int32; }\n"
                         "struct Line { start: Point; end: Point; }\n";

  bebop_parse_result_t* result = NULL;
  bebop_status_t status = bebop_parse_source(ctx, BEBOP_SOURCE(source, "test.bop"), &result);

  if (status != BEBOP_OK && status != BEBOP_OK_WITH_WARNINGS) {
    render_diagnostics(result);
  }
  TEST_ASSERT_TRUE(status == BEBOP_OK || status == BEBOP_OK_WITH_WARNINGS);
  TEST_ASSERT_EQUAL_UINT32(0, bebop_result_error_count(result));

  const bebop_def_t* line = bebop_result_find(result, "myapp.Line");
  TEST_ASSERT_NOT_NULL(line);

  const bebop_field_t* start = bebop_def_field_at(line, 0);
  const bebop_type_t* start_type = bebop_field_type(start);
  const bebop_def_t* start_resolved = bebop_type_resolved(start_type);
  TEST_ASSERT_NOT_NULL(start_resolved);

  const bebop_def_t* point = bebop_result_find(result, "myapp.Point");
  TEST_ASSERT_TRUE(start_resolved == point);
}

void test_nested_type_with_package(void);

void test_nested_type_with_package(void)
{
  const char* source = "package myapp;\n"
                         "struct Outer {\n"
                         "    value: int32;\n"
                         "    export struct Inner { name: string; }\n"
                         "}\n"
                         "struct User { info: Outer.Inner; }\n";

  bebop_parse_result_t* result = NULL;
  bebop_status_t status = bebop_parse_source(ctx, BEBOP_SOURCE(source, "test.bop"), &result);

  if (status != BEBOP_OK && status != BEBOP_OK_WITH_WARNINGS) {
    render_diagnostics(result);
  }
  TEST_ASSERT_TRUE(status == BEBOP_OK || status == BEBOP_OK_WITH_WARNINGS);
  TEST_ASSERT_EQUAL_UINT32(0, bebop_result_error_count(result));

  const bebop_def_t* user = bebop_result_find(result, "myapp.User");
  TEST_ASSERT_NOT_NULL(user);

  const bebop_field_t* info = bebop_def_field_at(user, 0);
  TEST_ASSERT_EQUAL_STRING("info", bebop_field_name(info));

  const bebop_def_t* resolved = bebop_type_resolved(bebop_field_type(info));
  TEST_ASSERT_NOT_NULL(resolved);

  const bebop_def_t* inner = bebop_result_find(result, "myapp.Outer.Inner");
  TEST_ASSERT_NOT_NULL(inner);
  TEST_ASSERT_TRUE(resolved == inner);
}

void test_cross_package_nested_requires_import(void);

void test_cross_package_nested_requires_import(void)
{
  const char* sources[] = {
        "package types;\n"
        "struct Container {\n"
        "    id: int32;\n"
        "    export struct Point { x: int32; y: int32; }\n"
        "}\n",

        "package app;\n"
        "struct Location { pos: types.Container.Point; }\n"
    };
  const char* paths[] = {"types.bop", "app.bop"};

  bebop_parse_result_t* result = NULL;
  bebop_status_t status = parse_and_validate_multi(sources, paths, 2, &result);

  TEST_ASSERT_EQUAL(BEBOP_ERROR, status);
  TEST_ASSERT_TRUE(bebop_result_error_count(result) > 0);
}

void test_simple_name_requires_same_package(void);

void test_simple_name_requires_same_package(void)
{
  const char* sources[] = {
      "package a;\n" "struct Foo { x: int32; }\n",

      "package b;\n" "struct Bar { f: Foo; }\n"  //! Error: must use "a.Foo"
  };
  const char* paths[] = {"a.bop", "b.bop"};

  bebop_parse_result_t* result = NULL;
  bebop_status_t status = parse_and_validate_multi(sources, paths, 2, &result);

  TEST_ASSERT_EQUAL(BEBOP_ERROR, status);
  TEST_ASSERT_TRUE(bebop_result_error_count(result) > 0);

  const bebop_diagnostic_t* diag = bebop_result_diagnostic_at(result, 0);
  const char* msg = bebop_diagnostic_message(diag);
  TEST_ASSERT_NOT_NULL(strstr(msg, "Unknown type"));
}

void test_no_import_no_visibility(void);

void test_no_import_no_visibility(void)
{
  const char* sources[] = {
      "struct Point { x: int32; y: int32; }\n", "struct Line { start: Point; end: Point; }\n"
  };
  const char* paths[] = {"point.bop", "line.bop"};

  bebop_parse_result_t* result = NULL;
  bebop_status_t status = parse_and_validate_multi(sources, paths, 2, &result);

  TEST_ASSERT_EQUAL(BEBOP_ERROR, status);
  TEST_ASSERT_TRUE(bebop_result_error_count(result) > 0);

  const bebop_diagnostic_t* diag = bebop_result_diagnostic_at(result, 0);
  const char* msg = bebop_diagnostic_message(diag);
  TEST_ASSERT_NOT_NULL(strstr(msg, "Unknown type"));
}

#define FIXTURE(name) BEBOP_TEST_FIXTURES_DIR "/valid/" name

static bebop_parse_result_t* parse_fixture_with_imports(const char* path)
{
  bebop_parse_result_t* result = NULL;
  const char* paths[] = {path};
  bebop_status_t status = bebop_parse(ctx, paths, 1, &result);

  if (status == BEBOP_ERROR || status == BEBOP_FATAL || bebop_result_error_count(result) > 0) {
    fprintf(stderr, "\nFailed to parse fixture '%s' (status=%d):\n", path, status);
    render_diagnostics(result);
    char msg[512];
    snprintf(msg, sizeof(msg), "Fixture '%s' failed to parse", path);
    TEST_FAIL_MESSAGE(msg);
  }

  return result;
}

void test_import_cross_package_fqn(void);

void test_import_cross_package_fqn(void)
{
  bebop_parse_result_t* result = parse_fixture_with_imports(FIXTURE("pkg_app.bop"));
  TEST_ASSERT_NOT_NULL(result);

  TEST_ASSERT_TRUE(bebop_result_schema_count(result) >= 2);

  const bebop_def_t* types_point = bebop_result_find(result, "types.Point");
  TEST_ASSERT_NOT_NULL_MESSAGE(types_point, "types.Point should exist");

  const bebop_def_t* app_point = bebop_result_find(result, "app.Point");
  TEST_ASSERT_NOT_NULL_MESSAGE(app_point, "app.Point should exist");
  TEST_ASSERT_TRUE(types_point != app_point);

  const bebop_def_t* rect = bebop_result_find(result, "app.Rectangle");
  TEST_ASSERT_NOT_NULL(rect);

  const bebop_field_t* origin = bebop_def_field_at(rect, 0);
  TEST_ASSERT_EQUAL_STRING("origin", bebop_field_name(origin));

  const bebop_def_t* origin_resolved = bebop_type_resolved(bebop_field_type(origin));
  TEST_ASSERT_NOT_NULL(origin_resolved);
  TEST_ASSERT_TRUE_MESSAGE(
      origin_resolved == types_point, "Rectangle.origin should resolve to types.Point"
  );

  const bebop_def_t* location = bebop_result_find(result, "app.Location");
  TEST_ASSERT_NOT_NULL(location);

  const bebop_field_t* coords = bebop_def_field_at(location, 0);
  TEST_ASSERT_EQUAL_STRING("coords", bebop_field_name(coords));

  const bebop_def_t* coords_resolved = bebop_type_resolved(bebop_field_type(coords));
  TEST_ASSERT_NOT_NULL(coords_resolved);
  TEST_ASSERT_TRUE_MESSAGE(
      coords_resolved == app_point, "Location.coords should resolve to app.Point (local)"
  );

  const bebop_field_t* screen = bebop_def_field_at(location, 1);
  TEST_ASSERT_EQUAL_STRING("screen", bebop_field_name(screen));

  const bebop_def_t* screen_resolved = bebop_type_resolved(bebop_field_type(screen));
  TEST_ASSERT_NOT_NULL(screen_resolved);
  TEST_ASSERT_TRUE_MESSAGE(
      screen_resolved == types_point, "Location.screen should resolve to types.Point (imported)"
  );
}

void test_import_nested_type_cross_package(void);

void test_import_nested_type_cross_package(void)
{
  bebop_parse_result_t* result = parse_fixture_with_imports(FIXTURE("pkg_import_nested.bop"));
  TEST_ASSERT_NOT_NULL(result);

  TEST_ASSERT_TRUE(bebop_result_schema_count(result) >= 2);

  const bebop_def_t* nested_point = bebop_result_find(result, "shapes.Container.Point");
  TEST_ASSERT_NOT_NULL_MESSAGE(nested_point, "shapes.Container.Point should exist");

  const bebop_def_t* polygon = bebop_result_find(result, "graphics.Polygon");
  TEST_ASSERT_NOT_NULL(polygon);

  const bebop_field_t* vertices = bebop_def_field_at(polygon, 0);
  TEST_ASSERT_EQUAL_STRING("vertices", bebop_field_name(vertices));

  const bebop_type_t* vertices_type = bebop_field_type(vertices);
  TEST_ASSERT_EQUAL(BEBOP_TYPE_ARRAY, bebop_type_kind(vertices_type));

  const bebop_type_t* element_type = bebop_type_element(vertices_type);
  TEST_ASSERT_NOT_NULL(element_type);

  const bebop_def_t* element_resolved = bebop_type_resolved(element_type);
  TEST_ASSERT_NOT_NULL(element_resolved);
  TEST_ASSERT_TRUE_MESSAGE(
      element_resolved == nested_point,
      "Polygon.vertices[] should resolve to shapes.Container.Point"
  );
}

void test_import_same_package_name_local_first(void);

void test_import_same_package_name_local_first(void)
{
  bebop_parse_result_t* result = parse_fixture_with_imports(FIXTURE("pkg_app.bop"));
  TEST_ASSERT_NOT_NULL(result);

  const bebop_def_t* app_point = bebop_result_find(result, "app.Point");
  const bebop_def_t* types_point = bebop_result_find(result, "types.Point");

  TEST_ASSERT_NOT_NULL(app_point);
  TEST_ASSERT_NOT_NULL(types_point);

  const bebop_def_t* location = bebop_result_find(result, "app.Location");
  const bebop_field_t* coords = bebop_def_field_at(location, 0);
  const bebop_def_t* coords_resolved = bebop_type_resolved(bebop_field_type(coords));

  TEST_ASSERT_TRUE_MESSAGE(
      coords_resolved == app_point,
      "Unqualified 'Point' should resolve to local " "app.Point, not imported types.Point"
  );
}

static int32_t find_def_index(bebop_parse_result_t* result, const char* fqn)
{
  uint32_t count = bebop_result_definition_count(result);
  for (uint32_t i = 0; i < count; i++) {
    const bebop_def_t* def = bebop_result_definition_at(result, i);
    const char* def_fqn = bebop_def_fqn(def);
    if (def_fqn && strcmp(def_fqn, fqn) == 0) {
      return (int32_t)i;
    }
  }
  return -1;
}

void test_toposort_simple_dependency(void);

void test_toposort_simple_dependency(void)
{
  const char* source = "struct A { x: int32; }\n" "struct B { a: A; }\n";

  bebop_parse_result_t* result = NULL;
  bebop_status_t status = bebop_parse_source(ctx, BEBOP_SOURCE(source, "test.bop"), &result);
  TEST_ASSERT_TRUE(status == BEBOP_OK || status == BEBOP_OK_WITH_WARNINGS);

  int32_t idx_a = find_def_index(result, "A");
  int32_t idx_b = find_def_index(result, "B");

  TEST_ASSERT_TRUE(idx_a >= 0);
  TEST_ASSERT_TRUE(idx_b >= 0);
  TEST_ASSERT_TRUE_MESSAGE(idx_a < idx_b, "A must come before B (B depends on A)");
}

void test_toposort_chain_dependency(void);

void test_toposort_chain_dependency(void)
{
  const char* source = "struct C { b: B; }\n" "struct A { x: int32; }\n" "struct B { a: A; }\n";

  bebop_parse_result_t* result = NULL;
  bebop_status_t status = bebop_parse_source(ctx, BEBOP_SOURCE(source, "test.bop"), &result);
  TEST_ASSERT_TRUE(status == BEBOP_OK || status == BEBOP_OK_WITH_WARNINGS);

  int32_t idx_a = find_def_index(result, "A");
  int32_t idx_b = find_def_index(result, "B");
  int32_t idx_c = find_def_index(result, "C");

  TEST_ASSERT_TRUE(idx_a >= 0);
  TEST_ASSERT_TRUE(idx_b >= 0);
  TEST_ASSERT_TRUE(idx_c >= 0);
  TEST_ASSERT_TRUE_MESSAGE(idx_a < idx_b, "A must come before B");
  TEST_ASSERT_TRUE_MESSAGE(idx_b < idx_c, "B must come before C");
}

void test_toposort_nested_type(void);

void test_toposort_nested_type(void)
{
  const char* source = "package myapp;\n"
                         "struct User { info: Outer.Inner; }\n"
                         "struct Outer {\n"
                         "    id: int32;\n"
                         "    export struct Inner { name: string; }\n"
                         "}\n";

  bebop_parse_result_t* result = NULL;
  bebop_status_t status = bebop_parse_source(ctx, BEBOP_SOURCE(source, "test.bop"), &result);

  if (status != BEBOP_OK && status != BEBOP_OK_WITH_WARNINGS) {
    render_diagnostics(result);
  }
  TEST_ASSERT_TRUE(status == BEBOP_OK || status == BEBOP_OK_WITH_WARNINGS);

  int32_t idx_inner = find_def_index(result, "myapp.Outer.Inner");
  int32_t idx_user = find_def_index(result, "myapp.User");

  TEST_ASSERT_TRUE_MESSAGE(idx_inner >= 0, "Outer.Inner should exist");
  TEST_ASSERT_TRUE_MESSAGE(idx_user >= 0, "User should exist");
  TEST_ASSERT_TRUE_MESSAGE(idx_inner < idx_user, "Outer.Inner must come before User");
}

void test_toposort_cross_schema_import(void);

void test_toposort_cross_schema_import(void)
{
  bebop_parse_result_t* result = parse_fixture_with_imports(FIXTURE("pkg_app.bop"));
  TEST_ASSERT_NOT_NULL(result);

  int32_t idx_point = find_def_index(result, "types.Point");
  int32_t idx_location = find_def_index(result, "app.Location");
  int32_t idx_rect = find_def_index(result, "app.Rectangle");

  TEST_ASSERT_TRUE_MESSAGE(idx_point >= 0, "types.Point should exist");
  TEST_ASSERT_TRUE_MESSAGE(idx_location >= 0, "app.Location should exist");
  TEST_ASSERT_TRUE_MESSAGE(idx_rect >= 0, "app.Rectangle should exist");

  TEST_ASSERT_TRUE_MESSAGE(
      idx_point < idx_rect,
      "types.Point must come before app.Rectangle " "(Rectangle uses types.Point)"
  );
}

void test_toposort_cross_schema_nested(void);

void test_toposort_cross_schema_nested(void)
{
  bebop_parse_result_t* result = parse_fixture_with_imports(FIXTURE("pkg_import_nested.bop"));
  TEST_ASSERT_NOT_NULL(result);

  int32_t idx_nested_point = find_def_index(result, "shapes.Container.Point");
  int32_t idx_polygon = find_def_index(result, "graphics.Polygon");

  TEST_ASSERT_TRUE_MESSAGE(idx_nested_point >= 0, "shapes.Container.Point should exist");
  TEST_ASSERT_TRUE_MESSAGE(idx_polygon >= 0, "graphics.Polygon should exist");
  TEST_ASSERT_TRUE_MESSAGE(
      idx_nested_point < idx_polygon, "shapes.Container.Point must come before graphics.Polygon"
  );
}

int main(void)
{
  UNITY_BEGIN();

  RUN_TEST(test_package_basic);

  RUN_TEST(test_same_name_different_packages);
  RUN_TEST(test_same_name_no_package_conflict);
  RUN_TEST(test_same_package_simple_name);

  RUN_TEST(test_nested_type_with_package);

  RUN_TEST(test_simple_name_requires_same_package);
  RUN_TEST(test_cross_package_requires_import);
  RUN_TEST(test_cross_package_nested_requires_import);
  RUN_TEST(test_no_import_no_visibility);

  RUN_TEST(test_import_cross_package_fqn);
  RUN_TEST(test_import_nested_type_cross_package);
  RUN_TEST(test_import_same_package_name_local_first);

  RUN_TEST(test_toposort_simple_dependency);
  RUN_TEST(test_toposort_chain_dependency);
  RUN_TEST(test_toposort_nested_type);
  RUN_TEST(test_toposort_cross_schema_import);
  RUN_TEST(test_toposort_cross_schema_nested);

  return UNITY_END();
}
