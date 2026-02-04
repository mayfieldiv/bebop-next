#include "bebopc_runner.h"

#include "../bebopc_dir.h"
#include "../bebopc_error.h"
#include "../bebopc_log.h"
#include "../bebopc_utils.h"

extern bebopc_log_ctx_t* g_log_ctx;

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef BEBOPC_WINDOWS
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#else
#include <sys/wait.h>
#include <unistd.h>
#endif

bebopc_error_code_t bebopc_runner_init(
    bebopc_runner_t* r,
    bebopc_ctx_t* ctx,
    bebop_context_t* beb_ctx,
    bebop_descriptor_t* desc,
    const char** files,
    uint32_t file_count
)
{
  if (!r) {
    return BEBOPC_ERR_INVALID_ARG;
  }

  memset(r, 0, sizeof(*r));

  if (!ctx || !ctx->cfg) {
    return BEBOPC_ERR_INVALID_ARG;
  }
  if (!beb_ctx) {
    return BEBOPC_ERR_INVALID_ARG;
  }
  if (!desc) {
    return BEBOPC_ERR_INVALID_ARG;
  }
  if (file_count > 0 && !files) {
    return BEBOPC_ERR_INVALID_ARG;
  }

  r->ctx = ctx;
  r->beb_ctx = beb_ctx;
  r->desc = desc;
  r->input_files = files;
  r->input_file_count = file_count;

  return BEBOPC_OK;
}

void bebopc_runner_cleanup(bebopc_runner_t* r)
{
  if (!r) {
    return;
  }
  if (r->desc) {
    bebop_descriptor_free(r->desc);
  }
  if (r->beb_ctx) {
    bebop_context_destroy(r->beb_ctx);
  }
  bebopc_files_free(r->input_files, r->input_file_count);
  memset(r, 0, sizeof(*r));
}

static void _report_plugin_diagnostics(
    bebopc_runner_t* r, const char* gen_name, bebop_plugin_response_t* resp
)
{
  uint32_t diag_count = bebop_plugin_response_diagnostic_count(resp);
  for (uint32_t i = 0; i < diag_count; i++) {
    bebop_plugin_severity_t sev = bebop_plugin_response_diagnostic_severity(resp, i);
    const char* text = bebop_plugin_response_diagnostic_text(resp, i);
    const char* file = bebop_plugin_response_diagnostic_file(resp, i);

    if (!text) {
      continue;
    }

    switch (sev) {
      case BEBOP_PLUGIN_SEV_WARNING:
        if (file) {
          log_warn("[%s] %s: %s", gen_name, file, text);
        } else {
          log_warn("[%s] %s", gen_name, text);
        }
        break;
      case BEBOP_PLUGIN_SEV_INFO:
        if (file) {
          log_info("[%s] %s: %s", gen_name, file, text);
        } else {
          log_info("[%s] %s", gen_name, text);
        }
        break;
      case BEBOP_PLUGIN_SEV_HINT:
        if (file) {
          log_debug("[%s] hint: %s: %s", gen_name, file, text);
        } else {
          log_debug("[%s] hint: %s", gen_name, text);
        }
        break;
      default:
        if (file) {
          BEBOPC_ERROR(&r->ctx->errors, BEBOPC_ERR_INTERNAL, "[%s] %s: %s", gen_name, file, text);
        } else {
          BEBOPC_ERROR(&r->ctx->errors, BEBOPC_ERR_INTERNAL, "[%s] %s", gen_name, text);
        }
        break;
    }
  }
}

static bebopc_error_code_t _write_generated_files(
    bebopc_runner_t* r, const bebopc_plugin_t* gen, bebop_plugin_response_t* resp
)
{
  uint32_t file_count = bebop_plugin_response_file_count(resp);
  bebopc_error_code_t result = BEBOPC_OK;

  for (uint32_t i = 0; i < file_count; i++) {
    const char* name = bebop_plugin_response_file_name(resp, i);
    const char* content = bebop_plugin_response_file_content(resp, i);
    if (!name || !content) {
      continue;
    }
    if (bebop_plugin_response_file_insertion_point(resp, i)) {
      continue;
    }

    char* full_path = bebopc_path_join(gen->out_dir, name);
    if (!full_path) {
      result = BEBOPC_ERR_OUT_OF_MEMORY;
      continue;
    }

    size_t content_len = bebopc_strlen(content);
    if (!bebopc_file_write(r->ctx, full_path, content, content_len)) {
      BEBOPC_ERROR(
          &r->ctx->errors, BEBOPC_ERR_IO, "[%s] failed to write: %s", gen->name, full_path
      );
      result = BEBOPC_ERR_IO;
    }
    free(full_path);
  }

  for (uint32_t i = 0; i < file_count; i++) {
    const char* name = bebop_plugin_response_file_name(resp, i);
    const char* content = bebop_plugin_response_file_content(resp, i);
    const char* point = bebop_plugin_response_file_insertion_point(resp, i);
    if (!name || !content || !point) {
      continue;
    }

    char* full_path = bebopc_path_join(gen->out_dir, name);
    if (!full_path) {
      result = BEBOPC_ERR_OUT_OF_MEMORY;
      continue;
    }

    size_t file_size = 0;
    char* file_content = bebopc_file_read(r->ctx, full_path, &file_size);
    if (!file_content) {
      BEBOPC_ERROR(
          &r->ctx->errors,
          BEBOPC_ERR_IO,
          "[%s] insertion point target not found: %s",
          gen->name,
          name
      );
      free(full_path);
      result = BEBOPC_ERR_IO;
      continue;
    }

    char marker[256];
    snprintf(marker, sizeof(marker), "// @@bebop_insertion_point(%s)", point);

    char* insert_pos = strstr(file_content, marker);
    if (!insert_pos) {
      BEBOPC_ERROR(
          &r->ctx->errors,
          BEBOPC_ERR_NOT_FOUND,
          "[%s] insertion point not found: %s in %s",
          gen->name,
          point,
          name
      );
      free(file_content);
      free(full_path);
      result = BEBOPC_ERR_NOT_FOUND;
      continue;
    }

    size_t before_len = (size_t)(insert_pos - file_content);
    size_t insert_len = bebopc_strlen(content);
    size_t marker_len = bebopc_strlen(marker);
    size_t after_len = file_size - before_len - marker_len;
    size_t new_size = before_len + insert_len + marker_len + after_len;

    char* new_content = malloc(new_size + 1);
    if (!new_content) {
      free(file_content);
      free(full_path);
      result = BEBOPC_ERR_OUT_OF_MEMORY;
      continue;
    }

    memcpy(new_content, file_content, before_len);
    memcpy(new_content + before_len, content, insert_len);
    memcpy(new_content + before_len + insert_len, insert_pos, marker_len + after_len);
    new_content[new_size] = '\0';

    if (!bebopc_file_write(r->ctx, full_path, new_content, new_size)) {
      BEBOPC_ERROR(
          &r->ctx->errors, BEBOPC_ERR_IO, "[%s] failed to write insertion: %s", gen->name, full_path
      );
      result = BEBOPC_ERR_IO;
    }

    free(new_content);
    free(file_content);
    free(full_path);
  }

  return result;
}

static bool _option_overridden(const bebopc_plugin_t* gen, const char* key)
{
  for (uint32_t i = 0; i < gen->option_count; i++) {
    if (strcmp(gen->options[i].key, key) == 0) {
      return true;
    }
  }
  return false;
}

static bebopc_error_code_t _invoke_plugin(bebopc_runner_t* r, const bebopc_plugin_t* gen)
{
  bebopc_error_code_t result = BEBOPC_OK;
  char* plugin_path = NULL;
  bebop_plugin_request_builder_t* builder = NULL;
  bebop_plugin_request_t* req = NULL;
  const uint8_t* req_buf = NULL;
  size_t req_len = 0;
  bebopc_process_t* proc = NULL;
  uint8_t* resp_buf = NULL;
  size_t resp_len = 0;
  bebop_plugin_response_t* resp = NULL;
  int exit_code = 0;

  if (bebopc_file_is_file(gen->out_dir)) {
    BEBOPC_ERROR(
        &r->ctx->errors,
        BEBOPC_ERR_INVALID_ARG,
        "[%s] output path is a file, not a directory: %s",
        gen->name,
        gen->out_dir
    );
    result = BEBOPC_ERR_INVALID_ARG;
    goto cleanup;
  }

  if (gen->path) {
    plugin_path = bebopc_strdup(gen->path);
  } else {
    plugin_path = bebopc_find_plugin(gen->name);
  }
  if (!plugin_path) {
    BEBOPC_ERROR(
        &r->ctx->errors, BEBOPC_ERR_NOT_FOUND, "plugin not found: bebopc-gen-%s", gen->name
    );
    result = BEBOPC_ERR_NOT_FOUND;
    goto cleanup;
  }

  log_trace("[%s] using plugin: %s", gen->name, plugin_path);

  builder = bebop_plugin_request_builder_create(&r->ctx->host.allocator);
  if (!builder) {
    BEBOPC_ERROR(
        &r->ctx->errors, BEBOPC_ERR_OUT_OF_MEMORY, "[%s] failed to create request builder", gen->name
    );
    result = BEBOPC_ERR_OUT_OF_MEMORY;
    goto cleanup;
  }

  bebop_plugin_request_builder_set_version(builder, bebop_version());
  bebop_plugin_request_builder_set_parameter(builder, gen->out_dir);
  bebop_plugin_request_builder_set_descriptor(builder, r->desc);

  for (uint32_t i = 0; i < r->input_file_count; i++) {
    bebop_plugin_request_builder_add_file(builder, r->input_files[i]);
  }

  const bebopc_config_t* cfg = r->ctx->cfg;
  for (uint32_t i = 0; i < cfg->option_count; i++) {
    if (!_option_overridden(gen, cfg->options[i].key)) {
      bebop_plugin_request_builder_add_option(builder, cfg->options[i].key, cfg->options[i].value);
    }
  }
  for (uint32_t i = 0; i < gen->option_count; i++) {
    bebop_plugin_request_builder_add_option(builder, gen->options[i].key, gen->options[i].value);
  }

  req = bebop_plugin_request_builder_finish(builder);
  builder = NULL;
  if (!req) {
    BEBOPC_ERROR(
        &r->ctx->errors, BEBOPC_ERR_OUT_OF_MEMORY, "[%s] failed to build request", gen->name
    );
    result = BEBOPC_ERR_OUT_OF_MEMORY;
    goto cleanup;
  }

  bebop_status_t status = bebop_plugin_request_encode(r->beb_ctx, req, &req_buf, &req_len);
  if (status != BEBOP_OK) {
    BEBOPC_ERROR(
        &r->ctx->errors,
        BEBOPC_ERR_INTERNAL,
        "[%s] failed to encode request (status=%d)",
        gen->name,
        status
    );
    result = BEBOPC_ERR_INTERNAL;
    goto cleanup;
  }

  log_trace("[%s] request size: %zu bytes", gen->name, req_len);

  proc = bebopc_process_spawn(plugin_path);
  if (!proc) {
    BEBOPC_ERROR(&r->ctx->errors, BEBOPC_ERR_IO, "[%s] failed to spawn plugin process", gen->name);
    result = BEBOPC_ERR_IO;
    goto cleanup;
  }

  if (!bebopc_process_write(proc, req_buf, req_len)) {
    BEBOPC_ERROR(
        &r->ctx->errors, BEBOPC_ERR_IO, "[%s] failed to write request to plugin", gen->name
    );
    result = BEBOPC_ERR_IO;
    goto cleanup;
  }
  bebopc_process_close_stdin(proc);

  resp_buf = bebopc_process_read_all(proc, &resp_len);
  exit_code = bebopc_process_wait(proc);

  log_trace(
      "[%s] plugin exited with code %d, response size: %zu bytes", gen->name, exit_code, resp_len
  );

  if (!resp_buf || resp_len == 0) {
    if (exit_code != 0) {
      BEBOPC_ERROR(
          &r->ctx->errors,
          BEBOPC_ERR_INTERNAL,
          "[%s] plugin exited with code %d (no response)",
          gen->name,
          exit_code
      );
      result = BEBOPC_ERR_INTERNAL;
    }
    goto cleanup;
  }

  status = bebop_plugin_response_decode(r->beb_ctx, resp_buf, resp_len, &resp);
  if (status != BEBOP_OK || !resp) {
    BEBOPC_ERROR(
        &r->ctx->errors,
        BEBOPC_ERR_PARSE,
        "[%s] failed to decode plugin response (status=%d, len=%zu)",
        gen->name,
        status,
        resp_len
    );
    result = BEBOPC_ERR_PARSE;
    goto cleanup;
  }

  _report_plugin_diagnostics(r, gen->name, resp);

  const char* error = bebop_plugin_response_error(resp);
  if (error) {
    BEBOPC_ERROR(&r->ctx->errors, BEBOPC_ERR_INTERNAL, "[%s] plugin error: %s", gen->name, error);
    result = BEBOPC_ERR_INTERNAL;
    goto cleanup;
  }

  if (exit_code != 0) {
    BEBOPC_ERROR(
        &r->ctx->errors, BEBOPC_ERR_INTERNAL, "[%s] plugin exited with code %d", gen->name, exit_code
    );
    result = BEBOPC_ERR_INTERNAL;
    goto cleanup;
  }

  result = _write_generated_files(r, gen, resp);

cleanup:
  if (resp) {
    bebop_plugin_response_free(resp);
  }
  free(resp_buf);
  if (proc) {
    bebopc_process_free(proc);
  }
  if (req) {
    bebop_plugin_request_free(req);
  }
  free(plugin_path);
  return result;
}

bebopc_error_code_t bebopc_runner_generate(bebopc_runner_t* r)
{
  if (!r || !r->ctx || !r->ctx->cfg) {
    return BEBOPC_ERR_INVALID_ARG;
  }
  if (!r->beb_ctx || !r->desc) {
    return BEBOPC_ERR_INVALID_ARG;
  }

  const bebopc_config_t* cfg = r->ctx->cfg;
  if (cfg->plugin_count == 0) {
    log_warn("no plugins configured");
    return BEBOPC_OK;
  }

  log_info("Running %u plugin(s)...", cfg->plugin_count);

  bebopc_error_code_t result = BEBOPC_OK;
  for (uint32_t i = 0; i < cfg->plugin_count; i++) {
    const bebopc_plugin_t* gen = &cfg->plugins[i];
    log_trace("invoking plugin: %s -> %s", gen->name, gen->out_dir);

    bebopc_error_code_t err = _invoke_plugin(r, gen);
    if (err != BEBOPC_OK) {
      log_error("[%s] plugin failed", gen->name);
      result = err;
    }
  }

  return result;
}

char* bebopc_find_plugin(const char* name)
{
  if (!name) {
    return NULL;
  }

#ifdef BEBOPC_WINDOWS
  const char* ext = ".exe";
#else
  const char* ext = "";
#endif

  size_t name_len = bebopc_strlen("bebopc-gen-") + bebopc_strlen(name) + bebopc_strlen(ext) + 1;
  char* plugin_name = malloc(name_len);
  if (!plugin_name) {
    return NULL;
  }
  snprintf(plugin_name, name_len, "bebopc-gen-%s%s", name, ext);

  char* exe_path = bebopc_exe_path();
  if (exe_path) {
    char* exe_dir = bebopc_path_dirname(exe_path);
    free(exe_path);
    if (exe_dir) {
      char* sibling = bebopc_path_join(exe_dir, plugin_name);
      free(exe_dir);
      if (sibling && bebopc_file_is_file(sibling)) {
        free(plugin_name);
        return sibling;
      }
      free(sibling);
    }
  }

  const char* path_env = getenv("PATH");
  if (path_env) {
    char* path_copy = bebopc_strdup(path_env);
    if (path_copy) {
#ifdef BEBOPC_WINDOWS
      const char* sep = ";";
#else
      const char* sep = ":";
#endif
      char* save_ptr = NULL;
      char* dir = strtok_r(path_copy, sep, &save_ptr);
      while (dir) {
        char* full = bebopc_path_join(dir, plugin_name);
        if (full && bebopc_file_is_file(full)) {
          free(path_copy);
          free(plugin_name);
          return full;
        }
        free(full);
        dir = strtok_r(NULL, sep, &save_ptr);
      }
      free(path_copy);
    }
  }

  free(plugin_name);
  return NULL;
}

struct bebopc_process {
#ifdef BEBOPC_WINDOWS
  HANDLE process;
  HANDLE stdin_write;
  HANDLE stdout_read;
#else
  pid_t pid;
  int stdin_fd;
  int stdout_fd;
#endif
};

bebopc_process_t* bebopc_process_spawn(const char* exe)
{
  if (!exe) {
    return NULL;
  }

  bebopc_process_t* p = calloc(1, sizeof(bebopc_process_t));
  if (!p) {
    return NULL;
  }
#ifndef BEBOPC_WINDOWS
  p->stdin_fd = -1;
  p->stdout_fd = -1;
#endif

#ifdef BEBOPC_WINDOWS
  int wide_len = MultiByteToWideChar(CP_UTF8, 0, exe, -1, NULL, 0);
  if (wide_len <= 0) {
    free(p);
    return NULL;
  }
  wchar_t* wide_exe = malloc((size_t)wide_len * sizeof(wchar_t));
  if (!wide_exe) {
    free(p);
    return NULL;
  }
  MultiByteToWideChar(CP_UTF8, 0, exe, -1, wide_exe, wide_len);

  SECURITY_ATTRIBUTES sa = {.nLength = sizeof(sa), .bInheritHandle = TRUE};

  HANDLE stdin_read, stdout_write;
  if (!CreatePipe(&stdin_read, &p->stdin_write, &sa, 0)) {
    free(wide_exe);
    free(p);
    return NULL;
  }
  if (!CreatePipe(&p->stdout_read, &stdout_write, &sa, 0)) {
    CloseHandle(stdin_read);
    CloseHandle(p->stdin_write);
    free(wide_exe);
    free(p);
    return NULL;
  }

  SetHandleInformation(p->stdin_write, HANDLE_FLAG_INHERIT, 0);
  SetHandleInformation(p->stdout_read, HANDLE_FLAG_INHERIT, 0);

  STARTUPINFOW si = {.cb = sizeof(si), .dwFlags = STARTF_USESTDHANDLES};
  si.hStdInput = stdin_read;
  si.hStdOutput = stdout_write;
  si.hStdError = GetStdHandle(STD_ERROR_HANDLE);

  PROCESS_INFORMATION pi = {0};
  BOOL ok = CreateProcessW(wide_exe, NULL, NULL, NULL, TRUE, 0, NULL, NULL, &si, &pi);
  free(wide_exe);

  CloseHandle(stdin_read);
  CloseHandle(stdout_write);

  if (!ok) {
    CloseHandle(p->stdin_write);
    CloseHandle(p->stdout_read);
    free(p);
    return NULL;
  }

  CloseHandle(pi.hThread);
  p->process = pi.hProcess;

#else
  int stdin_pipe[2], stdout_pipe[2];
  if (pipe(stdin_pipe) < 0) {
    free(p);
    return NULL;
  }
  if (pipe(stdout_pipe) < 0) {
    close(stdin_pipe[0]);
    close(stdin_pipe[1]);
    free(p);
    return NULL;
  }

  pid_t pid = fork();
  if (pid < 0) {
    close(stdin_pipe[0]);
    close(stdin_pipe[1]);
    close(stdout_pipe[0]);
    close(stdout_pipe[1]);
    free(p);
    return NULL;
  }

  if (pid == 0) {
    dup2(stdin_pipe[0], STDIN_FILENO);
    dup2(stdout_pipe[1], STDOUT_FILENO);
    close(stdin_pipe[0]);
    close(stdin_pipe[1]);
    close(stdout_pipe[0]);
    close(stdout_pipe[1]);
    execl(exe, exe, (char*)NULL);
    _exit(127);
  }

  close(stdin_pipe[0]);
  close(stdout_pipe[1]);
  p->pid = pid;
  p->stdin_fd = stdin_pipe[1];
  p->stdout_fd = stdout_pipe[0];
#endif

  return p;
}

bool bebopc_process_write(bebopc_process_t* p, const void* data, size_t len)
{
  if (!p || !data || len == 0) {
    return false;
  }

#ifdef BEBOPC_WINDOWS
  DWORD written;
  return WriteFile(p->stdin_write, data, (DWORD)len, &written, NULL) && written == len;
#else
  const uint8_t* ptr = data;
  size_t remaining = len;
  while (remaining > 0) {
    ssize_t n = write(p->stdin_fd, ptr, remaining);
    if (n <= 0) {
      return false;
    }
    ptr += n;
    remaining -= (size_t)n;
  }
  return true;
#endif
}

void bebopc_process_close_stdin(bebopc_process_t* p)
{
  if (!p) {
    return;
  }
#ifdef BEBOPC_WINDOWS
  if (p->stdin_write) {
    CloseHandle(p->stdin_write);
    p->stdin_write = NULL;
  }
#else
  if (p->stdin_fd >= 0) {
    close(p->stdin_fd);
    p->stdin_fd = -1;
  }
#endif
}

uint8_t* bebopc_process_read_all(bebopc_process_t* p, size_t* out_len)
{
  if (!p || !out_len) {
    return NULL;
  }

  size_t cap = 4096;
  size_t len = 0;
  uint8_t* buf = malloc(cap);
  if (!buf) {
    return NULL;
  }

  while (1) {
    if (len + 4096 > cap) {
      cap *= 2;
      uint8_t* new_buf = realloc(buf, cap);
      if (!new_buf) {
        free(buf);
        return NULL;
      }
      buf = new_buf;
    }

#ifdef BEBOPC_WINDOWS
    DWORD n = 0;
    if (!ReadFile(p->stdout_read, buf + len, (DWORD)(cap - len), &n, NULL) || n == 0) {
      break;
    }
    len += n;
#else
    ssize_t n = read(p->stdout_fd, buf + len, cap - len);
    if (n <= 0) {
      break;
    }
    len += (size_t)n;
#endif
  }

  *out_len = len;
  return buf;
}

int bebopc_process_wait(bebopc_process_t* p)
{
  if (!p) {
    return -1;
  }

#ifdef BEBOPC_WINDOWS
  WaitForSingleObject(p->process, INFINITE);
  DWORD exit_code = 0;
  GetExitCodeProcess(p->process, &exit_code);
  return (int)exit_code;
#else
  int status;
  waitpid(p->pid, &status, 0);
  if (WIFEXITED(status)) {
    return WEXITSTATUS(status);
  }
  if (WIFSIGNALED(status)) {
    return -WTERMSIG(status);
  }
  return -1;
#endif
}

void bebopc_process_free(bebopc_process_t* p)
{
  if (!p) {
    return;
  }

#ifdef BEBOPC_WINDOWS
  if (p->stdin_write) {
    CloseHandle(p->stdin_write);
  }
  if (p->stdout_read) {
    CloseHandle(p->stdout_read);
  }
  if (p->process) {
    CloseHandle(p->process);
  }
#else
  if (p->stdin_fd >= 0) {
    close(p->stdin_fd);
  }
  if (p->stdout_fd >= 0) {
    close(p->stdout_fd);
  }
#endif

  free(p);
}
