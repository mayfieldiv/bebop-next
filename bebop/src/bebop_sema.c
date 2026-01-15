static const bebop_type_kind_t _bebop_sema_valid_map_keys[] = {
    BEBOP_TYPE_BOOL,
    BEBOP_TYPE_BYTE,
    BEBOP_TYPE_INT8,
    BEBOP_TYPE_INT16,
    BEBOP_TYPE_UINT16,
    BEBOP_TYPE_INT32,
    BEBOP_TYPE_UINT32,
    BEBOP_TYPE_INT64,
    BEBOP_TYPE_UINT64,
    BEBOP_TYPE_INT128,
    BEBOP_TYPE_UINT128,
    BEBOP_TYPE_STRING,
    BEBOP_TYPE_UUID,
};

#define _bebop_SEMA_VALID_MAP_KEY_COUNT BEBOP_COUNTOF(_bebop_sema_valid_map_keys)

bool bebop_sema_init(bebop_sema_t* sema, bebop_context_t* ctx, bebop_schema_t* schema)
{
  BEBOP_ASSERT(sema != NULL);
  BEBOP_ASSERT(ctx != NULL);
  BEBOP_ASSERT(schema != NULL);

  memset(sema, 0, sizeof(*sema));
  sema->ctx = ctx;
  sema->schema = schema;
  sema->current_def = NULL;

  sema->name_scope = bebop_defmap_new(BEBOP_SEMA_INITIAL_SCOPE_CAPACITY, BEBOP_ARENA(ctx));
  if (!sema->name_scope.set_.ctrl_) {
    _bebop_context_set_error(ctx, BEBOP_ERR_OUT_OF_MEMORY, "Failed to allocate sema name scope");
    return false;
  }

  return true;
}

void bebop_sema_enter_def(bebop_sema_t* sema, bebop_def_t* def)
{
  BEBOP_ASSERT(sema != NULL);

  sema->current_def = def;
  bebop_defmap_clear(&sema->name_scope);
  memset(sema->seen_indices, 0, sizeof(sema->seen_indices));
  memset(sema->seen_index_spans, 0, sizeof(sema->seen_index_spans));
}

void bebop_sema_exit_def(bebop_sema_t* sema)
{
  BEBOP_ASSERT(sema != NULL);

  sema->current_def = NULL;
  bebop_defmap_clear(&sema->name_scope);
  memset(sema->seen_indices, 0, sizeof(sema->seen_indices));
  memset(sema->seen_index_spans, 0, sizeof(sema->seen_index_spans));
}

static void _bebop_sema_int_range(
    const bebop_type_kind_t kind, int64_t* min, uint64_t* max, bool* is_unsigned
)
{
  *is_unsigned = false;
  switch (kind) {
    case BEBOP_TYPE_BYTE:
      *min = 0;
      *max = 255;
      *is_unsigned = true;
      break;
    case BEBOP_TYPE_INT8:
      *min = -128;
      *max = 127;
      break;
    case BEBOP_TYPE_INT16:
      *min = -32768;
      *max = 32767;
      break;
    case BEBOP_TYPE_UINT16:
      *min = 0;
      *max = 65535;
      *is_unsigned = true;
      break;
    case BEBOP_TYPE_INT32:
      *min = -2147483648LL;
      *max = 2147483647LL;
      break;
    case BEBOP_TYPE_UINT32:
      *min = 0;
      *max = 4294967295ULL;
      *is_unsigned = true;
      break;
    case BEBOP_TYPE_INT64:
      *min = INT64_MIN;
      *max = (uint64_t)INT64_MAX;
      break;
    case BEBOP_TYPE_UINT64:
      *min = 0;
      *max = UINT64_MAX;
      *is_unsigned = true;
      break;
    default:
      BEBOP_ASSERT_MSG(false, "Invalid integer type kind for enum base");
      *min = 0;
      *max = 0;
      break;
  }
}

bool bebop_sema_check_enum_member(
    const bebop_sema_t* sema, const bebop_enum_member_t* member, const bebop_def_t* enum_def
)
{
  BEBOP_ASSERT(sema != NULL);
  BEBOP_ASSERT(member != NULL);
  BEBOP_ASSERT(enum_def != NULL);
  BEBOP_ASSERT(enum_def->kind == BEBOP_DEF_ENUM);

  const char* member_name = BEBOP_STR(sema->ctx, member->name);

  int64_t min_val;
  uint64_t max_val;
  bool is_unsigned;
  _bebop_sema_int_range(enum_def->enum_def.base_type, &min_val, &max_val, &is_unsigned);

  if (is_unsigned) {
    if (member->value > max_val) {
      goto out_of_range;
    }
  } else {
    if (member->value > (uint64_t)INT64_MAX) {
      if ((int64_t)member->value < min_val) {
        goto out_of_range;
      }
    } else {
      const int64_t signed_val = (int64_t)member->value;
      if (signed_val < min_val || signed_val > (int64_t)max_val) {
        goto out_of_range;
      }
    }
  }
  goto range_ok;

out_of_range:
  if (is_unsigned) {
    BEBOP_ERROR_FMT(
        sema->schema,
        BEBOP_DIAG_ENUM_VALUE_OVERFLOW,
        member->span,
        "Enum member '%s' has value %llu which is outside the " "range of the base type",
        member_name ? member_name : "",
        (unsigned long long)member->value
    );
  } else {
    BEBOP_ERROR_FMT(
        sema->schema,
        BEBOP_DIAG_ENUM_VALUE_OVERFLOW,
        member->span,
        "Enum member '%s' has value %lld which is outside the " "range of the base type",
        member_name ? member_name : "",
        (long long)(int64_t)member->value
    );
  }
  return false;

range_ok:;
  uint32_t decorator_count = 0;
  for (const bebop_decorator_t* dec = enum_def->decorators; dec != NULL; dec = dec->next) {
    if (++decorator_count > BEBOP_MAX_DECORATOR_CHAIN_LENGTH) {
      _bebop_context_set_error(
          sema->ctx, BEBOP_ERR_INTERNAL, "Decorator list exceeds maximum length (corrupted data)"
      );
      return false;
    }
    const char* dec_name = BEBOP_STR(sema->ctx, dec->name);
    if (dec_name && _bebop_streq(dec_name, "flags")) {
      return true;
    }
  }

  if (enum_def->enum_def.member_count > 0 && !enum_def->enum_def.members) {
    _bebop_context_set_error(
        sema->ctx, BEBOP_ERR_OUT_OF_MEMORY, "Enum members array is NULL despite non-zero count"
    );
    return false;
  }
  for (uint32_t i = 0; i < enum_def->enum_def.member_count; i++) {
    const bebop_enum_member_t* existing = &enum_def->enum_def.members[i];
    if (existing != member && existing->value == member->value) {
      const char* existing_name = BEBOP_STR(sema->ctx, existing->name);
      BEBOP_ERROR_FMT(
          sema->schema,
          BEBOP_DIAG_DUPLICATE_ENUM_VALUE,
          member->span,
          "Enum member '%s' has duplicate value %lld (use @flags " "for bit flags)",
          member_name ? member_name : "",
          (long long)member->value
      );
      char label_msg[128];
      snprintf(
          label_msg, sizeof(label_msg), "Same value used by '%s'", existing_name ? existing_name : ""
      );
      BEBOP_DIAG_ADD_LABEL(sema->schema, existing->span, label_msg);
      return false;
    }
  }

  return true;
}

bool bebop_sema_check_enum_complete(const bebop_sema_t* sema, const bebop_def_t* enum_def)
{
  BEBOP_ASSERT(sema != NULL);
  BEBOP_ASSERT(enum_def != NULL);
  BEBOP_ASSERT(enum_def->kind == BEBOP_DEF_ENUM);

  for (uint32_t i = 0; i < enum_def->enum_def.member_count; i++) {
    if (enum_def->enum_def.members[i].value == 0) {
      return true;
    }
  }

  if (enum_def->enum_def.member_count > 0) {
    BEBOP_ERROR_FMT(
        sema->schema,
        BEBOP_DIAG_ENUM_MISSING_ZERO_VALUE,
        enum_def->span,
        "Enum '%s' must have a member with value 0 (e.g., %s_UNSPECIFIED = 0)",
        BEBOP_STR(sema->ctx, enum_def->name),
        BEBOP_STR(sema->ctx, enum_def->name)
    );
    return false;
  }

  return true;
}

bool bebop_sema_check_duplicate_name(
    bebop_sema_t* sema, const bebop_str_t name, const bebop_span_t span
)
{
  BEBOP_ASSERT(sema != NULL);

  if (bebop_str_is_null(name)) {
    return true;
  }

  const bebop_defmap_CIter it = bebop_defmap_cfind(&sema->name_scope, &name.idx);
  const bebop_defmap_Entry* existing = bebop_defmap_CIter_get(&it);
  if (existing != NULL) {
    const char* name_str = BEBOP_STR(sema->ctx, name);
    const bebop_span_t* orig_span = (bebop_span_t*)existing->val;
    BEBOP_ERROR_FMT(
        sema->schema,
        BEBOP_DIAG_DUPLICATE_FIELD,
        span,
        "Duplicate name '%s'",
        name_str ? name_str : ""
    );
    if (orig_span) {
      BEBOP_DIAG_ADD_LABEL(sema->schema, *orig_span, "First defined here");
    }
    return false;
  }

  bebop_span_t* stored_span = bebop_arena_new(BEBOP_ARENA(sema->ctx), bebop_span_t, 1);
  if (stored_span) {
    *stored_span = span;
  }

  const bebop_defmap_Entry entry = {name.idx, stored_span};
  const bebop_defmap_Insert insert_result = bebop_defmap_insert(&sema->name_scope, &entry);
  if (bebop_defmap_Iter_get(&insert_result.iter) == NULL) {
    _bebop_context_set_error(
        sema->ctx, BEBOP_ERR_OUT_OF_MEMORY, "Failed to register name in scope"
    );
    return false;
  }

  return true;
}

bool bebop_sema_check_field_index(bebop_sema_t* sema, const uint32_t index, const bebop_span_t span)
{
  BEBOP_ASSERT(sema != NULL);

  if (index == 0) {
    BEBOP_ERROR_FMT(
        sema->schema, BEBOP_DIAG_INVALID_FIELD_INDEX, span, "Field index must be at least 1"
    );
    return false;
  }

  if (index > BEBOP_MAX_FIELD_INDEX) {
    BEBOP_ERROR_FMT(
        sema->schema,
        BEBOP_DIAG_INVALID_FIELD_INDEX,
        span,
        "Field index %u exceeds maximum of %u",
        index,
        (uint32_t)BEBOP_MAX_FIELD_INDEX
    );
    return false;
  }

  if (sema->seen_indices[index]) {
    BEBOP_ERROR_FMT(
        sema->schema, BEBOP_DIAG_DUPLICATE_FIELD_INDEX, span, "Duplicate field index %u", index
    );
    BEBOP_DIAG_ADD_LABEL(sema->schema, sema->seen_index_spans[index], "First used here");
    return false;
  }

  sema->seen_indices[index] = 1;
  sema->seen_index_spans[index] = span;
  return true;
}

bool bebop_sema_check_map_key_type(
    const bebop_sema_t* sema, bebop_type_t* key_type, const bebop_span_t span
)
{
  BEBOP_ASSERT(sema != NULL);

  if (!key_type) {
    BEBOP_ERROR_FMT(sema->schema, BEBOP_DIAG_INVALID_MAP_KEY_TYPE, span, "Map key type is missing");
    return false;
  }

  for (size_t i = 0; i < _bebop_SEMA_VALID_MAP_KEY_COUNT; i++) {
    if (key_type->kind == _bebop_sema_valid_map_keys[i]) {
      return true;
    }
  }

  const char* type_name = bebop_type_kind_name(key_type->kind);
  BEBOP_ERROR_HINT_FMT(
      sema->schema,
      BEBOP_DIAG_INVALID_MAP_KEY_TYPE,
      span,
      "valid key types: bool, byte, int8..uint64, int128, " "uint128, string, uuid",
      "Invalid map key type '%s'",
      type_name ? type_name : "unknown"
  );
  return false;
}

bool bebop_sema_check_self_reference(
    const bebop_sema_t* sema, bebop_type_t* type, const bebop_span_t span
)
{
  BEBOP_ASSERT(sema != NULL);

  if (!type || !sema->current_def) {
    return true;
  }

  if (type->kind != BEBOP_TYPE_DEFINED) {
    return true;
  }

  if (bebop_str_eq(type->defined.name, sema->current_def->name)) {
    if (sema->current_def->kind == BEBOP_DEF_STRUCT) {
      const char* name = BEBOP_STR(sema->ctx, sema->current_def->name);
      BEBOP_ERROR_FMT(
          sema->schema,
          BEBOP_DIAG_CYCLIC_DEFINITIONS,
          span,
          "Struct '%s' cannot contain itself directly",
          name ? name : ""
      );
      return false;
    }
  }

  return true;
}

bool bebop_sema_check_service_type(
    const bebop_sema_t* sema, bebop_type_t* type, const bebop_span_t span
)
{
  BEBOP_ASSERT(sema != NULL);

  if (!type) {
    BEBOP_ERROR_FMT(
        sema->schema, BEBOP_DIAG_INVALID_SERVICE_TYPE, span, "Service method type is missing"
    );
    return false;
  }

  if (type->kind != BEBOP_TYPE_DEFINED) {
    const char* type_name = bebop_type_kind_name(type->kind);
    BEBOP_ERROR_FMT(
        sema->schema,
        BEBOP_DIAG_INVALID_SERVICE_TYPE,
        span,
        "Service method types must be user-defined types (struct, " "message, or union), not '%s'",
        type_name ? type_name : "unknown"
    );
    return false;
  }

  return true;
}
