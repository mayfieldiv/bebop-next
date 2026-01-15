typedef struct {
  const char* source;
  size_t source_len;
  size_t pos;
  uint32_t line;
  uint32_t col;
  bebop_context_t* ctx;
  bebop_schema_t* schema;

  bebop_trivia_t* trivia_buf;
  uint32_t trivia_count;
  uint32_t trivia_capacity;

  bebop_error_t error;
} _bebop_scanner_t;

typedef struct {
  const char* keyword;
  bebop_token_kind_t kind;
} _bebop_keyword_t;

static const _bebop_keyword_t _bebop_keywords[] = {
#define X(name, str) {str, BEBOP_TOKEN_##name},
    BEBOP_KEYWORDS(X)
#undef X
};

#define BEBOP_KEYWORD_COUNT BEBOP_COUNTOF(_bebop_keywords)

static bebop_token_kind_t _bebop_scan_lookup_keyword(const char* str, const size_t len)
{
  for (size_t i = 0; i < BEBOP_KEYWORD_COUNT; i++) {
    const char* kw = _bebop_keywords[i].keyword;
    if (bebop_streqn(kw, str, len)) {
      return _bebop_keywords[i].kind;
    }
  }
  return BEBOP_TOKEN_IDENTIFIER;
}

static inline char _bebop_scan_peek_char(const _bebop_scanner_t* s)
{
  if (s->pos >= s->source_len) {
    return '\0';
  }
  return s->source[s->pos];
}

static inline char _bebop_scan_peek_char_at(const _bebop_scanner_t* s, const size_t offset)
{
  if (s->pos + offset >= s->source_len) {
    return '\0';
  }
  return s->source[s->pos + offset];
}

static inline void _bebop_scan_advance(_bebop_scanner_t* s)
{
  if (s->pos >= s->source_len) {
    return;
  }

  const char c = s->source[s->pos];
  if (c == '\n') {
    s->line++;
    s->col = 1;
    s->pos++;
  } else if (c == '\r') {
    s->line++;
    s->col = 1;
    s->pos++;
    if (s->pos < s->source_len && s->source[s->pos] == '\n') {
      s->pos++;
    }
  } else {
    s->col++;
    s->pos++;
  }
}

static inline bebop_span_t _bebop_scan_make_span(
    const _bebop_scanner_t* s,
    const size_t start_pos,
    const uint32_t start_line,
    const uint32_t start_col
)
{
  return (bebop_span_t) {
      .off = (uint32_t)start_pos,
      .len = (uint32_t)(s->pos - start_pos),
      .start_line = start_line,
      .start_col = start_col,
      .end_line = s->line,
      .end_col = s->col,
  };
}

#define BEBOP_TRIVIA_INITIAL_CAPACITY 8

static void _bebop_scan_trivia_push(
    _bebop_scanner_t* s, const bebop_trivia_kind_t kind, const bebop_span_t span
)
{
  if (s->error != BEBOP_ERR_NONE) {
    return;
  }

  if (s->trivia_count >= s->trivia_capacity) {
    const uint32_t new_cap =
        s->trivia_capacity == 0 ? BEBOP_TRIVIA_INITIAL_CAPACITY : s->trivia_capacity * 2;
    bebop_trivia_t* new_buf = bebop_arena_new(BEBOP_ARENA(s->ctx), bebop_trivia_t, new_cap);
    if (!new_buf) {
      s->error = BEBOP_ERR_OUT_OF_MEMORY;
      _bebop_context_set_error(s->ctx, BEBOP_ERR_OUT_OF_MEMORY, "Failed to allocate trivia buffer");
      return;
    }
    if (s->trivia_buf && s->trivia_count > 0) {
      memcpy(new_buf, s->trivia_buf, s->trivia_count * sizeof(bebop_trivia_t));
    }
    s->trivia_buf = new_buf;
    s->trivia_capacity = new_cap;
  }
  s->trivia_buf[s->trivia_count++] = (bebop_trivia_t) {.kind = kind, .span = span};
}

static bebop_trivia_list_t _bebop_scan_trivia_finalize(_bebop_scanner_t* s)
{
  bebop_trivia_list_t list = {0};
  if (s->error != BEBOP_ERR_NONE) {
    return list;
  }

  if (s->trivia_count > 0) {
    list.items = bebop_arena_new(BEBOP_ARENA(s->ctx), bebop_trivia_t, s->trivia_count);
    if (!list.items) {
      s->error = BEBOP_ERR_OUT_OF_MEMORY;
      _bebop_context_set_error(s->ctx, BEBOP_ERR_OUT_OF_MEMORY, "Failed to allocate trivia list");
      s->trivia_count = 0;
      return list;
    }
    memcpy(list.items, s->trivia_buf, s->trivia_count * sizeof(bebop_trivia_t));
    list.count = s->trivia_count;
    s->trivia_count = 0;
  }
  return list;
}

static void _bebop_scan_skip_whitespace(_bebop_scanner_t* s)
{
  const size_t start = s->pos;
  const uint32_t start_line = s->line;
  const uint32_t start_col = s->col;

  while (BEBOP_IS_WHITESPACE(_bebop_scan_peek_char(s))) {
    _bebop_scan_advance(s);
  }

  if (s->pos > start) {
    _bebop_scan_trivia_push(
        s, BEBOP_TRIVIA_WHITESPACE, _bebop_scan_make_span(s, start, start_line, start_col)
    );
  }
}

static bool _bebop_scan_skip_newline(_bebop_scanner_t* s)
{
  if (!BEBOP_IS_NEWLINE(_bebop_scan_peek_char(s))) {
    return false;
  }

  const size_t start = s->pos;
  const uint32_t start_line = s->line;
  const uint32_t start_col = s->col;

  _bebop_scan_advance(s);

  _bebop_scan_trivia_push(
      s, BEBOP_TRIVIA_NEWLINE, _bebop_scan_make_span(s, start, start_line, start_col)
  );
  return true;
}

static bool _bebop_scan_skip_line_comment(_bebop_scanner_t* s)
{
  if (_bebop_scan_peek_char(s) != '/' || _bebop_scan_peek_char_at(s, 1) != '/') {
    return false;
  }

  const size_t start = s->pos;
  const uint32_t start_line = s->line;
  const uint32_t start_col = s->col;

  const bool is_doc = _bebop_scan_peek_char_at(s, 2) == '/';

  _bebop_scan_advance(s);
  _bebop_scan_advance(s);

  while (_bebop_scan_peek_char(s) != '\0' && !BEBOP_IS_NEWLINE(_bebop_scan_peek_char(s))) {
    _bebop_scan_advance(s);
  }

  const bebop_trivia_kind_t kind = is_doc ? BEBOP_TRIVIA_DOC_COMMENT : BEBOP_TRIVIA_LINE_COMMENT;
  _bebop_scan_trivia_push(s, kind, _bebop_scan_make_span(s, start, start_line, start_col));
  return true;
}

static bool _bebop_scan_skip_block_comment(_bebop_scanner_t* s)
{
  if (_bebop_scan_peek_char(s) != '/' || _bebop_scan_peek_char_at(s, 1) != '*') {
    return false;
  }

  const size_t start = s->pos;
  const uint32_t start_line = s->line;
  const uint32_t start_col = s->col;

  const bool is_doc =
      _bebop_scan_peek_char_at(s, 2) == '*' && _bebop_scan_peek_char_at(s, 3) != '/';

  _bebop_scan_advance(s);
  _bebop_scan_advance(s);

  while (_bebop_scan_peek_char(s) != '\0') {
    if (_bebop_scan_peek_char(s) == '*' && _bebop_scan_peek_char_at(s, 1) == '/') {
      _bebop_scan_advance(s);
      _bebop_scan_advance(s);
      goto done;
    }
    _bebop_scan_advance(s);
  }

  if (s->schema) {
    _bebop_schema_add_diagnostic(
        s->schema,
        (_bebop_diag_loc_t) {BEBOP_DIAG_ERROR,
                             BEBOP_DIAG_UNTERMINATED_COMMENT,
                             _bebop_scan_make_span(s, start, start_line, start_col)},
        "Unterminated block comment",
        NULL
    );
  }

done:;
  const bebop_trivia_kind_t kind = is_doc ? BEBOP_TRIVIA_DOC_COMMENT : BEBOP_TRIVIA_BLOCK_COMMENT;
  _bebop_scan_trivia_push(s, kind, _bebop_scan_make_span(s, start, start_line, start_col));
  return true;
}

static void _bebop_scan_skip_leading_trivia(_bebop_scanner_t* s)
{
  for (;;) {
    _bebop_scan_skip_whitespace(s);

    if (_bebop_scan_skip_newline(s)) {
      continue;
    }
    if (_bebop_scan_skip_line_comment(s)) {
      continue;
    }
    if (_bebop_scan_skip_block_comment(s)) {
      continue;
    }

    break;
  }
}

static bebop_trivia_list_t _bebop_scan_collect_trailing_trivia(_bebop_scanner_t* s)
{
  s->trivia_count = 0;

  for (;;) {
    _bebop_scan_skip_whitespace(s);

    if (_bebop_scan_skip_line_comment(s)) {
      continue;
    }
    if (_bebop_scan_skip_block_comment(s)) {
      continue;
    }

    if (_bebop_scan_skip_newline(s)) {
      break;
    }

    break;
  }

  return _bebop_scan_trivia_finalize(s);
}

static bebop_token_t _bebop_scan_identifier(_bebop_scanner_t* s)
{
  const size_t start = s->pos;
  const uint32_t start_line = s->line;
  const uint32_t start_col = s->col;

  while (BEBOP_IS_IDENT_CHAR(_bebop_scan_peek_char(s))) {
    _bebop_scan_advance(s);
  }

  const size_t len = s->pos - start;
  const bebop_token_kind_t kind = _bebop_scan_lookup_keyword(s->source + start, len);

  const bebop_token_t tok = {
      .kind = kind,
      .span = _bebop_scan_make_span(s, start, start_line, start_col),
      .lexeme = bebop_intern_n(BEBOP_INTERN(s->ctx), s->source + start, len),
  };
  return tok;
}

static bebop_token_t _bebop_scan_number(_bebop_scanner_t* s)
{
  const size_t start = s->pos;
  const uint32_t start_line = s->line;
  const uint32_t start_col = s->col;

  while (BEBOP_IS_IDENT_CHAR(_bebop_scan_peek_char(s)) || _bebop_scan_peek_char(s) == '.'
         || _bebop_scan_peek_char(s) == '-')
  {
    if (_bebop_scan_peek_char(s) == '-') {
      const char prev = s->pos > 0 ? s->source[s->pos - 1] : '\0';
      if (prev != 'e' && prev != 'E') {
        break;
      }
    }
    _bebop_scan_advance(s);
  }

  const size_t len = s->pos - start;

  const bebop_token_t tok = {
      .kind = BEBOP_TOKEN_NUMBER,
      .span = _bebop_scan_make_span(s, start, start_line, start_col),
      .lexeme = bebop_intern_n(BEBOP_INTERN(s->ctx), s->source + start, len),
  };
  return tok;
}

static bool _bebop_scan_grow_buf(_bebop_scanner_t* s, char** buf, size_t* buf_cap, size_t buf_len)
{
  const size_t new_cap = *buf_cap == 0 ? 64 : *buf_cap * 2;
  char* new_buf = bebop_arena_new(BEBOP_ARENA(s->ctx), char, new_cap);
  if (!new_buf) {
    s->error = BEBOP_ERR_OUT_OF_MEMORY;
    _bebop_context_set_error(s->ctx, BEBOP_ERR_OUT_OF_MEMORY, "Failed to allocate string buffer");
    return false;
  }
  if (*buf && buf_len > 0) {
    memcpy(new_buf, *buf, buf_len);
  }
  *buf = new_buf;
  *buf_cap = new_cap;
  return true;
}

static bebop_token_t _bebop_scan_bytes(_bebop_scanner_t* s)
{
  const size_t start = s->pos;
  const uint32_t start_line = s->line;
  const uint32_t start_col = s->col;

  _bebop_scan_advance(s);

  const char quote = _bebop_scan_peek_char(s);
  if (quote != '"' && quote != '\'') {
    return (bebop_token_t) {
        .kind = BEBOP_TOKEN_ERROR,
        .span = _bebop_scan_make_span(s, start, start_line, start_col),
    };
  }
  _bebop_scan_advance(s);

  char* buf = NULL;
  size_t buf_len = 0;
  size_t buf_cap = 0;

  while (_bebop_scan_peek_char(s) != '\0' && s->error == BEBOP_ERR_NONE) {
    const char c = _bebop_scan_peek_char(s);

    if (c == quote) {
      if (_bebop_scan_peek_char_at(s, 1) == quote) {
        _bebop_scan_advance(s);
        _bebop_scan_advance(s);
        if (buf_len >= buf_cap && !_bebop_scan_grow_buf(s, &buf, &buf_cap, buf_len)) {
          break;
        }
        buf[buf_len++] = quote;
      } else {
        _bebop_scan_advance(s);
        goto done_bytes;
      }
    } else if (c == '\\') {
      const size_t esc_start = s->pos;
      const uint32_t esc_line = s->line;
      const uint32_t esc_col = s->col;
      _bebop_scan_advance(s);

      const size_t remaining = s->source_len - s->pos;
      char esc_out[4];
      int esc_out_len = 0;
      const int consumed =
          bebop_unescape_char(s->source + s->pos, remaining, esc_out, &esc_out_len);

      if (consumed == 0) {
        if (s->schema) {
          _bebop_schema_add_diagnostic(
              s->schema,
              (_bebop_diag_loc_t) {BEBOP_DIAG_ERROR,
                                   BEBOP_DIAG_INVALID_ESCAPE,
                                   _bebop_scan_make_span(s, esc_start, esc_line, esc_col)},
              "Invalid escape sequence",
              NULL
          );
        }
        goto skip_bytes_to_end;
      }

      while (buf_len + (size_t)esc_out_len > buf_cap) {
        if (!_bebop_scan_grow_buf(s, &buf, &buf_cap, buf_len)) {
          break;
        }
      }
      if (s->error != BEBOP_ERR_NONE) {
        break;
      }
      for (int i = 0; i < esc_out_len; i++) {
        buf[buf_len++] = esc_out[i];
      }
      for (int i = 0; i < consumed; i++) {
        _bebop_scan_advance(s);
      }
    } else if (c == '\r' && _bebop_scan_peek_char_at(s, 1) == '\n') {
      if (buf_len >= buf_cap && !_bebop_scan_grow_buf(s, &buf, &buf_cap, buf_len)) {
        break;
      }
      buf[buf_len++] = '\n';
      _bebop_scan_advance(s);
      _bebop_scan_advance(s);
      s->line++;
      s->col = 1;
    } else if (BEBOP_IS_NEWLINE(c)) {
      if (buf_len >= buf_cap && !_bebop_scan_grow_buf(s, &buf, &buf_cap, buf_len)) {
        break;
      }
      buf[buf_len++] = '\n';
      _bebop_scan_advance(s);
      s->line++;
      s->col = 1;
    } else {
      if (buf_len >= buf_cap && !_bebop_scan_grow_buf(s, &buf, &buf_cap, buf_len)) {
        break;
      }
      buf[buf_len++] = c;
      _bebop_scan_advance(s);
    }
  }

done_bytes:
  if (s->error != BEBOP_ERR_NONE) {
    return (bebop_token_t) {
        .kind = BEBOP_TOKEN_ERROR,
        .span = _bebop_scan_make_span(s, start, start_line, start_col),
    };
  }

  if (buf_len >= buf_cap) {
    const size_t final_cap = buf_len + 1;
    char* new_buf = bebop_arena_new(BEBOP_ARENA(s->ctx), char, final_cap);
    if (!new_buf) {
      s->error = BEBOP_ERR_OUT_OF_MEMORY;
      _bebop_context_set_error(s->ctx, BEBOP_ERR_OUT_OF_MEMORY, "Failed to allocate bytes buffer");
      return (bebop_token_t) {
          .kind = BEBOP_TOKEN_ERROR,
          .span = _bebop_scan_make_span(s, start, start_line, start_col),
      };
    }
    if (buf && buf_len > 0) {
      memcpy(new_buf, buf, buf_len);
    }
    buf = new_buf;
  }
  if (buf) {
    buf[buf_len] = '\0';
  }

  return (bebop_token_t) {
      .kind = BEBOP_TOKEN_BYTES,
      .span = _bebop_scan_make_span(s, start, start_line, start_col),
      .lexeme = bebop_intern_n(BEBOP_INTERN(s->ctx), buf ? buf : "", buf_len),
  };

skip_bytes_to_end:
  while (_bebop_scan_peek_char(s) != '\0') {
    const char c = _bebop_scan_peek_char(s);
    if (c == quote && _bebop_scan_peek_char_at(s, 1) != quote) {
      _bebop_scan_advance(s);
      break;
    }
    _bebop_scan_advance(s);
  }
  return (bebop_token_t) {
      .kind = BEBOP_TOKEN_ERROR,
      .span = _bebop_scan_make_span(s, start, start_line, start_col),
  };
}

static bebop_token_t _bebop_scan_string(_bebop_scanner_t* s)
{
  const size_t start = s->pos;
  const uint32_t start_line = s->line;
  const uint32_t start_col = s->col;

  const char quote = _bebop_scan_peek_char(s);
  _bebop_scan_advance(s);

  char* buf = NULL;
  size_t buf_len = 0;
  size_t buf_cap = 0;

  while (_bebop_scan_peek_char(s) != '\0' && s->error == BEBOP_ERR_NONE) {
    const char c = _bebop_scan_peek_char(s);

    if (c == quote) {
      if (_bebop_scan_peek_char_at(s, 1) == quote) {
        _bebop_scan_advance(s);
        _bebop_scan_advance(s);
        if (buf_len >= buf_cap && !_bebop_scan_grow_buf(s, &buf, &buf_cap, buf_len)) {
          break;
        }
        buf[buf_len++] = quote;
      } else {
        _bebop_scan_advance(s);
        goto done;
      }
    } else if (c == '\\') {
      const size_t esc_start = s->pos;
      const uint32_t esc_line = s->line;
      const uint32_t esc_col = s->col;
      _bebop_scan_advance(s);

      const size_t remaining = s->source_len - s->pos;
      char esc_out[4];
      int esc_out_len = 0;
      const int consumed =
          bebop_unescape_char(s->source + s->pos, remaining, esc_out, &esc_out_len);

      if (consumed == 0) {
        if (s->schema) {
          _bebop_schema_add_diagnostic(
              s->schema,
              (_bebop_diag_loc_t) {BEBOP_DIAG_ERROR,
                                   BEBOP_DIAG_INVALID_ESCAPE,
                                   _bebop_scan_make_span(s, esc_start, esc_line, esc_col)},
              "Invalid escape sequence",
              NULL
          );
        }
        goto skip_to_end;
      }

      while (buf_len + (size_t)esc_out_len > buf_cap) {
        if (!_bebop_scan_grow_buf(s, &buf, &buf_cap, buf_len)) {
          break;
        }
      }
      if (s->error != BEBOP_ERR_NONE) {
        break;
      }
      for (int i = 0; i < esc_out_len; i++) {
        buf[buf_len++] = esc_out[i];
      }
      for (int i = 0; i < consumed; i++) {
        _bebop_scan_advance(s);
      }
    } else if (c == '\r' && _bebop_scan_peek_char_at(s, 1) == '\n') {
      if (buf_len >= buf_cap && !_bebop_scan_grow_buf(s, &buf, &buf_cap, buf_len)) {
        break;
      }
      buf[buf_len++] = '\n';
      _bebop_scan_advance(s);
      _bebop_scan_advance(s);
      s->line++;
      s->col = 1;
    } else if (BEBOP_IS_NEWLINE(c)) {
      if (buf_len >= buf_cap && !_bebop_scan_grow_buf(s, &buf, &buf_cap, buf_len)) {
        break;
      }
      buf[buf_len++] = '\n';
      _bebop_scan_advance(s);
      s->line++;
      s->col = 1;
    } else {
      if (buf_len >= buf_cap && !_bebop_scan_grow_buf(s, &buf, &buf_cap, buf_len)) {
        break;
      }
      buf[buf_len++] = c;
      _bebop_scan_advance(s);
    }
  }

done:
  if (s->error != BEBOP_ERR_NONE) {
    return (bebop_token_t) {
        .kind = BEBOP_TOKEN_ERROR,
        .span = _bebop_scan_make_span(s, start, start_line, start_col),
    };
  }

  if (buf_len >= buf_cap) {
    const size_t final_cap = buf_len + 1;
    char* new_buf = bebop_arena_new(BEBOP_ARENA(s->ctx), char, final_cap);
    if (!new_buf) {
      s->error = BEBOP_ERR_OUT_OF_MEMORY;
      _bebop_context_set_error(s->ctx, BEBOP_ERR_OUT_OF_MEMORY, "Failed to allocate string buffer");
      return (bebop_token_t) {
          .kind = BEBOP_TOKEN_ERROR,
          .span = _bebop_scan_make_span(s, start, start_line, start_col),
      };
    }
    if (buf && buf_len > 0) {
      memcpy(new_buf, buf, buf_len);
    }
    buf = new_buf;
  }
  if (buf) {
    buf[buf_len] = '\0';
  }

  if (buf_len > 0 && !bebop_utf8_valid(buf, buf_len)) {
    if (s->schema) {
      _bebop_schema_add_diagnostic(
          s->schema,
          (_bebop_diag_loc_t) {BEBOP_DIAG_ERROR,
                               BEBOP_DIAG_INVALID_UTF8,
                               _bebop_scan_make_span(s, start, start_line, start_col)},
          "Invalid UTF-8 encoding in string literal",
          NULL
      );
    }
    return (bebop_token_t) {
        .kind = BEBOP_TOKEN_ERROR,
        .span = _bebop_scan_make_span(s, start, start_line, start_col),
    };
  }

  return (bebop_token_t) {
      .kind = BEBOP_TOKEN_STRING,
      .span = _bebop_scan_make_span(s, start, start_line, start_col),
      .lexeme = bebop_intern_n(BEBOP_INTERN(s->ctx), buf ? buf : "", buf_len),
  };

skip_to_end:
  while (_bebop_scan_peek_char(s) != '\0') {
    const char c = _bebop_scan_peek_char(s);
    if (c == quote && _bebop_scan_peek_char_at(s, 1) != quote) {
      _bebop_scan_advance(s);
      break;
    }
    _bebop_scan_advance(s);
  }
  return (bebop_token_t) {
      .kind = BEBOP_TOKEN_ERROR,
      .span = _bebop_scan_make_span(s, start, start_line, start_col),
  };
}

static bebop_token_t _bebop_scan_token(_bebop_scanner_t* s)
{
  const size_t start = s->pos;
  const uint32_t start_line = s->line;
  const uint32_t start_col = s->col;

  const char c = _bebop_scan_peek_char(s);

  if (c == '\0') {
    return (bebop_token_t) {
        .kind = BEBOP_TOKEN_EOF,
        .span = _bebop_scan_make_span(s, start, start_line, start_col),
    };
  }

  if (BEBOP_IS_IDENT_START(c)) {
    if (c == 'b') {
      const char next = _bebop_scan_peek_char_at(s, 1);
      if (next == '"' || next == '\'') {
        return _bebop_scan_bytes(s);
      }
    }
    return _bebop_scan_identifier(s);
  }

  if (BEBOP_IS_DIGIT(c)) {
    return _bebop_scan_number(s);
  }

  if (c == '"' || c == '\'') {
    return _bebop_scan_string(s);
  }

  _bebop_scan_advance(s);

  switch (c) {
    case '(':
      return (bebop_token_t) {.kind = BEBOP_TOKEN_LPAREN,
                              .span = _bebop_scan_make_span(s, start, start_line, start_col)};
    case ')':
      return (bebop_token_t) {.kind = BEBOP_TOKEN_RPAREN,
                              .span = _bebop_scan_make_span(s, start, start_line, start_col)};
    case '{':
      return (bebop_token_t) {.kind = BEBOP_TOKEN_LBRACE,
                              .span = _bebop_scan_make_span(s, start, start_line, start_col)};
    case '}':
      return (bebop_token_t) {.kind = BEBOP_TOKEN_RBRACE,
                              .span = _bebop_scan_make_span(s, start, start_line, start_col)};
    case '[':
      if (_bebop_scan_peek_char(s) == '[') {
        _bebop_scan_advance(s);
        const size_t content_start = s->pos;
        while (_bebop_scan_peek_char(s) != '\0') {
          if (_bebop_scan_peek_char(s) == ']' && _bebop_scan_peek_char_at(s, 1) == ']') {
            const size_t content_len = s->pos - content_start;
            _bebop_scan_advance(s);
            _bebop_scan_advance(s);
            return (bebop_token_t) {
                .kind = BEBOP_TOKEN_RAW_BLOCK,
                .span = _bebop_scan_make_span(s, start, start_line, start_col),
                .lexeme =
                    bebop_intern_n(BEBOP_INTERN(s->ctx), s->source + content_start, content_len),
            };
          }
          _bebop_scan_advance(s);
        }

        if (s->schema) {
          _bebop_schema_add_diagnostic(
              s->schema,
              (_bebop_diag_loc_t) {BEBOP_DIAG_ERROR,
                                   BEBOP_DIAG_INVALID_MACRO,
                                   _bebop_scan_make_span(s, start, start_line, start_col)},
              "Unterminated raw block: expected ']]'",
              NULL
          );
        }
        return (bebop_token_t) {
            .kind = BEBOP_TOKEN_ERROR,
            .span = _bebop_scan_make_span(s, start, start_line, start_col),
        };
      }
      return (bebop_token_t) {.kind = BEBOP_TOKEN_LBRACKET,
                              .span = _bebop_scan_make_span(s, start, start_line, start_col)};
    case ']':
      return (bebop_token_t) {.kind = BEBOP_TOKEN_RBRACKET,
                              .span = _bebop_scan_make_span(s, start, start_line, start_col)};
    case '<':
      return (bebop_token_t) {.kind = BEBOP_TOKEN_LANGLE,
                              .span = _bebop_scan_make_span(s, start, start_line, start_col)};
    case '>':
      return (bebop_token_t) {.kind = BEBOP_TOKEN_RANGLE,
                              .span = _bebop_scan_make_span(s, start, start_line, start_col)};
    case ':':
      return (bebop_token_t) {.kind = BEBOP_TOKEN_COLON,
                              .span = _bebop_scan_make_span(s, start, start_line, start_col)};
    case ';':
      return (bebop_token_t) {.kind = BEBOP_TOKEN_SEMICOLON,
                              .span = _bebop_scan_make_span(s, start, start_line, start_col)};
    case ',':
      return (bebop_token_t) {.kind = BEBOP_TOKEN_COMMA,
                              .span = _bebop_scan_make_span(s, start, start_line, start_col)};
    case '.':
      return (bebop_token_t) {.kind = BEBOP_TOKEN_DOT,
                              .span = _bebop_scan_make_span(s, start, start_line, start_col)};
    case '?':
      return (bebop_token_t) {.kind = BEBOP_TOKEN_QUESTION,
                              .span = _bebop_scan_make_span(s, start, start_line, start_col)};
    case '/':
      return (bebop_token_t) {.kind = BEBOP_TOKEN_SLASH,
                              .span = _bebop_scan_make_span(s, start, start_line, start_col)};
    case '=':
      return (bebop_token_t) {.kind = BEBOP_TOKEN_EQUALS,
                              .span = _bebop_scan_make_span(s, start, start_line, start_col)};
    case '@':
      return (bebop_token_t) {.kind = BEBOP_TOKEN_AT,
                              .span = _bebop_scan_make_span(s, start, start_line, start_col)};
    case '#':
      return (bebop_token_t) {.kind = BEBOP_TOKEN_HASH,
                              .span = _bebop_scan_make_span(s, start, start_line, start_col)};
    case '!':
      return (bebop_token_t) {.kind = BEBOP_TOKEN_BANG,
                              .span = _bebop_scan_make_span(s, start, start_line, start_col)};
    case '$':
      return (bebop_token_t) {.kind = BEBOP_TOKEN_DOLLAR,
                              .span = _bebop_scan_make_span(s, start, start_line, start_col)};
    case '\\':
      return (bebop_token_t) {.kind = BEBOP_TOKEN_BACKSLASH,
                              .span = _bebop_scan_make_span(s, start, start_line, start_col)};
    case '`':
      return (bebop_token_t) {.kind = BEBOP_TOKEN_BACKTICK,
                              .span = _bebop_scan_make_span(s, start, start_line, start_col)};
    case '~':
      return (bebop_token_t) {.kind = BEBOP_TOKEN_TILDE,
                              .span = _bebop_scan_make_span(s, start, start_line, start_col)};
    case '&':
      return (bebop_token_t) {.kind = BEBOP_TOKEN_AMPERSAND,
                              .span = _bebop_scan_make_span(s, start, start_line, start_col)};
    case '|':
      return (bebop_token_t) {.kind = BEBOP_TOKEN_PIPE,
                              .span = _bebop_scan_make_span(s, start, start_line, start_col)};

    case '-':
      if (_bebop_scan_peek_char(s) == '>') {
        _bebop_scan_advance(s);
        return (bebop_token_t) {.kind = BEBOP_TOKEN_ARROW,
                                .span = _bebop_scan_make_span(s, start, start_line, start_col)};
      }
      return (bebop_token_t) {.kind = BEBOP_TOKEN_MINUS,
                              .span = _bebop_scan_make_span(s, start, start_line, start_col)};

    default:
      return (bebop_token_t) {
          .kind = BEBOP_TOKEN_ERROR,
          .span = _bebop_scan_make_span(s, start, start_line, start_col),
          .lexeme = bebop_intern_n(BEBOP_INTERN(s->ctx), s->source + start, 1),
      };
  }
}

static bebop_token_t _bebop_scan_next(_bebop_scanner_t* s)
{
  if (s->error != BEBOP_ERR_NONE) {
    return (bebop_token_t) {.kind = BEBOP_TOKEN_EOF};
  }

  _bebop_scan_skip_leading_trivia(s);
  if (s->error != BEBOP_ERR_NONE) {
    return (bebop_token_t) {.kind = BEBOP_TOKEN_EOF};
  }
  const bebop_trivia_list_t leading = _bebop_scan_trivia_finalize(s);

  bebop_token_t tok = _bebop_scan_token(s);
  tok.leading = leading;

  if (tok.kind != BEBOP_TOKEN_EOF && s->error == BEBOP_ERR_NONE) {
    tok.trailing = _bebop_scan_collect_trailing_trivia(s);
  }

  return tok;
}

bebop_token_stream_t _bebop_scan_with_schema(
    bebop_context_t* ctx, const char* source, const size_t len, bebop_schema_t* schema
)
{
  BEBOP_ASSERT(ctx != NULL);

  _bebop_scanner_t s = {
      .source = source,
      .source_len = len,
      .pos = 0,
      .line = 1,
      .col = 1,
      .ctx = ctx,
      .schema = schema,
      .error = BEBOP_ERR_NONE,
  };

  bebop_token_t* tokens = NULL;
  uint32_t count = 0;
  uint32_t capacity = 0;

  while (s.error == BEBOP_ERR_NONE) {
    const bebop_token_t tok = _bebop_scan_next(&s);

    if (count >= capacity) {
      const uint32_t new_cap = capacity == 0 ? 64 : capacity * 2;
      bebop_token_t* new_buf = bebop_arena_new(BEBOP_ARENA(ctx), bebop_token_t, new_cap);
      if (!new_buf) {
        s.error = BEBOP_ERR_OUT_OF_MEMORY;
        _bebop_context_set_error(ctx, BEBOP_ERR_OUT_OF_MEMORY, "Failed to allocate token buffer");
        break;
      }
      if (tokens && count > 0) {
        memcpy(new_buf, tokens, count * sizeof(bebop_token_t));
      }
      tokens = new_buf;
      capacity = new_cap;
    }

    tokens[count++] = tok;

    if (tok.kind == BEBOP_TOKEN_EOF) {
      break;
    }
  }

  return (bebop_token_stream_t) {
      .tokens = tokens,
      .count = count,
  };
}

bebop_token_stream_t bebop_scan(bebop_context_t* ctx, const char* source, const size_t len)
{
  return _bebop_scan_with_schema(ctx, source, len, NULL);
}

const char* bebop_token_kind_name(const bebop_token_kind_t kind)
{
  switch (kind) {
    case BEBOP_TOKEN_ENUM:
      return "enum";
    case BEBOP_TOKEN_STRUCT:
      return "struct";
    case BEBOP_TOKEN_MESSAGE:
      return "message";
    case BEBOP_TOKEN_MUT:
      return "mut";
    case BEBOP_TOKEN_READONLY:
      return "readonly";
    case BEBOP_TOKEN_MAP:
      return "map";
    case BEBOP_TOKEN_ARRAY:
      return "array";
    case BEBOP_TOKEN_UNION:
      return "union";
    case BEBOP_TOKEN_SERVICE:
      return "service";
    case BEBOP_TOKEN_STREAM:
      return "stream";
    case BEBOP_TOKEN_IMPORT:
      return "import";
    case BEBOP_TOKEN_EDITION:
      return "edition";
    case BEBOP_TOKEN_PACKAGE:
      return "package";
    case BEBOP_TOKEN_EXPORT:
      return "export";
    case BEBOP_TOKEN_LOCAL:
      return "local";
    case BEBOP_TOKEN_TRUE:
      return "true";
    case BEBOP_TOKEN_FALSE:
      return "false";
    case BEBOP_TOKEN_CONST:
      return "const";
    case BEBOP_TOKEN_WITH:
      return "with";
    case BEBOP_TOKEN_IDENTIFIER:
      return "identifier";
    case BEBOP_TOKEN_STRING:
      return "string";
    case BEBOP_TOKEN_BYTES:
      return "bytes";
    case BEBOP_TOKEN_NUMBER:
      return "number";
    case BEBOP_TOKEN_BLOCK_COMMENT:
      return "block_comment";
    case BEBOP_TOKEN_LPAREN:
      return "(";
    case BEBOP_TOKEN_RPAREN:
      return ")";
    case BEBOP_TOKEN_LBRACE:
      return "{";
    case BEBOP_TOKEN_RBRACE:
      return "}";
    case BEBOP_TOKEN_LBRACKET:
      return "[";
    case BEBOP_TOKEN_RBRACKET:
      return "]";
    case BEBOP_TOKEN_LANGLE:
      return "<";
    case BEBOP_TOKEN_RANGLE:
      return ">";
    case BEBOP_TOKEN_COLON:
      return ":";
    case BEBOP_TOKEN_SEMICOLON:
      return ";";
    case BEBOP_TOKEN_COMMA:
      return ",";
    case BEBOP_TOKEN_DOT:
      return ".";
    case BEBOP_TOKEN_QUESTION:
      return "?";
    case BEBOP_TOKEN_SLASH:
      return "/";
    case BEBOP_TOKEN_EQUALS:
      return "=";
    case BEBOP_TOKEN_AT:
      return "@";
    case BEBOP_TOKEN_DOLLAR:
      return "$";
    case BEBOP_TOKEN_BACKSLASH:
      return "\\";
    case BEBOP_TOKEN_BACKTICK:
      return "`";
    case BEBOP_TOKEN_TILDE:
      return "~";
    case BEBOP_TOKEN_AMPERSAND:
      return "&";
    case BEBOP_TOKEN_PIPE:
      return "|";
    case BEBOP_TOKEN_MINUS:
      return "-";
    case BEBOP_TOKEN_ARROW:
      return "->";
    case BEBOP_TOKEN_HASH:
      return "#";
    case BEBOP_TOKEN_BANG:
      return "!";
    case BEBOP_TOKEN_RAW_BLOCK:
      return "raw_block";
    case BEBOP_TOKEN_EOF:
      return "EOF";
    case BEBOP_TOKEN_ERROR:
      return "error";
  }
  return "unknown";
}

const char* bebop_trivia_kind_name(const bebop_trivia_kind_t kind)
{
  switch (kind) {
    case BEBOP_TRIVIA_WHITESPACE:
      return "whitespace";
    case BEBOP_TRIVIA_NEWLINE:
      return "newline";
    case BEBOP_TRIVIA_LINE_COMMENT:
      return "line_comment";
    case BEBOP_TRIVIA_BLOCK_COMMENT:
      return "block_comment";
    case BEBOP_TRIVIA_DOC_COMMENT:
      return "doc_comment";
  }
  return "unknown";
}
