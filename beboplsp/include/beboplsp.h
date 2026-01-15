#ifndef BEBOPLSP_H
#define BEBOPLSP_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct beboplsp_server beboplsp_server_t;

typedef struct {
    const char** paths;
    uint32_t count;
} beboplsp_includes_t;

/**
 * Create an LSP server instance.
 * The server uses stdin/stdout for communication.
 */
beboplsp_server_t* beboplsp_server_create(const beboplsp_includes_t* includes);

/**
 * Run the LSP server message loop.
 * This blocks until the server receives an exit notification.
 */
void beboplsp_server_run(beboplsp_server_t* server);

/**
 * Destroy the LSP server and free resources.
 */
void beboplsp_server_destroy(beboplsp_server_t* server);

#ifdef __cplusplus
}
#endif

#endif /* BEBOPLSP_H */
