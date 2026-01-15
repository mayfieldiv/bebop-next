#ifndef BEBOPC_LOG_H
#define BEBOPC_LOG_H

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>

#if defined(__GNUC__) || defined(__clang__)
#define BEBOPC_PRINTF(fmt_idx, arg_idx) \
  __attribute__((format(printf, fmt_idx, arg_idx)))
#else
#define BEBOPC_PRINTF(fmt_idx, arg_idx)
#endif

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
  BEBOPC_LOG_TRACE = 0,
  BEBOPC_LOG_DEBUG = 1,
  BEBOPC_LOG_INFO = 2,
  BEBOPC_LOG_WARN = 3,
  BEBOPC_LOG_ERROR = 4,
  BEBOPC_LOG_FATAL = 5,
  BEBOPC_LOG_OFF = 6
} bebopc_log_level_t;

typedef struct bebopc_log_ctx {
  bebopc_log_level_t level;
  bool colors_enabled;
  bool show_timestamp;
  bool show_level;
  bool show_source;
  FILE* output;
} bebopc_log_ctx_t;

void bebopc_log_ctx_init(bebopc_log_ctx_t* ctx, bool colors_enabled);

const char* bebopc_log_level_name(bebopc_log_level_t level);

const char* bebopc_log_level_color(bebopc_log_level_t level);

bebopc_log_level_t bebopc_log_level_parse(const char* name);

BEBOPC_PRINTF(3, 4)
void bebopc_log(bebopc_log_ctx_t* ctx,
                bebopc_log_level_t level,
                const char* fmt,
                ...);
void bebopc_logv(bebopc_log_ctx_t* ctx,
                 bebopc_log_level_t level,
                 const char* fmt,
                 va_list args);

BEBOPC_PRINTF(5, 6)
void bebopc_log_src(bebopc_log_ctx_t* ctx,
                    bebopc_log_level_t level,
                    const char* file,
                    int line,
                    const char* fmt,
                    ...);

BEBOPC_PRINTF(2, 3)
void bebopc_markup(bebopc_log_ctx_t* ctx, const char* fmt, ...);
void bebopc_markupv(bebopc_log_ctx_t* ctx, const char* fmt, va_list args);

BEBOPC_PRINTF(2, 3)
void bebopc_markup_line(bebopc_log_ctx_t* ctx, const char* fmt, ...);

BEBOPC_PRINTF(3, 4)
void bebopc_markup_to(FILE* stream, bool use_colors, const char* fmt, ...);

char* bebopc_markup_strip(const char* markup);

char* bebopc_markup_render(const char* markup, bool use_colors);

typedef enum {
  BEBOPC_TABLE_BORDER_NONE,
  BEBOPC_TABLE_BORDER_ROUNDED,
  BEBOPC_TABLE_BORDER_SQUARE,
  BEBOPC_TABLE_BORDER_HEAVY,
  BEBOPC_TABLE_BORDER_DOUBLE,
  BEBOPC_TABLE_BORDER_ASCII
} bebopc_table_border_t;

typedef struct bebopc_table bebopc_table_t;

bebopc_table_t* bebopc_table_new(void);
void bebopc_table_free(bebopc_table_t* t);
void bebopc_table_set_border(bebopc_table_t* t, bebopc_table_border_t border);
void bebopc_table_set_border_color(bebopc_table_t* t, const char* color);
void bebopc_table_set_title(bebopc_table_t* t, const char* title);
void bebopc_table_add_column(bebopc_table_t* t, const char* header);
void bebopc_table_add_row(bebopc_table_t* t, ...);
void bebopc_table_render(bebopc_table_t* t, bebopc_log_ctx_t* ctx);

#ifndef BEBOPC_LOG_CTX
#define BEBOPC_LOG_CTX g_log_ctx
#endif

#define LOG_TRACE(...) \
  bebopc_log_src( \
      BEBOPC_LOG_CTX, BEBOPC_LOG_TRACE, __FILE__, __LINE__, __VA_ARGS__)
#define LOG_DEBUG(...) \
  bebopc_log_src( \
      BEBOPC_LOG_CTX, BEBOPC_LOG_DEBUG, __FILE__, __LINE__, __VA_ARGS__)
#define LOG_INFO(...) \
  bebopc_log_src( \
      BEBOPC_LOG_CTX, BEBOPC_LOG_INFO, __FILE__, __LINE__, __VA_ARGS__)
#define LOG_WARN(...) \
  bebopc_log_src( \
      BEBOPC_LOG_CTX, BEBOPC_LOG_WARN, __FILE__, __LINE__, __VA_ARGS__)
#define LOG_ERROR(...) \
  bebopc_log_src( \
      BEBOPC_LOG_CTX, BEBOPC_LOG_ERROR, __FILE__, __LINE__, __VA_ARGS__)
#define LOG_FATAL(...) \
  bebopc_log_src( \
      BEBOPC_LOG_CTX, BEBOPC_LOG_FATAL, __FILE__, __LINE__, __VA_ARGS__)

#define log_trace(...) bebopc_log(BEBOPC_LOG_CTX, BEBOPC_LOG_TRACE, __VA_ARGS__)
#define log_debug(...) bebopc_log(BEBOPC_LOG_CTX, BEBOPC_LOG_DEBUG, __VA_ARGS__)
#define log_info(...) bebopc_log(BEBOPC_LOG_CTX, BEBOPC_LOG_INFO, __VA_ARGS__)
#define log_warn(...) bebopc_log(BEBOPC_LOG_CTX, BEBOPC_LOG_WARN, __VA_ARGS__)
#define log_error(...) bebopc_log(BEBOPC_LOG_CTX, BEBOPC_LOG_ERROR, __VA_ARGS__)
#define log_fatal(...) bebopc_log(BEBOPC_LOG_CTX, BEBOPC_LOG_FATAL, __VA_ARGS__)

#define markup(...) bebopc_markup(BEBOPC_LOG_CTX, __VA_ARGS__)
#define markup_line(...) bebopc_markup_line(BEBOPC_LOG_CTX, __VA_ARGS__)

#ifdef __cplusplus
}
#endif

#endif
