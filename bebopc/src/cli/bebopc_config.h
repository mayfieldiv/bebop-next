#ifndef BEBOPC_CONFIG_H
#define BEBOPC_CONFIG_H

#include <stdbool.h>
#include <stdint.h>

#include "../bebopc.h"
#include "../bebopc_error.h"
#include "bebopc_cli.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
  char* key;
  char* value;
} bebopc_kv_t;

typedef struct {
  char* name;
  char* out_dir;
  char* path;
  char* parameter;
  bebopc_kv_t* options;
  uint32_t option_count;
} bebopc_plugin_t;

typedef struct {
  char** exclude_directories;
  uint32_t exclude_directory_count;
  char** exclude_files;
  uint32_t exclude_file_count;
  bool preserve_output;
  bool no_emit;
} bebopc_watch_options_t;

typedef struct bebopc_config {
  char** sources;
  uint32_t source_count;

  char** exclude;
  uint32_t exclude_count;

  char** include;
  uint32_t include_count;

  bebopc_plugin_t* plugins;
  uint32_t plugin_count;

  bebopc_kv_t* options;
  uint32_t option_count;

  bebopc_watch_options_t watch;

  cli_color_mode_t color;
  cli_format_t format;
  bool verbose;
  bool quiet;
  bool no_warn;
  bool trace;
  bool emit_source_info;

  char* config_path;
  char* project_root;
} bebopc_config_t;

void bebopc_config_init(bebopc_config_t* cfg);
void bebopc_config_cleanup(bebopc_config_t* cfg);

bebopc_error_code_t bebopc_config_load(bebopc_config_t* cfg,
                                       bebopc_ctx_t* ctx,
                                       const char* explicit_path);

bebopc_error_code_t bebopc_config_merge_cli(bebopc_config_t* cfg,
                                            bebopc_ctx_t* ctx,
                                            const cli_args_t* args);

char* bebopc_config_find(const char* start_dir);

bebopc_error_code_t bebopc_config_add_source(bebopc_config_t* cfg,
                                             bebopc_ctx_t* ctx,
                                             const char* pattern);

bebopc_error_code_t bebopc_config_add_exclude(bebopc_config_t* cfg,
                                              bebopc_ctx_t* ctx,
                                              const char* pattern);

bebopc_error_code_t bebopc_config_add_include(bebopc_config_t* cfg,
                                              bebopc_ctx_t* ctx,
                                              const char* path);

bebopc_error_code_t bebopc_config_add_plugin(bebopc_config_t* cfg,
                                             bebopc_ctx_t* ctx,
                                             const char* name,
                                             const char* out_dir);

bebopc_error_code_t bebopc_config_add_option(bebopc_config_t* cfg,
                                             bebopc_ctx_t* ctx,
                                             const char* key,
                                             const char* value);

#ifdef __cplusplus
}
#endif

#endif
