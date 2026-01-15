#ifndef BEBOPC_ERROR_H
#define BEBOPC_ERROR_H

#include <stdbool.h>
#include <stddef.h>

typedef enum {
  BEBOPC_OK = 0,
  BEBOPC_ERR_OUT_OF_MEMORY,
  BEBOPC_ERR_INVALID_ARG,
  BEBOPC_ERR_IO,
  BEBOPC_ERR_NOT_FOUND,
  BEBOPC_ERR_PERMISSION,
  BEBOPC_ERR_ALREADY_EXISTS,
  BEBOPC_ERR_PARSE,
  BEBOPC_ERR_SEMANTIC,
  BEBOPC_ERR_CODEGEN,
  BEBOPC_ERR_INTERNAL,
} bebopc_error_code_t;

typedef struct {
  bebopc_error_code_t code;
  char* message;
  const char* file;
  int line;
} bebopc_error_t;

typedef struct {
  bebopc_error_t* errors;
  size_t count;
  size_t capacity;
} bebopc_error_ctx_t;

void bebopc_error_init(bebopc_error_ctx_t* ctx);
void bebopc_error_cleanup(bebopc_error_ctx_t* ctx);

void bebopc_error_add(bebopc_error_ctx_t* ctx,
                      bebopc_error_code_t code,
                      const char* file,
                      int line,
                      const char* fmt,
                      ...) __attribute__((format(printf, 5, 6)));

void bebopc_error_addf(bebopc_error_ctx_t* ctx,
                       bebopc_error_code_t code,
                       const char* fmt,
                       ...) __attribute__((format(printf, 3, 4)));

#define BEBOPC_ERROR(ctx, code, ...) \
  bebopc_error_add((ctx), (code), __FILE__, __LINE__, __VA_ARGS__)

bool bebopc_error_has(const bebopc_error_ctx_t* ctx);
size_t bebopc_error_count(const bebopc_error_ctx_t* ctx);
const bebopc_error_t* bebopc_error_get(const bebopc_error_ctx_t* ctx,
                                       size_t index);
const bebopc_error_t* bebopc_error_last(const bebopc_error_ctx_t* ctx);
void bebopc_error_clear(bebopc_error_ctx_t* ctx);

const char* bebopc_error_code_str(bebopc_error_code_t code);

#endif
