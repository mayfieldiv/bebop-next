typedef struct {
  bebop_context_t* ctx;
  const bebop_schema_t* schema;
  char* data;
  size_t len;
  size_t capacity;
  int indent;
  bool error;
  size_t last_pos;
} _bebop_emit_buf_t;

static void _bebop_emit_init(_bebop_emit_buf_t* buf, const bebop_schema_t* schema)
{
  buf->ctx = schema->ctx;
  buf->schema = schema;
  buf->data = NULL;
  buf->len = 0;
  buf->capacity = 0;
  buf->indent = 0;
  buf->error = false;
  buf->last_pos = 0;
}

static void _bebop_emit_grow(_bebop_emit_buf_t* buf, const size_t needed)
{
  if (buf->error) {
    return;
  }

  size_t new_cap = buf->capacity == 0 ? 1024 : buf->capacity;
  while (new_cap < buf->len + needed + 1) {
    new_cap *= 2;
  }

  if (new_cap > buf->capacity) {
    char* new_data = bebop_arena_realloc(BEBOP_ARENA(buf->ctx), buf->data, buf->capacity, new_cap);
    if (!new_data) {
      buf->error = true;
      return;
    }
    buf->data = new_data;
    buf->capacity = new_cap;
  }
}

static void _bebop_emit_str(_bebop_emit_buf_t* buf, const char* str)
{
  if (!str || buf->error) {
    return;
  }
  size_t slen = strlen(str);
  _bebop_emit_grow(buf, slen);
  if (buf->error) {
    return;
  }
  memcpy(buf->data + buf->len, str, slen);
  buf->len += slen;
}

static void _bebop_emit_str_n(_bebop_emit_buf_t* buf, const char* str, size_t n)
{
  if (!str || n == 0 || buf->error) {
    return;
  }
  _bebop_emit_grow(buf, n);
  if (buf->error) {
    return;
  }
  memcpy(buf->data + buf->len, str, n);
  buf->len += n;
}

static void _bebop_emit_char(_bebop_emit_buf_t* buf, const char c)
{
  _bebop_emit_grow(buf, 1);
  if (buf->error) {
    return;
  }
  buf->data[buf->len++] = c;
}

static void _bebop_emit_u32(_bebop_emit_buf_t* buf, const uint32_t val)
{
  char tmp[16];
  int n = snprintf(tmp, sizeof(tmp), "%" PRIu32, val);
  if (n > 0) {
    _bebop_emit_str(buf, tmp);
  }
}

static void _bebop_emit_i64(_bebop_emit_buf_t* buf, const int64_t val)
{
  char tmp[32];
  int n = snprintf(tmp, sizeof(tmp), "%" PRId64, val);
  if (n > 0) {
    _bebop_emit_str(buf, tmp);
  }
}

static void _bebop_emit_u64(_bebop_emit_buf_t* buf, const uint64_t val)
{
  char tmp[32];
  int n = snprintf(tmp, sizeof(tmp), "%" PRIu64, val);
  if (n > 0) {
    _bebop_emit_str(buf, tmp);
  }
}

static void _bebop_emit_f64(_bebop_emit_buf_t* buf, const double val)
{
  char tmp[64];
  int n = snprintf(tmp, sizeof(tmp), "%g", val);
  if (n > 0) {
    _bebop_emit_str(buf, tmp);
  }
}

static void _bebop_emit_indent(_bebop_emit_buf_t* buf)
{
  for (int i = 0; i < buf->indent; i++) {
    _bebop_emit_str(buf, "    ");
  }
}

static void _bebop_emit_newline(_bebop_emit_buf_t* buf)
{
  _bebop_emit_char(buf, '\n');
}

static void _bebop_emit_trivia_until(_bebop_emit_buf_t* buf, const size_t target_pos)
{
  const bebop_schema_t* schema = buf->schema;
  if (!schema || schema->tokens.count == 0) {
    return;
  }

  for (uint32_t i = 0; i < schema->tokens.count; i++) {
    const bebop_token_t* tok = &schema->tokens.tokens[i];

    for (uint32_t j = 0; j < tok->leading.count; j++) {
      const bebop_trivia_t* t = &tok->leading.items[j];
      if (t->span.off < buf->last_pos) {
        continue;
      }
      if (t->span.off >= target_pos) {
        break;
      }

      if (t->kind == BEBOP_TRIVIA_LINE_COMMENT || t->kind == BEBOP_TRIVIA_BLOCK_COMMENT) {
        _bebop_emit_indent(buf);
        for (size_t k = 0; k < t->span.len; k++) {
          _bebop_emit_char(buf, schema->source[t->span.off + k]);
        }
        _bebop_emit_newline(buf);
      }
    }

    if (tok->span.off >= target_pos) {
      break;
    }
  }

  buf->last_pos = target_pos;
}

static void _bebop_emit_type(_bebop_emit_buf_t* buf, const bebop_type_t* type);
static void _bebop_emit_def(_bebop_emit_buf_t* buf, const bebop_def_t* def);

static void _bebop_emit_literal(_bebop_emit_buf_t* buf, const bebop_literal_t* lit)
{
  if (!lit) {
    return;
  }

  // Use raw_value when available to preserve original format (hex, precision, etc.)
  const char* raw = bebop_str_get(&lit->ctx->intern, lit->raw_value);
  size_t raw_len = bebop_str_len(&lit->ctx->intern, lit->raw_value);

  switch (lit->kind) {
    case BEBOP_LITERAL_BOOL:
      _bebop_emit_str(buf, lit->bool_val ? "true" : "false");
      break;

    case BEBOP_LITERAL_INT:
      if (raw && raw_len > 0) {
        _bebop_emit_str_n(buf, raw, raw_len);
      } else {
        _bebop_emit_i64(buf, lit->int_val);
      }
      break;

    case BEBOP_LITERAL_FLOAT:
      if (raw && raw_len > 0) {
        _bebop_emit_str_n(buf, raw, raw_len);
      } else {
        _bebop_emit_f64(buf, lit->float_val);
      }
      break;

    case BEBOP_LITERAL_STRING:
      // Use source span to preserve original escaping (raw_value has processed content)
      if (buf->schema->source && lit->span.len > 0) {
        _bebop_emit_str_n(buf, buf->schema->source + lit->span.off, lit->span.len);
      } else if (raw && raw_len > 0) {
        _bebop_emit_char(buf, '"');
        _bebop_emit_str_n(buf, raw, raw_len);
        _bebop_emit_char(buf, '"');
      } else {
        _bebop_emit_char(buf, '"');
        const char* s = bebop_str_get(&lit->ctx->intern, lit->string_val);
        const size_t len = bebop_str_len(&lit->ctx->intern, lit->string_val);
        if (s) {
          for (size_t i = 0; i < len; i++) {
            const unsigned char c = (unsigned char)s[i];
            switch (c) {
              case '"':
                _bebop_emit_str(buf, "\\\"");
                break;
              case '\\':
                _bebop_emit_str(buf, "\\\\");
                break;
              case '\n':
                _bebop_emit_str(buf, "\\n");
                break;
              case '\r':
                _bebop_emit_str(buf, "\\r");
                break;
              case '\t':
                _bebop_emit_str(buf, "\\t");
                break;
              case '\0':
                _bebop_emit_str(buf, "\\0");
                break;
              default:
                if (c < 0x20 || c == 0x7F) {
                  char esc[8];
                  snprintf(esc, sizeof(esc), "\\u{%X}", c);
                  _bebop_emit_str(buf, esc);
                } else {
                  _bebop_emit_char(buf, (char)c);
                }
                break;
            }
          }
        }
        _bebop_emit_char(buf, '"');
      }
      break;

    case BEBOP_LITERAL_UUID:
      if (raw && raw_len > 0) {
        _bebop_emit_char(buf, '"');
        _bebop_emit_str_n(buf, raw, raw_len);
        _bebop_emit_char(buf, '"');
      } else {
        // Fallback: emit from binary (note: byte order was swapped on parse)
        const uint8_t* g = lit->uuid_val;
        char tmp[48];
        snprintf(
            tmp,
            sizeof(tmp),
            "\"%02x%02x%02x%02x-%02x%02x-%02x%02x-%02x%02x-%02x%02x%02x%02x%02x%02x\"",
            g[3],
            g[2],
            g[1],
            g[0],
            g[5],
            g[4],
            g[7],
            g[6],
            g[8],
            g[9],
            g[10],
            g[11],
            g[12],
            g[13],
            g[14],
            g[15]
        );
        _bebop_emit_str(buf, tmp);
      }
      break;

    case BEBOP_LITERAL_BYTES:
      if (raw && raw_len > 0) {
        _bebop_emit_str(buf, "b\"");
        _bebop_emit_str_n(buf, raw, raw_len);
        _bebop_emit_char(buf, '"');
      } else {
        _bebop_emit_str(buf, "b\"");
        const uint8_t* data = lit->bytes_val.data;
        const size_t len = lit->bytes_val.len;
        for (size_t i = 0; i < len; i++) {
          const uint8_t c = data[i];
          switch (c) {
            case '"':
              _bebop_emit_str(buf, "\\\"");
              break;
            case '\\':
              _bebop_emit_str(buf, "\\\\");
              break;
            case '\n':
              _bebop_emit_str(buf, "\\n");
              break;
            case '\r':
              _bebop_emit_str(buf, "\\r");
              break;
            case '\t':
              _bebop_emit_str(buf, "\\t");
              break;
            case '\0':
              _bebop_emit_str(buf, "\\0");
              break;
            default:
              if (c >= 0x20 && c < 0x7F) {
                _bebop_emit_char(buf, (char)c);
              } else {
                char esc[8];
                snprintf(esc, sizeof(esc), "\\x%02x", c);
                _bebop_emit_str(buf, esc);
              }
              break;
          }
        }
        _bebop_emit_char(buf, '"');
      }
      break;

    case BEBOP_LITERAL_TIMESTAMP:
      if (raw && raw_len > 0) {
        _bebop_emit_char(buf, '"');
        _bebop_emit_str_n(buf, raw, raw_len);
        _bebop_emit_char(buf, '"');
      } else {
        int64_t secs = lit->timestamp_val.seconds;
        int32_t nanos = lit->timestamp_val.nanos;
        int64_t days = secs / 86400;
        int64_t rem = secs % 86400;
        if (rem < 0) {
          days--;
          rem += 86400;
        }
        days += 719468;
        const int64_t era = (days >= 0 ? days : days - 146096) / 146097;
        const int doe = (int)(days - era * 146097);
        const int yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        const int y = (int)(yoe + era * 400);
        const int doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        const int mp = (5 * doy + 2) / 153;
        const int d = doy - (153 * mp + 2) / 5 + 1;
        const int m = mp + (mp < 10 ? 3 : -9);
        const int year = y + (m <= 2 ? 1 : 0);
        const int hour = (int)(rem / 3600);
        const int minute = (int)((rem % 3600) / 60);
        const int second = (int)(rem % 60);
        char tmp[48];
        if (nanos != 0) {
          int32_t n = nanos;
          int digits = 9;
          while (digits > 1 && (n % 10) == 0) {
            n /= 10;
            digits--;
          }
          snprintf(
              tmp,
              sizeof(tmp),
              "\"%04d-%02d-%02dT%02d:%02d:%02d.%0*dZ\"",
              year,
              m,
              d,
              hour,
              minute,
              second,
              digits,
              n
          );
        } else {
          snprintf(
              tmp, sizeof(tmp), "\"%04d-%02d-%02dT%02d:%02d:%02dZ\"", year, m, d, hour, minute, second
          );
        }
        _bebop_emit_str(buf, tmp);
      }
      break;

    case BEBOP_LITERAL_DURATION:
      if (raw && raw_len > 0) {
        _bebop_emit_char(buf, '"');
        _bebop_emit_str_n(buf, raw, raw_len);
        _bebop_emit_char(buf, '"');
      } else {
        int64_t secs = lit->duration_val.seconds;
        int32_t nanos = lit->duration_val.nanos;
        _bebop_emit_char(buf, '"');
        if (secs < 0 || nanos < 0) {
          _bebop_emit_char(buf, '-');
          secs = secs < 0 ? -secs : secs;
          nanos = nanos < 0 ? -nanos : nanos;
        }
        const int64_t hours = secs / 3600;
        secs %= 3600;
        const int64_t minutes = secs / 60;
        secs %= 60;
        const int32_t ms = nanos / 1000000;
        const int32_t us = (nanos % 1000000) / 1000;
        const int32_t ns = nanos % 1000;
        char tmp[32];
        if (hours > 0) {
          snprintf(tmp, sizeof(tmp), "%" PRId64 "h", hours);
          _bebop_emit_str(buf, tmp);
        }
        if (minutes > 0) {
          snprintf(tmp, sizeof(tmp), "%" PRId64 "m", minutes);
          _bebop_emit_str(buf, tmp);
        }
        if (secs > 0 || (hours == 0 && minutes == 0 && nanos == 0)) {
          snprintf(tmp, sizeof(tmp), "%" PRId64 "s", secs);
          _bebop_emit_str(buf, tmp);
        }
        if (ms > 0) {
          snprintf(tmp, sizeof(tmp), "%dms", ms);
          _bebop_emit_str(buf, tmp);
        }
        if (us > 0) {
          snprintf(tmp, sizeof(tmp), "%dus", us);
          _bebop_emit_str(buf, tmp);
        }
        if (ns > 0) {
          snprintf(tmp, sizeof(tmp), "%dns", ns);
          _bebop_emit_str(buf, tmp);
        }
        _bebop_emit_char(buf, '"');
      }
      break;

    default:
      break;
  }
}

static void _bebop_emit_decorators(_bebop_emit_buf_t* buf, const bebop_decorator_t* dec)
{
  const bebop_intern_t* intern = &buf->ctx->intern;

  for (; dec; dec = dec->next) {
    _bebop_emit_indent(buf);
    _bebop_emit_char(buf, '@');
    _bebop_emit_str(buf, bebop_str_get(intern, dec->name));

    if (dec->arg_count > 0) {
      _bebop_emit_char(buf, '(');
      for (uint32_t i = 0; i < dec->arg_count; i++) {
        if (i > 0) {
          _bebop_emit_str(buf, ", ");
        }
        const bebop_decorator_arg_t* arg = &dec->args[i];

        if (!bebop_str_is_null(arg->name)) {
          _bebop_emit_str(buf, bebop_str_get(intern, arg->name));
          _bebop_emit_str(buf, ": ");
        }
        _bebop_emit_literal(buf, &arg->value);
      }
      _bebop_emit_char(buf, ')');
    }
    _bebop_emit_newline(buf);
  }
}

static void _bebop_emit_doc(_bebop_emit_buf_t* buf, const bebop_str_t doc)
{
  if (bebop_str_is_null(doc)) {
    return;
  }

  const char* text = bebop_str_get(&buf->ctx->intern, doc);
  if (!text || !*text) {
    return;
  }

  const char* start = text;
  for (; *text; text++) {
    if (text == start || *(text - 1) == '\n') {
      _bebop_emit_indent(buf);
      _bebop_emit_str(buf, "/// ");
    }
    _bebop_emit_char(buf, *text);
  }
  _bebop_emit_newline(buf);
}

typedef enum {
  EMIT_TYPE_VISIT,
  EMIT_TYPE_ARRAY_SUFFIX,
  EMIT_TYPE_FIXED_ARRAY_SUFFIX,
  EMIT_TYPE_MAP_COMMA,
  EMIT_TYPE_MAP_CLOSE,
} _bebop_emit_type_action_t;

typedef struct {
  _bebop_emit_type_action_t action;
  const bebop_type_t* type;
} _bebop_emit_type_frame_t;

#define BEBOP_EMIT_TYPE_STACK_SIZE 128

static void _bebop_emit_type(_bebop_emit_buf_t* buf, const bebop_type_t* type)
{
  if (!type) {
    return;
  }

  _bebop_emit_type_frame_t stack[BEBOP_EMIT_TYPE_STACK_SIZE];
  int sp = 0;

  stack[sp++] = (_bebop_emit_type_frame_t) {EMIT_TYPE_VISIT, type};

  while (sp > 0) {
    const _bebop_emit_type_frame_t frame = stack[--sp];

    switch (frame.action) {
      case EMIT_TYPE_VISIT: {
        const bebop_type_t* t = frame.type;
        if (!t) {
          continue;
        }

        switch (t->kind) {
          case BEBOP_TYPE_BOOL:
            _bebop_emit_str(buf, "bool");
            break;
          case BEBOP_TYPE_BYTE:
            _bebop_emit_str(buf, "byte");
            break;
          case BEBOP_TYPE_INT8:
            _bebop_emit_str(buf, "int8");
            break;
          case BEBOP_TYPE_INT16:
            _bebop_emit_str(buf, "int16");
            break;
          case BEBOP_TYPE_UINT16:
            _bebop_emit_str(buf, "uint16");
            break;
          case BEBOP_TYPE_INT32:
            _bebop_emit_str(buf, "int32");
            break;
          case BEBOP_TYPE_UINT32:
            _bebop_emit_str(buf, "uint32");
            break;
          case BEBOP_TYPE_INT64:
            _bebop_emit_str(buf, "int64");
            break;
          case BEBOP_TYPE_UINT64:
            _bebop_emit_str(buf, "uint64");
            break;
          case BEBOP_TYPE_INT128:
            _bebop_emit_str(buf, "int128");
            break;
          case BEBOP_TYPE_UINT128:
            _bebop_emit_str(buf, "uint128");
            break;
          case BEBOP_TYPE_FLOAT16:
            _bebop_emit_str(buf, "float16");
            break;
          case BEBOP_TYPE_FLOAT32:
            _bebop_emit_str(buf, "float32");
            break;
          case BEBOP_TYPE_FLOAT64:
            _bebop_emit_str(buf, "float64");
            break;
          case BEBOP_TYPE_BFLOAT16:
            _bebop_emit_str(buf, "bfloat16");
            break;
          case BEBOP_TYPE_STRING:
            _bebop_emit_str(buf, "string");
            break;
          case BEBOP_TYPE_UUID:
            _bebop_emit_str(buf, "uuid");
            break;
          case BEBOP_TYPE_TIMESTAMP:
            _bebop_emit_str(buf, "timestamp");
            break;
          case BEBOP_TYPE_DURATION:
            _bebop_emit_str(buf, "duration");
            break;

          case BEBOP_TYPE_ARRAY:
            if (sp + 2 > BEBOP_EMIT_TYPE_STACK_SIZE) {
              break;
            }
            stack[sp++] = (_bebop_emit_type_frame_t) {EMIT_TYPE_ARRAY_SUFFIX, NULL};
            stack[sp++] = (_bebop_emit_type_frame_t) {EMIT_TYPE_VISIT, t->array.element};
            break;

          case BEBOP_TYPE_FIXED_ARRAY:
            if (sp + 2 > BEBOP_EMIT_TYPE_STACK_SIZE) {
              break;
            }
            stack[sp++] = (_bebop_emit_type_frame_t) {EMIT_TYPE_FIXED_ARRAY_SUFFIX, t};
            stack[sp++] = (_bebop_emit_type_frame_t) {EMIT_TYPE_VISIT, t->fixed_array.element};
            break;

          case BEBOP_TYPE_MAP:
            _bebop_emit_str(buf, "map[");
            if (sp + 3 > BEBOP_EMIT_TYPE_STACK_SIZE) {
              break;
            }
            stack[sp++] = (_bebop_emit_type_frame_t) {EMIT_TYPE_MAP_CLOSE, NULL};
            stack[sp++] = (_bebop_emit_type_frame_t) {EMIT_TYPE_VISIT, t->map.value};
            stack[sp++] = (_bebop_emit_type_frame_t) {EMIT_TYPE_MAP_COMMA, NULL};
            stack[sp++] = (_bebop_emit_type_frame_t) {EMIT_TYPE_VISIT, t->map.key};
            break;

          case BEBOP_TYPE_DEFINED:
            _bebop_emit_str(buf, bebop_str_get(&buf->ctx->intern, t->defined.name));
            break;

          default:
            break;
        }
        break;
      }

      case EMIT_TYPE_ARRAY_SUFFIX:
        _bebop_emit_str(buf, "[]");
        break;

      case EMIT_TYPE_FIXED_ARRAY_SUFFIX:
        _bebop_emit_char(buf, '[');
        _bebop_emit_u32(buf, frame.type->fixed_array.size);
        _bebop_emit_char(buf, ']');
        break;

      case EMIT_TYPE_MAP_COMMA:
        _bebop_emit_str(buf, ", ");
        break;

      case EMIT_TYPE_MAP_CLOSE:
        _bebop_emit_char(buf, ']');
        break;
    }
  }
}

static void _bebop_emit_struct_fields(
    _bebop_emit_buf_t* buf, const bebop_field_t* fields, const uint32_t count
)
{
  for (uint32_t i = 0; i < count; i++) {
    const bebop_field_t* f = &fields[i];
    _bebop_emit_doc(buf, f->documentation);
    _bebop_emit_decorators(buf, f->decorators);
    _bebop_emit_indent(buf);
    _bebop_emit_str(buf, bebop_str_get(&buf->ctx->intern, f->name));
    _bebop_emit_str(buf, ": ");
    _bebop_emit_type(buf, f->type);
    _bebop_emit_char(buf, ';');
    _bebop_emit_newline(buf);
  }
}

static void _bebop_emit_message_fields(
    _bebop_emit_buf_t* buf, const bebop_field_t* fields, const uint32_t count
)
{
  for (uint32_t i = 0; i < count; i++) {
    const bebop_field_t* f = &fields[i];
    _bebop_emit_doc(buf, f->documentation);
    _bebop_emit_decorators(buf, f->decorators);
    _bebop_emit_indent(buf);
    _bebop_emit_str(buf, bebop_str_get(&buf->ctx->intern, f->name));
    _bebop_emit_char(buf, '(');
    _bebop_emit_u32(buf, f->index);
    _bebop_emit_str(buf, "): ");
    _bebop_emit_type(buf, f->type);
    _bebop_emit_char(buf, ';');
    _bebop_emit_newline(buf);
  }
}

static void _bebop_emit_enum_members(
    _bebop_emit_buf_t* buf,
    const bebop_enum_member_t* members,
    const uint32_t count,
    const bebop_type_kind_t base_type
)
{
  const bool is_signed = base_type == BEBOP_TYPE_INT8 || base_type == BEBOP_TYPE_INT16
      || base_type == BEBOP_TYPE_INT32 || base_type == BEBOP_TYPE_INT64
      || base_type == BEBOP_TYPE_INT128;

  for (uint32_t i = 0; i < count; i++) {
    const bebop_enum_member_t* m = &members[i];
    _bebop_emit_doc(buf, m->documentation);
    _bebop_emit_decorators(buf, m->decorators);
    _bebop_emit_indent(buf);
    _bebop_emit_str(buf, bebop_str_get(&buf->ctx->intern, m->name));
    _bebop_emit_str(buf, " = ");
    if (!bebop_str_is_null(m->value_expr)) {
      _bebop_emit_str(buf, bebop_str_get(&buf->ctx->intern, m->value_expr));
    } else if (is_signed) {
      _bebop_emit_i64(buf, (int64_t)m->value);
    } else {
      _bebop_emit_u64(buf, m->value);
    }
    _bebop_emit_char(buf, ';');
    _bebop_emit_newline(buf);
  }
}

static void _bebop_emit_union_branches(
    _bebop_emit_buf_t* buf, const bebop_union_branch_t* branches, const uint32_t count
)
{
  for (uint32_t i = 0; i < count; i++) {
    const bebop_union_branch_t* b = &branches[i];
    _bebop_emit_doc(buf, b->documentation);
    _bebop_emit_decorators(buf, b->decorators);
    _bebop_emit_indent(buf);
    if (b->def && b->def->visibility == BEBOP_VIS_EXPORT) {
      _bebop_emit_str(buf, "export ");
    }
    _bebop_emit_str(buf, bebop_str_get(&buf->ctx->intern, b->name));
    _bebop_emit_char(buf, '(');
    _bebop_emit_u32(buf, b->discriminator);
    _bebop_emit_str(buf, "): ");
    if (b->def) {
      const bool is_message = b->def->kind == BEBOP_DEF_MESSAGE;
      if (is_message) {
        _bebop_emit_str(buf, "message {");
      } else if (b->def->struct_def.is_mutable) {
        _bebop_emit_str(buf, "mut {");
      } else {
        _bebop_emit_char(buf, '{');
      }
      _bebop_emit_newline(buf);
      buf->indent++;
      for (uint32_t j = 0; j < b->def->struct_def.field_count; j++) {
        const bebop_field_t* f = &b->def->struct_def.fields[j];
        _bebop_emit_indent(buf);
        _bebop_emit_str(buf, bebop_str_get(&buf->ctx->intern, f->name));
        if (is_message) {
          _bebop_emit_char(buf, '(');
          _bebop_emit_u32(buf, f->index);
          _bebop_emit_char(buf, ')');
        }
        _bebop_emit_str(buf, ": ");
        _bebop_emit_type(buf, f->type);
        _bebop_emit_char(buf, ';');
        _bebop_emit_newline(buf);
      }
      buf->indent--;
      _bebop_emit_indent(buf);
      _bebop_emit_char(buf, '}');
      _bebop_emit_newline(buf);
    } else if (b->type_ref) {
      _bebop_emit_type(buf, b->type_ref);
      _bebop_emit_newline(buf);
    }
  }
}

static void _bebop_emit_methods(
    _bebop_emit_buf_t* buf, const bebop_method_t* methods, const uint32_t count
)
{
  for (uint32_t i = 0; i < count; i++) {
    const bebop_method_t* m = &methods[i];
    _bebop_emit_doc(buf, m->documentation);
    _bebop_emit_decorators(buf, m->decorators);
    _bebop_emit_indent(buf);
    _bebop_emit_str(buf, bebop_str_get(&buf->ctx->intern, m->name));
    _bebop_emit_char(buf, '(');

    const bool req_stream = m->method_type == BEBOP_METHOD_CLIENT_STREAM
        || m->method_type == BEBOP_METHOD_DUPLEX_STREAM;
    if (req_stream) {
      _bebop_emit_str(buf, "stream ");
    }
    _bebop_emit_type(buf, m->request_type);

    _bebop_emit_str(buf, "): ");

    const bool res_stream = m->method_type == BEBOP_METHOD_SERVER_STREAM
        || m->method_type == BEBOP_METHOD_DUPLEX_STREAM;
    if (res_stream) {
      _bebop_emit_str(buf, "stream ");
    }
    _bebop_emit_type(buf, m->response_type);

    _bebop_emit_char(buf, ';');
    _bebop_emit_newline(buf);
  }
}

static bool _bebop_is_inline_branch_def(const bebop_def_t* parent, const bebop_def_t* nested)
{
  if (parent->kind != BEBOP_DEF_UNION) {
    return false;
  }
  for (uint32_t i = 0; i < parent->union_def.branch_count; i++) {
    if (parent->union_def.branches[i].def == nested) {
      return true;
    }
  }
  return false;
}

static void _bebop_emit_nested_defs(_bebop_emit_buf_t* buf, const bebop_def_t* def)
{
  for (const bebop_def_t* nested = def->nested_defs; nested; nested = nested->next) {
    // Skip inline branch definitions - they're emitted as part of the branch
    if (_bebop_is_inline_branch_def(def, nested)) {
      continue;
    }
    _bebop_emit_newline(buf);
    _bebop_emit_def(buf, nested);
  }
}

static void _bebop_emit_visibility(_bebop_emit_buf_t* buf, const bebop_def_t* def)
{
  const bool is_nested = def->parent != NULL;

  if (!is_nested && def->visibility == BEBOP_VIS_LOCAL) {
    _bebop_emit_str(buf, "local ");
  } else if (is_nested && def->visibility == BEBOP_VIS_EXPORT) {
    _bebop_emit_str(buf, "export ");
  }
}

static void _bebop_emit_def(_bebop_emit_buf_t* buf, const bebop_def_t* def)
{
  if (!def) {
    return;
  }

  const bebop_intern_t* intern = &buf->ctx->intern;

  _bebop_emit_doc(buf, def->documentation);
  _bebop_emit_decorators(buf, def->decorators);

  switch (def->kind) {
    case BEBOP_DEF_ENUM:
      _bebop_emit_indent(buf);
      _bebop_emit_visibility(buf, def);
      _bebop_emit_str(buf, "enum ");
      _bebop_emit_str(buf, bebop_str_get(intern, def->name));
      if (def->enum_def.base_type != BEBOP_TYPE_UINT32) {
        _bebop_emit_str(buf, " : ");
        _bebop_emit_str(buf, bebop_type_kind_name(def->enum_def.base_type));
      }
      _bebop_emit_str(buf, " {");
      _bebop_emit_newline(buf);
      buf->indent++;
      _bebop_emit_enum_members(
          buf, def->enum_def.members, def->enum_def.member_count, def->enum_def.base_type
      );
      buf->indent--;
      _bebop_emit_indent(buf);
      _bebop_emit_char(buf, '}');
      _bebop_emit_newline(buf);
      break;

    case BEBOP_DEF_STRUCT:
      _bebop_emit_indent(buf);
      _bebop_emit_visibility(buf, def);
      if (def->struct_def.is_mutable) {
        _bebop_emit_str(buf, "mut ");
      }
      _bebop_emit_str(buf, "struct ");
      _bebop_emit_str(buf, bebop_str_get(intern, def->name));
      _bebop_emit_str(buf, " {");
      _bebop_emit_newline(buf);
      buf->indent++;
      _bebop_emit_struct_fields(buf, def->struct_def.fields, def->struct_def.field_count);
      _bebop_emit_nested_defs(buf, def);
      buf->indent--;
      _bebop_emit_indent(buf);
      _bebop_emit_char(buf, '}');
      _bebop_emit_newline(buf);
      break;

    case BEBOP_DEF_MESSAGE:
      _bebop_emit_indent(buf);
      _bebop_emit_visibility(buf, def);
      _bebop_emit_str(buf, "message ");
      _bebop_emit_str(buf, bebop_str_get(intern, def->name));
      _bebop_emit_str(buf, " {");
      _bebop_emit_newline(buf);
      buf->indent++;
      _bebop_emit_message_fields(buf, def->message_def.fields, def->message_def.field_count);
      _bebop_emit_nested_defs(buf, def);
      buf->indent--;
      _bebop_emit_indent(buf);
      _bebop_emit_char(buf, '}');
      _bebop_emit_newline(buf);
      break;

    case BEBOP_DEF_UNION:
      _bebop_emit_indent(buf);
      _bebop_emit_visibility(buf, def);
      _bebop_emit_str(buf, "union ");
      _bebop_emit_str(buf, bebop_str_get(intern, def->name));
      _bebop_emit_str(buf, " {");
      _bebop_emit_newline(buf);
      buf->indent++;
      _bebop_emit_union_branches(buf, def->union_def.branches, def->union_def.branch_count);
      _bebop_emit_nested_defs(buf, def);
      buf->indent--;
      _bebop_emit_indent(buf);
      _bebop_emit_char(buf, '}');
      _bebop_emit_newline(buf);
      break;

    case BEBOP_DEF_SERVICE:
      _bebop_emit_indent(buf);
      _bebop_emit_visibility(buf, def);
      _bebop_emit_str(buf, "service ");
      _bebop_emit_str(buf, bebop_str_get(intern, def->name));
      _bebop_emit_str(buf, " {");
      _bebop_emit_newline(buf);
      buf->indent++;
      _bebop_emit_methods(buf, def->service_def.methods, def->service_def.method_count);
      buf->indent--;
      _bebop_emit_indent(buf);
      _bebop_emit_char(buf, '}');
      _bebop_emit_newline(buf);
      break;

    case BEBOP_DEF_CONST:
      _bebop_emit_indent(buf);
      _bebop_emit_visibility(buf, def);
      _bebop_emit_str(buf, "const ");
      _bebop_emit_type(buf, def->const_def.type);
      _bebop_emit_char(buf, ' ');
      _bebop_emit_str(buf, bebop_str_get(intern, def->name));
      _bebop_emit_str(buf, " = ");
      _bebop_emit_literal(buf, &def->const_def.value);
      _bebop_emit_char(buf, ';');
      _bebop_emit_newline(buf);
      break;

    case BEBOP_DEF_DECORATOR: {
      _bebop_emit_str(buf, "#decorator(");
      _bebop_emit_str(buf, bebop_str_get(intern, def->name));
      _bebop_emit_str(buf, ") {\n");
      buf->indent++;

      _bebop_emit_indent(buf);
      _bebop_emit_str(buf, "targets = ");
      {
        static const struct {
          bebop_decorator_target_t bit;
          const char* name;
        } targets[] = {
            {BEBOP_TARGET_ENUM, "ENUM"},
            {BEBOP_TARGET_STRUCT, "STRUCT"},
            {BEBOP_TARGET_MESSAGE, "MESSAGE"},
            {BEBOP_TARGET_UNION, "UNION"},
            {BEBOP_TARGET_FIELD, "FIELD"},
            {BEBOP_TARGET_SERVICE, "SERVICE"},
            {BEBOP_TARGET_METHOD, "METHOD"},
            {BEBOP_TARGET_BRANCH, "BRANCH"},
        };

        const bebop_decorator_target_t t = def->decorator_def.targets;
        if (t == BEBOP_TARGET_ALL) {
          _bebop_emit_str(buf, "ALL");
        } else {
          bool first = true;
          for (size_t i = 0; i < sizeof(targets) / sizeof(targets[0]); i++) {
            if (t & targets[i].bit) {
              if (!first) {
                _bebop_emit_str(buf, " | ");
              }
              _bebop_emit_str(buf, targets[i].name);
              first = false;
            }
          }
        }
      }
      _bebop_emit_newline(buf);

      if (def->decorator_def.allow_multiple) {
        _bebop_emit_indent(buf);
        _bebop_emit_str(buf, "multiple = true\n");
      }

      for (uint32_t i = 0; i < def->decorator_def.param_count; i++) {
        const bebop_macro_param_def_t* p = &def->decorator_def.params[i];
        _bebop_emit_indent(buf);
        _bebop_emit_str(buf, "param ");
        _bebop_emit_str(buf, bebop_str_get(intern, p->name));
        _bebop_emit_char(buf, p->required ? '!' : '?');
        _bebop_emit_str(buf, ": ");
        if (p->type == BEBOP_TYPE_DEFINED) {
          _bebop_emit_str(buf, "type");
        } else {
          _bebop_emit_str(buf, bebop_type_kind_name(p->type));
        }
        _bebop_emit_newline(buf);
      }

      if (def->decorator_def.validate_span.len > 0 && def->schema->source) {
        _bebop_emit_indent(buf);
        _bebop_emit_str(buf, "validate [[");
        _bebop_emit_str_n(
            buf,
            def->schema->source + def->decorator_def.validate_span.off,
            def->decorator_def.validate_span.len
        );
        _bebop_emit_str(buf, "]]\n");
      }

      if (def->decorator_def.export_span.len > 0 && def->schema->source) {
        _bebop_emit_indent(buf);
        _bebop_emit_str(buf, "export [[");
        _bebop_emit_str_n(
            buf,
            def->schema->source + def->decorator_def.export_span.off,
            def->decorator_def.export_span.len
        );
        _bebop_emit_str(buf, "]]\n");
      }

      buf->indent--;
      _bebop_emit_str(buf, "}\n");
      break;
    }

    default:
      break;
  }
}

const char* bebop_emit_schema(const bebop_schema_t* schema, size_t* len)
{
  if (!schema) {
    if (len) {
      *len = 0;
    }
    return NULL;
  }

  _bebop_emit_buf_t buf;
  _bebop_emit_init(&buf, schema);

  switch (schema->edition) {
    case BEBOP_ED_2026:
      _bebop_emit_str(&buf, "edition = \"2026\"\n\n");
      break;
    default:
      break;
  }

  if (!bebop_str_is_null(schema->package)) {
    _bebop_emit_str(&buf, "package ");
    _bebop_emit_str(&buf, bebop_str_get(&schema->ctx->intern, schema->package));
    _bebop_emit_str(&buf, ";\n\n");
  }

  if (schema->import_count > 0) {
    uint32_t* idx = bebop_arena_new(BEBOP_ARENA(schema->ctx), uint32_t, schema->import_count);
    if (idx) {
      for (uint32_t i = 0; i < schema->import_count; i++) {
        idx[i] = i;
      }

      for (uint32_t i = 1; i < schema->import_count; i++) {
        const uint32_t key = idx[i];
        const char* key_path = BEBOP_STR(schema->ctx, schema->imports[key].path);
        uint32_t j = i;
        while (j > 0
               && _bebop_strcmp(BEBOP_STR(schema->ctx, schema->imports[idx[j - 1]].path), key_path)
                   > 0)
        {
          idx[j] = idx[j - 1];
          j--;
        }
        idx[j] = key;
      }

      for (uint32_t i = 0; i < schema->import_count; i++) {
        _bebop_emit_str(&buf, "import \"");
        _bebop_emit_str(&buf, BEBOP_STR(schema->ctx, schema->imports[idx[i]].path));
        _bebop_emit_str(&buf, "\"\n");
      }
    }
    _bebop_emit_newline(&buf);
  }

  for (const bebop_def_t* def = schema->definitions; def; def = def->next) {
    if (def->parent) {
      continue;
    }
    _bebop_emit_trivia_until(&buf, def->span.off);
    _bebop_emit_def(&buf, def);
    _bebop_emit_newline(&buf);
  }

  _bebop_emit_trivia_until(&buf, schema->source_len);

  if (buf.error) {
    if (len) {
      *len = 0;
    }
    return NULL;
  }

  _bebop_emit_char(&buf, '\0');
  if (buf.error) {
    if (len) {
      *len = 0;
    }
    return NULL;
  }

  if (len) {
    *len = buf.len - 1;
  }
  return buf.data;
}
