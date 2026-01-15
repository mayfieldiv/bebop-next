#define _bebop_SCHEMA_INITIAL_CAPACITY 4

bebop_parse_result_t* _bebop_result_create(bebop_context_t* ctx)
{
  BEBOP_ASSERT(ctx != NULL);

  bebop_parse_result_t* result = bebop_arena_new(BEBOP_ARENA(ctx), bebop_parse_result_t, 1);
  if (BEBOP_UNLIKELY(!result)) {
    _bebop_context_set_error(ctx, BEBOP_ERR_OUT_OF_MEMORY, "Failed to allocate parse result");
    return NULL;
  }
  result->ctx = ctx;

  result->def_table = bebop_defmap_new(64, BEBOP_ARENA(ctx));

  return result;
}

void _bebop_result_add_schema(bebop_parse_result_t* result, bebop_schema_t* schema)
{
  BEBOP_ASSERT(result != NULL);
  BEBOP_ASSERT(schema != NULL);

  if (result->schema_count >= result->schema_capacity) {
    const uint32_t new_capacity =
        result->schema_capacity ? result->schema_capacity * 2 : _bebop_SCHEMA_INITIAL_CAPACITY;
    bebop_schema_t** new_schemas =
        bebop_arena_new(BEBOP_ARENA(result->ctx), bebop_schema_t*, new_capacity);
    if (BEBOP_UNLIKELY(!new_schemas)) {
      return;
    }

    if (result->schemas) {
      memcpy(new_schemas, result->schemas, sizeof(bebop_schema_t*) * result->schema_count);
    }
    result->schemas = new_schemas;
    result->schema_capacity = new_capacity;
  }

  result->schemas[result->schema_count++] = schema;
  schema->result = result;

  for (uint32_t i = 0; i < schema->sorted_defs_count; i++) {
    bebop_def_t* def = schema->sorted_defs[i];
    if (!def || bebop_str_is_null(def->fqn)) {
      continue;
    }

    bebop_defmap_Entry fqn_entry = {def->fqn.idx, def};
    bebop_defmap_insert(&result->def_table, &fqn_entry);
  }

  result->total_error_count += schema->error_count;
  result->total_warning_count += schema->warning_count;
}

bebop_def_t* _bebop_result_find_def(bebop_parse_result_t* result, const char* name)
{
  BEBOP_ASSERT(result != NULL);

  if (!name) {
    return NULL;
  }

  const bebop_str_t interned = bebop_intern(BEBOP_INTERN(result->ctx), name);
  if (bebop_str_is_null(interned)) {
    return NULL;
  }

  const bebop_defmap_Iter it = bebop_defmap_find(&result->def_table, &interned.idx);
  const bebop_defmap_Entry* entry = bebop_defmap_Iter_get(&it);
  return entry ? (bebop_def_t*)entry->val : NULL;
}

static bebop_def_t* _bebop_find_nested_by_name(
    bebop_def_t* parent, const char* name, const size_t len, bebop_intern_t* intern
)
{
  if (!parent || !name || len == 0) {
    return NULL;
  }

  const bebop_str_t interned = bebop_intern_n(intern, name, len);
  if (bebop_str_is_null(interned)) {
    return NULL;
  }

  return _bebop_def_find_nested(parent, interned);
}

static bool _bebop_is_visible(bebop_schema_t* from, bebop_def_t* def)
{
  if (!def || !def->schema) {
    return false;
  }
  return _bebop_schema_has_visibility(from, def->schema);
}

bebop_def_t* _bebop_result_resolve_type(
    bebop_parse_result_t* result,
    bebop_schema_t* schema,
    bebop_def_t* context_def,
    const char* name,
    bebop_def_t** ambiguous_with
)
{
  BEBOP_ASSERT(result != NULL);
  BEBOP_ASSERT(schema != NULL);

  if (ambiguous_with) {
    *ambiguous_with = NULL;
  }

  if (!name || !name[0]) {
    return NULL;
  }

  bebop_context_t* ctx = result->ctx;

  const char* dot = strchr(name, '.');
  if (dot) {
    const size_t first_len = (size_t)(dot - name);
    char* first_part = bebop_arena_strndup(BEBOP_ARENA(ctx), name, first_len);
    if (!first_part) {
      return NULL;
    }

    bebop_def_t* resolved =
        _bebop_result_resolve_type(result, schema, context_def, first_part, NULL);
    if (!resolved) {
      resolved = _bebop_result_find_def(result, name);
      if (resolved && !_bebop_is_visible(schema, resolved)) {
        return NULL;
      }
      return resolved;
    }

    const char* remaining = dot + 1;
    while (remaining && *remaining && resolved) {
      const char* next_dot = strchr(remaining, '.');
      const size_t part_len = next_dot ? (size_t)(next_dot - remaining) : strlen(remaining);
      resolved = _bebop_find_nested_by_name(resolved, remaining, part_len, BEBOP_INTERN(ctx));
      remaining = next_dot ? next_dot + 1 : NULL;
    }

    return resolved;
  }

  const bebop_str_t interned = bebop_intern(BEBOP_INTERN(ctx), name);
  if (bebop_str_is_null(interned)) {
    return NULL;
  }

  if (context_def) {
    bebop_def_t* nested = _bebop_def_find_nested(context_def, interned);
    if (nested) {
      return nested;
    }

    for (bebop_def_t* parent = context_def->parent; parent; parent = parent->parent) {
      nested = _bebop_def_find_nested(parent, interned);
      if (nested) {
        return nested;
      }
    }
  }

  bebop_def_t* found = _bebop_schema_find_def(schema, interned);
  if (found) {
    return found;
  }

  for (uint32_t i = 0; i < schema->import_count; i++) {
    if (!schema->imports[i].schema) {
      continue;
    }
    found = _bebop_schema_find_def(schema->imports[i].schema, interned);
    if (found) {
      if (ambiguous_with) {
        for (uint32_t j = i + 1; j < schema->import_count; j++) {
          if (!schema->imports[j].schema) {
            continue;
          }
          bebop_def_t* other = _bebop_schema_find_def(schema->imports[j].schema, interned);
          if (other) {
            *ambiguous_with = other;
            break;
          }
        }
      }
      return found;
    }
  }

  if (!bebop_str_is_null(schema->package)) {
    const char* pkg = BEBOP_STR(ctx, schema->package);
    const size_t pkg_len = BEBOP_STR_LEN(ctx, schema->package);
    const size_t name_len = strlen(name);

    const char* fqn_buf = _bebop_join_dotted(
        BEBOP_ARENA(ctx), (_bebop_str_view_t) {pkg, pkg_len}, (_bebop_str_view_t) {name, name_len}
    );
    if (fqn_buf) {
      found = _bebop_result_find_def(result, fqn_buf);
      if (found && _bebop_is_visible(schema, found)) {
        return found;
      }
    }
  }

  found = _bebop_result_find_def(result, name);
  if (found && _bebop_is_visible(schema, found)) {
    return found;
  }

  return NULL;
}
