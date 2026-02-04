#define bebop__IMPORT_INITIAL_CAPACITY 8
#define bebop__DIAGNOSTIC_INITIAL_CAPACITY 16

bebop_schema_t* bebop__schema_create(
    bebop_context_t* ctx, const char* path, const char* source, const size_t len
)
{
  BEBOP_ASSERT(ctx != NULL);

  bebop_schema_t* schema = bebop_arena_new(BEBOP_ARENA(ctx), bebop_schema_t, 1);
  if (BEBOP_UNLIKELY(!schema)) {
    return NULL;
  }

  schema->ctx = ctx;
  schema->source = source;
  schema->source_len = len;
  schema->edition = BEBOP_ED_2026;
  schema->state = BEBOP_SCHEMA_PARSED;

  if (path) {
    schema->path = bebop_arena_strdup(BEBOP_ARENA(ctx), path);
  }

  schema->def_table = bebop_defmap_new(16, BEBOP_ARENA(ctx));

  return schema;
}

#define bebop__SORTED_DEFS_INITIAL_CAPACITY 16

void bebop__schema_register_def(bebop_schema_t* schema, bebop_def_t* def)
{
  BEBOP_ASSERT(schema != NULL);
  BEBOP_ASSERT(def != NULL);

  bebop__def_compute_fqn(def);

  if (schema->sorted_defs_count >= schema->sorted_defs_capacity) {
    const uint32_t new_capacity = schema->sorted_defs_capacity
        ? schema->sorted_defs_capacity * 2
        : bebop__SORTED_DEFS_INITIAL_CAPACITY;
    bebop_def_t** new_arr = bebop_arena_new(BEBOP_ARENA(schema->ctx), bebop_def_t*, new_capacity);
    if (BEBOP_UNLIKELY(!new_arr)) {
      return;
    }

    if (schema->sorted_defs) {
      memcpy(new_arr, schema->sorted_defs, sizeof(bebop_def_t*) * schema->sorted_defs_count);
    }
    schema->sorted_defs = new_arr;
    schema->sorted_defs_capacity = new_capacity;
  }

  schema->sorted_defs[schema->sorted_defs_count++] = def;
}

void bebop__schema_add_def(bebop_schema_t* schema, bebop_def_t* def)
{
  BEBOP_ASSERT(schema != NULL);
  BEBOP_ASSERT(def != NULL);

  def->next = NULL;
  def->schema = schema;

  if (schema->definitions_tail) {
    schema->definitions_tail->next = def;
    schema->definitions_tail = def;
  } else {
    schema->definitions = def;
    schema->definitions_tail = def;
  }
  schema->definition_count++;

  bebop__schema_register_def(schema, def);

  if (!bebop_str_is_null(def->name)) {
    const bebop_defmap_Entry entry = {def->name.idx, def};
    bebop_defmap_insert(&schema->def_table, &entry);
  }
}

void bebop__def_add_nested(bebop_def_t* parent, bebop_def_t* nested)
{
  BEBOP_ASSERT(parent != NULL);
  BEBOP_ASSERT(nested != NULL);

  nested->next = NULL;
  nested->parent = parent;
  nested->schema = parent->schema;

  if (parent->nested_defs_tail) {
    parent->nested_defs_tail->next = nested;
    parent->nested_defs_tail = nested;
  } else {
    parent->nested_defs = nested;
    parent->nested_defs_tail = nested;
  }
  parent->nested_def_count++;

  bebop__schema_register_def(parent->schema, nested);
}

bebop_def_t* bebop__schema_find_def(bebop_schema_t* schema, const bebop_str_t name)
{
  BEBOP_ASSERT(schema != NULL);

  if (bebop_str_is_null(name)) {
    return NULL;
  }

  const bebop_defmap_Iter it = bebop_defmap_find(&schema->def_table, &name.idx);
  const bebop_defmap_Entry* entry = bebop_defmap_Iter_get(&it);
  return entry ? (bebop_def_t*)entry->val : NULL;
}

bebop_def_t* bebop__def_find_nested(bebop_def_t* parent, const bebop_str_t name)
{
  if (!parent || bebop_str_is_null(name)) {
    return NULL;
  }

  for (bebop_def_t* nested = parent->nested_defs; nested; nested = nested->next) {
    if (nested->name.idx == name.idx) {
      return nested;
    }
  }
  return NULL;
}

bool bebop__def_is_accessible(const bebop_def_t* def)
{
  if (!def) {
    return false;
  }

  for (const bebop_def_t* d = def; d != NULL; d = d->parent) {
    const bool is_nested = d->parent != NULL;
    bool is_exported;

    if (d->visibility == BEBOP_VIS_EXPORT) {
      is_exported = true;
    } else if (d->visibility == BEBOP_VIS_LOCAL) {
      is_exported = false;
    } else {
      is_exported = !is_nested;
    }

    if (!is_exported) {
      return false;
    }
  }
  return true;
}

bebop_str_t bebop__def_compute_fqn(bebop_def_t* def)
{
  BEBOP_ASSERT(def != NULL);
  BEBOP_ASSERT(def->schema != NULL);

  if (bebop_str_is_null(def->name)) {
    return BEBOP_STR_NULL;
  }

  bebop_context_t* ctx = def->schema->ctx;
  const bebop_schema_t* schema = def->schema;

  if (def->parent && bebop_str_is_null(def->parent->fqn)) {
    bebop__def_compute_fqn(def->parent);
  }

  const char* name = BEBOP_STR(ctx, def->name);
  const size_t name_len = BEBOP_STR_LEN(ctx, def->name);

  if (def->parent) {
    const char* parent_fqn = BEBOP_STR(ctx, def->parent->fqn);
    const size_t parent_len = BEBOP_STR_LEN(ctx, def->parent->fqn);

    char* fqn_buf = bebop__join_dotted(
        BEBOP_ARENA(ctx),
        (bebop__str_view_t) {parent_fqn, parent_len},
        (bebop__str_view_t) {name, name_len}
    );
    if (!fqn_buf) {
      return BEBOP_STR_NULL;
    }

    def->fqn = bebop_intern_n(BEBOP_INTERN(ctx), fqn_buf, parent_len + 1 + name_len);
  } else if (!bebop_str_is_null(schema->package)) {
    const char* pkg = BEBOP_STR(ctx, schema->package);
    const size_t pkg_len = BEBOP_STR_LEN(ctx, schema->package);

    char* fqn_buf = bebop__join_dotted(
        BEBOP_ARENA(ctx), (bebop__str_view_t) {pkg, pkg_len}, (bebop__str_view_t) {name, name_len}
    );
    if (!fqn_buf) {
      return BEBOP_STR_NULL;
    }

    def->fqn = bebop_intern_n(BEBOP_INTERN(ctx), fqn_buf, pkg_len + 1 + name_len);
  } else {
    def->fqn = def->name;
  }

  return def->fqn;
}

void bebop__schema_add_import(
    bebop_schema_t* schema, const bebop_str_t path, const bebop_span_t span
)
{
  BEBOP_ASSERT(schema != NULL);

  if (schema->import_count >= schema->import_capacity) {
    const uint32_t new_capacity =
        schema->import_capacity ? schema->import_capacity * 2 : bebop__IMPORT_INITIAL_CAPACITY;
    bebop_import_t* new_imports =
        bebop_arena_new(BEBOP_ARENA(schema->ctx), bebop_import_t, new_capacity);
    if (BEBOP_UNLIKELY(!new_imports)) {
      return;
    }

    if (schema->imports) {
      memcpy(new_imports, schema->imports, sizeof(bebop_import_t) * schema->import_count);
    }
    schema->imports = new_imports;
    schema->import_capacity = new_capacity;
  }

  schema->imports[schema->import_count++] = (bebop_import_t) {
      .path = path,
      .span = span,
  };
}

bool bebop__schema_has_visibility(bebop_schema_t* source, bebop_schema_t* target)
{
  if (!source || !target) {
    return false;
  }
  if (source == target) {
    return true;
  }

  for (uint32_t i = 0; i < source->import_count; i++) {
    if (source->imports[i].schema == target) {
      return true;
    }
  }
  return false;
}

void bebop__schema_add_diagnostic(
    bebop_schema_t* schema, const bebop__diag_loc_t loc, const char* message, const char* hint
)
{
  BEBOP_ASSERT(schema != NULL);
  BEBOP_ASSERT(message != NULL);

  if (schema->diagnostic_count >= BEBOP_MAX_DIAGNOSTICS) {
    return;
  }

  if (schema->diagnostic_count >= schema->diagnostic_capacity) {
    const uint32_t new_capacity = schema->diagnostic_capacity ? schema->diagnostic_capacity * 2
                                                              : bebop__DIAGNOSTIC_INITIAL_CAPACITY;
    bebop_diagnostic_t* new_diags =
        bebop_arena_new(BEBOP_ARENA(schema->ctx), bebop_diagnostic_t, new_capacity);
    if (BEBOP_UNLIKELY(!new_diags)) {
      return;
    }

    if (schema->diagnostics) {
      memcpy(new_diags, schema->diagnostics, sizeof(bebop_diagnostic_t) * schema->diagnostic_count);
    }
    schema->diagnostics = new_diags;
    schema->diagnostic_capacity = new_capacity;
  }

  bebop_diagnostic_t* diag = &schema->diagnostics[schema->diagnostic_count++];
  diag->severity = loc.severity;
  diag->code = loc.code;
  diag->span = loc.span;
  diag->path = schema->path;
  diag->message = bebop_arena_strdup(BEBOP_ARENA(schema->ctx), message);
  diag->hint = hint ? bebop_arena_strdup(BEBOP_ARENA(schema->ctx), hint) : NULL;
  diag->labels = NULL;
  diag->label_count = 0;

  if (loc.severity == BEBOP_DIAG_ERROR) {
    schema->error_count++;
  } else if (loc.severity == BEBOP_DIAG_WARNING) {
    schema->warning_count++;
  }
}

void bebop__schema_diag_add_label(bebop_schema_t* schema, bebop_span_t span, const char* message)
{
  if (schema->diagnostic_count == 0) {
    return;
  }
  bebop_diagnostic_t* diag = &schema->diagnostics[schema->diagnostic_count - 1];

  const uint32_t new_count = diag->label_count + 1;
  bebop_diag_label_t* new_labels =
      bebop_arena_new(BEBOP_ARENA(schema->ctx), bebop_diag_label_t, new_count);
  if (!new_labels) {
    return;
  }

  if (diag->labels && diag->label_count > 0) {
    memcpy(new_labels, diag->labels, sizeof(bebop_diag_label_t) * diag->label_count);
  }
  new_labels[diag->label_count].span = span;
  new_labels[diag->label_count].message =
      message ? bebop_arena_strdup(BEBOP_ARENA(schema->ctx), message) : NULL;
  diag->labels = new_labels;
  diag->label_count = new_count;
}
