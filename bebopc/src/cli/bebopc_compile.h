#ifndef BEBOPC_COMPILE_H
#define BEBOPC_COMPILE_H

#include <bebop.h>

#include "../bebopc.h"

typedef struct {
  bebop_context_t* beb_ctx;
  bebop_parse_result_t* result;
  bebop_descriptor_t* desc;
  uint32_t error_count;
  uint32_t warning_count;
  uint32_t file_count;
} bebopc_compile_result_t;

const char** bebopc_collect_files(bebopc_ctx_t* ctx,
                                  char** cli_files,
                                  uint32_t cli_file_count,
                                  uint32_t* out_count);

bebopc_error_code_t bebopc_compile(bebopc_ctx_t* ctx,
                                   const char** files,
                                   uint32_t file_count,
                                   bebopc_compile_result_t* out);

void bebopc_compile_cleanup(bebopc_compile_result_t* result);

uint32_t bebopc_render_diagnostics(bebopc_ctx_t* ctx,
                                   bebop_parse_result_t* result);

#endif
