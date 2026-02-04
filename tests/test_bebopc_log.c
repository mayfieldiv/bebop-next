#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "bebopc_log.h"
#include "unity.h"

#ifdef _WIN32
#include <fcntl.h>
#include <io.h>
#else
#include <unistd.h>
#endif

static bebopc_log_ctx_t ctx;

void setUp(void)
{
  bebopc_log_ctx_init(&ctx, true);
}

void tearDown(void) {}

void test_log_level_name(void);
void test_log_level_color(void);
void test_log_level_parse(void);
void test_log_level_parse_case_insensitive(void);

void test_log_level_name(void)
{
  TEST_ASSERT_EQUAL_STRING("TRACE", bebopc_log_level_name(BEBOPC_LOG_TRACE));
  TEST_ASSERT_EQUAL_STRING("DEBUG", bebopc_log_level_name(BEBOPC_LOG_DEBUG));
  TEST_ASSERT_EQUAL_STRING("INFO", bebopc_log_level_name(BEBOPC_LOG_INFO));
  TEST_ASSERT_EQUAL_STRING("WARN", bebopc_log_level_name(BEBOPC_LOG_WARN));
  TEST_ASSERT_EQUAL_STRING("ERROR", bebopc_log_level_name(BEBOPC_LOG_ERROR));
  TEST_ASSERT_EQUAL_STRING("FATAL", bebopc_log_level_name(BEBOPC_LOG_FATAL));
}

void test_log_level_color(void)
{
  TEST_ASSERT_EQUAL_STRING("[dim]", bebopc_log_level_color(BEBOPC_LOG_TRACE));
  TEST_ASSERT_EQUAL_STRING("[cyan]", bebopc_log_level_color(BEBOPC_LOG_DEBUG));
  TEST_ASSERT_EQUAL_STRING("[green]", bebopc_log_level_color(BEBOPC_LOG_INFO));
  TEST_ASSERT_EQUAL_STRING("[yellow]", bebopc_log_level_color(BEBOPC_LOG_WARN));
  TEST_ASSERT_EQUAL_STRING("[red]", bebopc_log_level_color(BEBOPC_LOG_ERROR));
  TEST_ASSERT_EQUAL_STRING("[bold red]", bebopc_log_level_color(BEBOPC_LOG_FATAL));
}

void test_log_level_parse(void)
{
  TEST_ASSERT_EQUAL(BEBOPC_LOG_TRACE, bebopc_log_level_parse("trace"));
  TEST_ASSERT_EQUAL(BEBOPC_LOG_DEBUG, bebopc_log_level_parse("debug"));
  TEST_ASSERT_EQUAL(BEBOPC_LOG_INFO, bebopc_log_level_parse("info"));
  TEST_ASSERT_EQUAL(BEBOPC_LOG_WARN, bebopc_log_level_parse("warn"));
  TEST_ASSERT_EQUAL(BEBOPC_LOG_WARN, bebopc_log_level_parse("warning"));
  TEST_ASSERT_EQUAL(BEBOPC_LOG_ERROR, bebopc_log_level_parse("error"));
  TEST_ASSERT_EQUAL(BEBOPC_LOG_FATAL, bebopc_log_level_parse("fatal"));
  TEST_ASSERT_EQUAL(BEBOPC_LOG_OFF, bebopc_log_level_parse("off"));
  TEST_ASSERT_EQUAL(BEBOPC_LOG_INFO, bebopc_log_level_parse("unknown"));
  TEST_ASSERT_EQUAL(BEBOPC_LOG_INFO, bebopc_log_level_parse(NULL));
}

void test_log_level_parse_case_insensitive(void)
{
  TEST_ASSERT_EQUAL(BEBOPC_LOG_TRACE, bebopc_log_level_parse("TRACE"));
  TEST_ASSERT_EQUAL(BEBOPC_LOG_DEBUG, bebopc_log_level_parse("Debug"));
  TEST_ASSERT_EQUAL(BEBOPC_LOG_INFO, bebopc_log_level_parse("INFO"));
  TEST_ASSERT_EQUAL(BEBOPC_LOG_WARN, bebopc_log_level_parse("WARN"));
  TEST_ASSERT_EQUAL(BEBOPC_LOG_ERROR, bebopc_log_level_parse("Error"));
}

void test_markup_strip_plain_text(void);
void test_markup_strip_colors(void);
void test_markup_strip_styles(void);
void test_markup_strip_nested(void);
void test_markup_strip_escaped_brackets(void);
void test_markup_strip_unclosed_tag(void);

void test_markup_strip_plain_text(void)
{
  char* result = bebopc_markup_strip("Hello, World!");
  TEST_ASSERT_NOT_NULL(result);
  TEST_ASSERT_EQUAL_STRING("Hello, World!", result);
  free(result);
}

void test_markup_strip_colors(void)
{
  char* result = bebopc_markup_strip("[red]Error[/]");
  TEST_ASSERT_NOT_NULL(result);
  TEST_ASSERT_EQUAL_STRING("Error", result);
  free(result);

  result = bebopc_markup_strip("[green]Success[/] and [yellow]Warning[/]");
  TEST_ASSERT_NOT_NULL(result);
  TEST_ASSERT_EQUAL_STRING("Success and Warning", result);
  free(result);
}

void test_markup_strip_styles(void)
{
  char* result = bebopc_markup_strip("[bold]Important[/]");
  TEST_ASSERT_NOT_NULL(result);
  TEST_ASSERT_EQUAL_STRING("Important", result);
  free(result);

  result = bebopc_markup_strip("[italic]Emphasis[/]");
  TEST_ASSERT_NOT_NULL(result);
  TEST_ASSERT_EQUAL_STRING("Emphasis", result);
  free(result);
}

void test_markup_strip_nested(void)
{
  char* result = bebopc_markup_strip("[bold][red]Bold Red[/][/]");
  TEST_ASSERT_NOT_NULL(result);
  TEST_ASSERT_EQUAL_STRING("Bold Red", result);
  free(result);
}

void test_markup_strip_escaped_brackets(void)
{
  char* result = bebopc_markup_strip("Use [[brackets]] for escaping");
  TEST_ASSERT_NOT_NULL(result);
  TEST_ASSERT_EQUAL_STRING("Use [brackets] for escaping", result);
  free(result);
}

void test_markup_strip_unclosed_tag(void)
{
  char* result = bebopc_markup_strip("[unclosed tag");
  TEST_ASSERT_NOT_NULL(result);
  TEST_ASSERT_EQUAL_STRING("[unclosed tag", result);
  free(result);
}

void test_markup_render_no_colors(void);
void test_markup_render_with_colors(void);
void test_markup_render_background(void);
void test_markup_render_combined(void);
void test_markup_render_hex_color(void);
void test_markup_render_rgb_color(void);
void test_markup_render_link(void);
void test_markup_render_bright_colors(void);

void test_markup_render_no_colors(void)
{
  char* result = bebopc_markup_render("[red]Error[/]", false);
  TEST_ASSERT_NOT_NULL(result);
  TEST_ASSERT_EQUAL_STRING("Error", result);
  free(result);
}

void test_markup_render_with_colors(void)
{
  char* result = bebopc_markup_render("[red]Error[/]", true);
  TEST_ASSERT_NOT_NULL(result);

  TEST_ASSERT_TRUE(strstr(result, "\x1b[") != NULL);
  TEST_ASSERT_TRUE(strstr(result, "Error") != NULL);
  TEST_ASSERT_TRUE(strstr(result, "\x1b[0m") != NULL);
  free(result);
}

void test_markup_render_background(void)
{
  char* result = bebopc_markup_render("[white on red]Alert[/]", true);
  TEST_ASSERT_NOT_NULL(result);

  TEST_ASSERT_TRUE(strstr(result, "\x1b[37m") != NULL);
  TEST_ASSERT_TRUE(strstr(result, "\x1b[41m") != NULL);
  TEST_ASSERT_TRUE(strstr(result, "Alert") != NULL);
  free(result);
}

void test_markup_render_combined(void)
{
  char* result = bebopc_markup_render("[bold red]Important[/]", true);
  TEST_ASSERT_NOT_NULL(result);
  TEST_ASSERT_TRUE(strstr(result, "\x1b[1m") != NULL);
  TEST_ASSERT_TRUE(strstr(result, "\x1b[31m") != NULL);
  TEST_ASSERT_TRUE(strstr(result, "Important") != NULL);
  free(result);
}

void test_markup_render_hex_color(void)
{
  char* result = bebopc_markup_render("[#ff0000]Red[/]", true);
  TEST_ASSERT_NOT_NULL(result);

  TEST_ASSERT_TRUE(strstr(result, "\x1b[38;2;255;0;0m") != NULL);
  TEST_ASSERT_TRUE(strstr(result, "Red") != NULL);
  free(result);

  result = bebopc_markup_render("[#ffffff on #0000ff]Text[/]", true);
  TEST_ASSERT_NOT_NULL(result);
  TEST_ASSERT_TRUE(strstr(result, "\x1b[38;2;255;255;255m") != NULL);
  TEST_ASSERT_TRUE(strstr(result, "\x1b[48;2;0;0;255m") != NULL);
  free(result);
}

void test_markup_render_rgb_color(void)
{
  char* result = bebopc_markup_render("[rgb(255,128,0)]Orange[/]", true);
  TEST_ASSERT_NOT_NULL(result);
  TEST_ASSERT_TRUE(strstr(result, "\x1b[38;2;255;128;0m") != NULL);
  TEST_ASSERT_TRUE(strstr(result, "Orange") != NULL);
  free(result);

  result = bebopc_markup_render("[rgb(255,255,255) on rgb(0,0,255)]Text[/]", true);
  TEST_ASSERT_NOT_NULL(result);
  TEST_ASSERT_TRUE(strstr(result, "\x1b[38;2;255;255;255m") != NULL);
  TEST_ASSERT_TRUE(strstr(result, "\x1b[48;2;0;0;255m") != NULL);
  free(result);
}

void test_markup_render_link(void)
{
  char* result = bebopc_markup_render("[link=https://example.com]Click here[/]", true);
  TEST_ASSERT_NOT_NULL(result);

  TEST_ASSERT_TRUE(strstr(result, "\x1b]8;;https://example.com\x07") != NULL);
  TEST_ASSERT_TRUE(strstr(result, "\x1b[4m") != NULL);
  TEST_ASSERT_TRUE(strstr(result, "Click here") != NULL);
  TEST_ASSERT_TRUE(strstr(result, "\x1b]8;;\x07") != NULL);
  free(result);
}

void test_markup_render_bright_colors(void)
{
  char* result = bebopc_markup_render("[brightred]Bright[/]", true);
  TEST_ASSERT_NOT_NULL(result);
  TEST_ASSERT_TRUE(strstr(result, "\x1b[91m") != NULL);
  free(result);

  result = bebopc_markup_render("[brightgreen]Success[/]", true);
  TEST_ASSERT_NOT_NULL(result);
  TEST_ASSERT_TRUE(strstr(result, "\x1b[92m") != NULL);
  free(result);
}

void test_markup_render_all_styles(void);

void test_markup_render_all_styles(void)
{
  char* result = bebopc_markup_render("[bold]B[/]", true);
  TEST_ASSERT_TRUE(strstr(result, "\x1b[1m") != NULL);
  free(result);

  result = bebopc_markup_render("[dim]D[/]", true);
  TEST_ASSERT_TRUE(strstr(result, "\x1b[2m") != NULL);
  free(result);

  result = bebopc_markup_render("[italic]I[/]", true);
  TEST_ASSERT_TRUE(strstr(result, "\x1b[3m") != NULL);
  free(result);

  result = bebopc_markup_render("[underline]U[/]", true);
  TEST_ASSERT_TRUE(strstr(result, "\x1b[4m") != NULL);
  free(result);

  result = bebopc_markup_render("[strikethrough]S[/]", true);
  TEST_ASSERT_TRUE(strstr(result, "\x1b[9m") != NULL);
  free(result);

  result = bebopc_markup_render("[invert]R[/]", true);
  TEST_ASSERT_TRUE(strstr(result, "\x1b[7m") != NULL);
  free(result);
}

void test_log_ctx_init(void);
void test_log_ctx_defaults(void);

void test_log_ctx_init(void)
{
  bebopc_log_ctx_t test_ctx;
  bebopc_log_ctx_init(&test_ctx, false);

  TEST_ASSERT_EQUAL(BEBOPC_LOG_INFO, test_ctx.level);
  TEST_ASSERT_FALSE(test_ctx.colors_enabled);
  TEST_ASSERT_FALSE(test_ctx.show_timestamp);
  TEST_ASSERT_TRUE(test_ctx.show_level);
  TEST_ASSERT_FALSE(test_ctx.show_source);
}

void test_log_ctx_defaults(void)
{
  bebopc_log_ctx_t test_ctx;
  bebopc_log_ctx_init(&test_ctx, true);

  TEST_ASSERT_EQUAL(BEBOPC_LOG_INFO, test_ctx.level);
  TEST_ASSERT_TRUE(test_ctx.colors_enabled);
}

typedef struct {
  FILE* file;
  char* buffer;
  size_t size;
} _capture_t;

static _capture_t _capture_start(void)
{
  _capture_t cap = {0};
#ifdef _WIN32
  cap.file = tmpfile();
#else
  cap.file = tmpfile();
#endif
  return cap;
}

static char* _capture_end(_capture_t* cap)
{
  if (!cap->file) {
    return NULL;
  }

  fflush(cap->file);
  long len = ftell(cap->file);
  rewind(cap->file);

  cap->buffer = malloc((size_t)len + 1);
  if (cap->buffer) {
    size_t read = fread(cap->buffer, 1, (size_t)len, cap->file);
    cap->buffer[read] = '\0';
  }
  fclose(cap->file);
  cap->file = NULL;
  return cap->buffer;
}

static void _capture_free(_capture_t* cap)
{
  free(cap->buffer);
  cap->buffer = NULL;
}

void test_log_basic_output(void);
void test_log_level_filtering(void);
void test_log_with_markup(void);
void test_log_with_timestamp(void);
void test_log_with_source(void);
void test_markup_output(void);
void test_markup_line_output(void);

void test_log_basic_output(void)
{
  _capture_t cap = _capture_start();
  TEST_ASSERT_NOT_NULL(cap.file);

  bebopc_log_ctx_t test_ctx;
  bebopc_log_ctx_init(&test_ctx, false);
  test_ctx.output = cap.file;

  bebopc_log(&test_ctx, BEBOPC_LOG_INFO, "Hello %s", "World");

  char* output = _capture_end(&cap);
  TEST_ASSERT_NOT_NULL(output);
  printf("  Output: [%s]\n", output);
  TEST_ASSERT_TRUE(strstr(output, "INFO") != NULL);
  TEST_ASSERT_TRUE(strstr(output, "Hello World") != NULL);
  TEST_ASSERT_TRUE(strstr(output, "\n") != NULL);

  _capture_free(&cap);
}

void test_log_level_filtering(void)
{
  _capture_t cap = _capture_start();
  TEST_ASSERT_NOT_NULL(cap.file);

  bebopc_log_ctx_t test_ctx;
  bebopc_log_ctx_init(&test_ctx, false);
  test_ctx.output = cap.file;
  test_ctx.level = BEBOPC_LOG_WARN;

  bebopc_log(&test_ctx, BEBOPC_LOG_DEBUG, "Debug message");
  bebopc_log(&test_ctx, BEBOPC_LOG_INFO, "Info message");
  bebopc_log(&test_ctx, BEBOPC_LOG_WARN, "Warning message");
  bebopc_log(&test_ctx, BEBOPC_LOG_ERROR, "Error message");

  char* output = _capture_end(&cap);
  TEST_ASSERT_NOT_NULL(output);
  printf("  Output (level=WARN): [%s]\n", output);

  TEST_ASSERT_NULL(strstr(output, "Debug message"));
  TEST_ASSERT_NULL(strstr(output, "Info message"));

  TEST_ASSERT_TRUE(strstr(output, "Warning message") != NULL);
  TEST_ASSERT_TRUE(strstr(output, "Error message") != NULL);

  _capture_free(&cap);
}

void test_log_with_markup(void)
{
  _capture_t cap = _capture_start();
  TEST_ASSERT_NOT_NULL(cap.file);

  bebopc_log_ctx_t test_ctx;
  bebopc_log_ctx_init(&test_ctx, false);
  test_ctx.output = cap.file;

  bebopc_log(&test_ctx, BEBOPC_LOG_INFO, "[red]Error:[/] Something went wrong");

  char* output = _capture_end(&cap);
  TEST_ASSERT_NOT_NULL(output);
  printf("  Output (no colors): [%s]\n", output);
  TEST_ASSERT_TRUE(strstr(output, "Error: Something went wrong") != NULL);

  TEST_ASSERT_NULL(strstr(output, "[red]"));
  TEST_ASSERT_NULL(strstr(output, "[/]"));

  _capture_free(&cap);
}

void test_log_with_timestamp(void)
{
  _capture_t cap = _capture_start();
  TEST_ASSERT_NOT_NULL(cap.file);

  bebopc_log_ctx_t test_ctx;
  bebopc_log_ctx_init(&test_ctx, false);
  test_ctx.output = cap.file;
  test_ctx.show_timestamp = true;

  bebopc_log(&test_ctx, BEBOPC_LOG_INFO, "Timestamped message");

  char* output = _capture_end(&cap);
  TEST_ASSERT_NOT_NULL(output);
  printf("  Output (timestamp): [%s]\n", output);
  TEST_ASSERT_TRUE(strstr(output, "Timestamped message") != NULL);

  TEST_ASSERT_TRUE(strstr(output, "20") != NULL);

  _capture_free(&cap);
}

void test_log_with_source(void)
{
  _capture_t cap = _capture_start();
  TEST_ASSERT_NOT_NULL(cap.file);

  bebopc_log_ctx_t test_ctx;
  bebopc_log_ctx_init(&test_ctx, false);
  test_ctx.output = cap.file;
  test_ctx.show_source = true;

  bebopc_log_src(&test_ctx, BEBOPC_LOG_INFO, __FILE__, __LINE__, "Source logged");

  char* output = _capture_end(&cap);
  TEST_ASSERT_NOT_NULL(output);
  printf("  Output (source): [%s]\n", output);
  TEST_ASSERT_TRUE(strstr(output, "Source logged") != NULL);

  TEST_ASSERT_TRUE(strstr(output, "test_bebopc_log.c") != NULL);

  _capture_free(&cap);
}

void test_markup_output(void)
{
  _capture_t cap = _capture_start();
  TEST_ASSERT_NOT_NULL(cap.file);

  bebopc_log_ctx_t test_ctx;
  bebopc_log_ctx_init(&test_ctx, false);
  test_ctx.output = cap.file;

  bebopc_markup(&test_ctx, "No newline here");
  bebopc_markup(&test_ctx, " - continued");

  char* output = _capture_end(&cap);
  TEST_ASSERT_NOT_NULL(output);
  printf("  Output (markup): [%s]\n", output);
  TEST_ASSERT_EQUAL_STRING("No newline here - continued", output);

  _capture_free(&cap);
}

void test_markup_line_output(void)
{
  _capture_t cap = _capture_start();
  TEST_ASSERT_NOT_NULL(cap.file);

  bebopc_log_ctx_t test_ctx;
  bebopc_log_ctx_init(&test_ctx, false);
  test_ctx.output = cap.file;

  bebopc_markup_line(&test_ctx, "Line one");
  bebopc_markup_line(&test_ctx, "Line two");

  char* output = _capture_end(&cap);
  TEST_ASSERT_NOT_NULL(output);
  printf("  Output (markup_line): [%s]\n", output);
  TEST_ASSERT_TRUE(strstr(output, "Line one\n") != NULL);
  TEST_ASSERT_TRUE(strstr(output, "Line two\n") != NULL);

  _capture_free(&cap);
}

void test_log_with_colors(void);

void test_log_with_colors(void)
{
  _capture_t cap = _capture_start();
  TEST_ASSERT_NOT_NULL(cap.file);

  bebopc_log_ctx_t test_ctx;
  bebopc_log_ctx_init(&test_ctx, true);
  test_ctx.output = cap.file;

  bebopc_log(&test_ctx, BEBOPC_LOG_ERROR, "[bold]Critical:[/] System failure");

  char* output = _capture_end(&cap);
  TEST_ASSERT_NOT_NULL(output);
  printf("  Output (colors): [%s]\n", output);
  TEST_ASSERT_TRUE(strstr(output, "Critical:") != NULL);
  TEST_ASSERT_TRUE(strstr(output, "System failure") != NULL);

  TEST_ASSERT_TRUE(strstr(output, "\x1b[") != NULL);

  _capture_free(&cap);
}

void test_markup_null_input(void);
void test_markup_empty_string(void);
void test_markup_only_tags(void);
void test_markup_invalid_hex(void);
void test_markup_invalid_rgb(void);

void test_markup_null_input(void)
{
  char* result = bebopc_markup_strip(NULL);
  TEST_ASSERT_NULL(result);

  result = bebopc_markup_render(NULL, true);
  TEST_ASSERT_NULL(result);
}

void test_markup_empty_string(void)
{
  char* result = bebopc_markup_strip("");
  TEST_ASSERT_NOT_NULL(result);
  TEST_ASSERT_EQUAL_STRING("", result);
  free(result);
}

void test_markup_only_tags(void)
{
  char* result = bebopc_markup_strip("[red][/]");
  TEST_ASSERT_NOT_NULL(result);
  TEST_ASSERT_EQUAL_STRING("", result);
  free(result);
}

void test_markup_invalid_hex(void)
{
  char* result = bebopc_markup_render("[#gggggg]Text[/]", true);
  TEST_ASSERT_NOT_NULL(result);
  TEST_ASSERT_TRUE(strstr(result, "Text") != NULL);

  TEST_ASSERT_NULL(strstr(result, "\x1b[38;2;"));
  free(result);
}

void test_markup_invalid_rgb(void)
{
  char* result = bebopc_markup_render("[rgb(300,0,0)]Text[/]", true);
  TEST_ASSERT_NOT_NULL(result);
  TEST_ASSERT_TRUE(strstr(result, "Text") != NULL);
  free(result);
}

int main(void)
{
  UNITY_BEGIN();

  RUN_TEST(test_log_level_name);
  RUN_TEST(test_log_level_color);
  RUN_TEST(test_log_level_parse);
  RUN_TEST(test_log_level_parse_case_insensitive);

  RUN_TEST(test_markup_strip_plain_text);
  RUN_TEST(test_markup_strip_colors);
  RUN_TEST(test_markup_strip_styles);
  RUN_TEST(test_markup_strip_nested);
  RUN_TEST(test_markup_strip_escaped_brackets);
  RUN_TEST(test_markup_strip_unclosed_tag);

  RUN_TEST(test_markup_render_no_colors);
  RUN_TEST(test_markup_render_with_colors);
  RUN_TEST(test_markup_render_background);
  RUN_TEST(test_markup_render_combined);
  RUN_TEST(test_markup_render_hex_color);
  RUN_TEST(test_markup_render_rgb_color);
  RUN_TEST(test_markup_render_link);
  RUN_TEST(test_markup_render_bright_colors);

  RUN_TEST(test_markup_render_all_styles);

  RUN_TEST(test_log_ctx_init);
  RUN_TEST(test_log_ctx_defaults);

  RUN_TEST(test_log_basic_output);
  RUN_TEST(test_log_level_filtering);
  RUN_TEST(test_log_with_markup);
  RUN_TEST(test_log_with_colors);
  RUN_TEST(test_log_with_timestamp);
  RUN_TEST(test_log_with_source);
  RUN_TEST(test_markup_output);
  RUN_TEST(test_markup_line_output);

  RUN_TEST(test_markup_null_input);
  RUN_TEST(test_markup_empty_string);
  RUN_TEST(test_markup_only_tags);
  RUN_TEST(test_markup_invalid_hex);
  RUN_TEST(test_markup_invalid_rgb);

  return UNITY_END();
}
