#include "bebopc.h"

#include <string.h>

void bebopc_ctx_init(bebopc_ctx_t* ctx)
{
  memset(ctx, 0, sizeof(*ctx));
  bebopc_error_init(&ctx->errors);
}

void bebopc_ctx_cleanup(bebopc_ctx_t* ctx)
{
  bebopc_error_cleanup(&ctx->errors);
}
