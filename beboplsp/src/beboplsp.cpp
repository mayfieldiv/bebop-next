#include "beboplsp.h"
#include "server.hpp"
#include <lsp/io/standardio.h>
#include <vector>
#include <string>

struct beboplsp_server {
    std::unique_ptr<beboplsp::Server> server;
};

extern "C" {

beboplsp_server_t* beboplsp_server_create(const beboplsp_includes_t* includes)
{
    auto* s = new (std::nothrow) beboplsp_server();
    if (!s) {
        return nullptr;
    }

    std::vector<std::string> includePaths;
    if (includes && includes->paths) {
        for (uint32_t i = 0; i < includes->count; ++i) {
            if (includes->paths[i]) {
                includePaths.emplace_back(includes->paths[i]);
            }
        }
    }

    s->server = std::make_unique<beboplsp::Server>(lsp::io::standardIO(), std::move(includePaths));

    return s;
}

void beboplsp_server_run(beboplsp_server_t* server)
{
    if (server && server->server) {
        server->server->run();
    }
}

void beboplsp_server_destroy(beboplsp_server_t* server)
{
    delete server;
}

} // extern "C"
