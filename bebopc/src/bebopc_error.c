#include "bebopc_error.h"

#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "bebopc_utils.h"

void bebopc_error_init(bebopc_error_ctx_t* ctx)
{
  ctx->errors = NULL;
  ctx->count = 0;
  ctx->capacity = 0;
}

void bebopc_error_cleanup(bebopc_error_ctx_t* ctx)
{
  if (!ctx) {
    return;
  }

  for (size_t i = 0; i < ctx->count; i++) {
    free(ctx->errors[i].message);
  }

  free(ctx->errors);
  ctx->errors = NULL;
  ctx->count = 0;
  ctx->capacity = 0;
}

static bool _bebopc_error_grow(bebopc_error_ctx_t* ctx)
{
  size_t new_cap = ctx->capacity ? ctx->capacity * 2 : 8;
  bebopc_error_t* new_errors =
      (bebopc_error_t*)realloc(ctx->errors, new_cap * sizeof(bebopc_error_t));
  if (!new_errors) {
    return false;
  }
  ctx->errors = new_errors;
  ctx->capacity = new_cap;
  return true;
}

static char* _bebopc_error_vformat(const char* fmt, va_list ap)
{
  va_list ap2;
  va_copy(ap2, ap);
#ifdef __clang__
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wformat-nonliteral"
#endif
  int len = vsnprintf(NULL, 0, fmt, ap2);
  va_end(ap2);

  if (len < 0) {
    return NULL;
  }

  char* buf = (char*)malloc((size_t)len + 1);
  if (!buf) {
    return NULL;
  }

  vsnprintf(buf, (size_t)len + 1, fmt, ap);
#ifdef __clang__
#pragma clang diagnostic pop
#endif
  return buf;
}

void bebopc_error_add(
    bebopc_error_ctx_t* ctx,
    bebopc_error_code_t code,
    const char* file,
    int line,
    const char* fmt,
    ...
)
{
  if (ctx->count >= ctx->capacity) {
    if (!_bebopc_error_grow(ctx)) {
      return;
    }
  }

  va_list ap;
  va_start(ap, fmt);
  char* message = _bebopc_error_vformat(fmt, ap);
  va_end(ap);

  if (!message) {
    message = bebopc_strdup("(out of memory formatting error)");
  }

  bebopc_error_t* err = &ctx->errors[ctx->count++];
  err->code = code;
  err->message = message;
  err->file = file;
  err->line = line;
}

void bebopc_error_addf(bebopc_error_ctx_t* ctx, bebopc_error_code_t code, const char* fmt, ...)
{
  if (ctx->count >= ctx->capacity) {
    if (!_bebopc_error_grow(ctx)) {
      return;
    }
  }

  va_list ap;
  va_start(ap, fmt);
  char* message = _bebopc_error_vformat(fmt, ap);
  va_end(ap);

  bebopc_error_t* err = &ctx->errors[ctx->count++];
  err->code = code;
  err->message = message;
  err->file = NULL;
  err->line = 0;
}

bool bebopc_error_has(const bebopc_error_ctx_t* ctx)
{
  return ctx->count > 0;
}

size_t bebopc_error_count(const bebopc_error_ctx_t* ctx)
{
  return ctx->count;
}

const bebopc_error_t* bebopc_error_get(const bebopc_error_ctx_t* ctx, size_t index)
{
  if (index >= ctx->count) {
    return NULL;
  }
  return &ctx->errors[index];
}

const bebopc_error_t* bebopc_error_last(const bebopc_error_ctx_t* ctx)
{
  if (ctx->count == 0) {
    return NULL;
  }
  return &ctx->errors[ctx->count - 1];
}

void bebopc_error_clear(bebopc_error_ctx_t* ctx)
{
  for (size_t i = 0; i < ctx->count; i++) {
    free(ctx->errors[i].message);
  }
  ctx->count = 0;
}

const char* bebopc_error_code_str(bebopc_error_code_t code)
{
  switch (code) {
    case BEBOPC_OK:
      return "ok";
    case BEBOPC_ERR_OUT_OF_MEMORY:
      return "out of memory";
    case BEBOPC_ERR_INVALID_ARG:
      return "invalid argument";
    case BEBOPC_ERR_IO:
      return "i/o error";
    case BEBOPC_ERR_NOT_FOUND:
      return "not found";
    case BEBOPC_ERR_PERMISSION:
      return "permission denied";
    case BEBOPC_ERR_ALREADY_EXISTS:
      return "already exists";
    case BEBOPC_ERR_PARSE:
      return "parse error";
    case BEBOPC_ERR_SEMANTIC:
      return "semantic error";
    case BEBOPC_ERR_CODEGEN:
      return "code generation error";
    case BEBOPC_ERR_INTERNAL:
      return "internal error";
    default:
      return "unknown error";
  }
}
