#ifndef BEBOPC_H
#define BEBOPC_H

#include <bebop.h>

#include "bebopc_error.h"

typedef struct bebopc_config bebopc_config_t;

typedef struct {
  bebopc_error_ctx_t errors;
  bebop_host_t host;
  bebopc_config_t* cfg;
} bebopc_ctx_t;

void bebopc_ctx_init(bebopc_ctx_t* ctx);
void bebopc_ctx_cleanup(bebopc_ctx_t* ctx);

#endif
