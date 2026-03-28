use std::collections::{HashMap, HashSet};

use crate::generated::{
  DefinitionDescriptor, DefinitionKind, FieldDescriptor, SchemaDescriptor, TypeDescriptor, TypeKind,
};

use super::field_codegen::is_bulk_scalar;
use super::{has_decorator, naming, FORWARD_COMPATIBLE};

/// Pre-computed lifetime and kind information for all definitions in a schema.
pub struct SchemaAnalysis {
  /// FQNs of enum definitions (never need a lifetime parameter).
  enum_fqns: HashSet<String>,
  /// FQNs of types that need `'buf` (contain strings, byte arrays, or unions).
  lifetime_fqns: HashSet<String>,
  /// FQNs of types that can derive `Eq` (no floating-point fields transitively).
  eq_fqns: HashSet<String>,
  /// FQNs of types that can derive `Hash` (no floating-point or map fields transitively).
  hash_fqns: HashSet<String>,
}

impl SchemaAnalysis {
  /// Build a combined analysis from all schemas, so cross-schema type
  /// references resolve correctly.
  pub fn build(schemas: &[SchemaDescriptor]) -> Self {
    let mut analysis = SchemaAnalysis {
      enum_fqns: HashSet::new(),
      lifetime_fqns: HashSet::new(),
      eq_fqns: HashSet::new(),
      hash_fqns: HashSet::new(),
    };

    let mut def_by_fqn: HashMap<String, &DefinitionDescriptor> = HashMap::new();
    for schema in schemas {
      let definitions = schema.definitions.as_deref().unwrap_or(&[]);
      collect_definitions(definitions, &mut def_by_fqn);
    }

    for (fqn, def) in &def_by_fqn {
      if def.kind == Some(DefinitionKind::Enum) {
        analysis.enum_fqns.insert(fqn.clone());
      }
      if def.kind == Some(DefinitionKind::Union) && has_decorator(def, FORWARD_COMPATIBLE) {
        // Forward-compatible unions always need lifetime (Unknown variant uses
        // Cow<'buf, [u8]>). Strict unions only need lifetime if their branches
        // do, which is resolved by the fixpoint loop below.
        analysis.lifetime_fqns.insert(fqn.clone());
      }
    }

    // Resolve lifetime requirements to a fixpoint so forward references work.
    loop {
      let mut changed = false;
      for (fqn, def) in &def_by_fqn {
        if analysis.lifetime_fqns.contains(fqn) {
          continue;
        }

        let needs_lifetime = match def.kind {
          Some(DefinitionKind::Struct) => def
            .struct_def
            .as_ref()
            .and_then(|sd| sd.fields.as_deref())
            .is_some_and(|fields| {
              fields.iter().any(|f| {
                f.r#type
                  .as_ref()
                  .is_some_and(|td| analysis.type_needs_lifetime(td))
              })
            }),
          Some(DefinitionKind::Message) => def
            .message_def
            .as_ref()
            .and_then(|md| md.fields.as_deref())
            .is_some_and(|fields| {
              fields.iter().any(|f| {
                f.r#type
                  .as_ref()
                  .is_some_and(|td| analysis.type_needs_lifetime(td))
              })
            }),
          // Forward-compatible unions were already seeded above and skipped
          // by the `continue`. Only strict unions reach here; they need
          // lifetime only if any branch does.
          Some(DefinitionKind::Union) => def
            .union_def
            .as_ref()
            .and_then(|ud| ud.branches.as_deref())
            .is_some_and(|branches| {
              branches.iter().any(|b| {
                let branch_fqn = b.type_ref_fqn.as_deref().or(b.inline_fqn.as_deref());
                branch_fqn.is_some_and(|branch_fqn| analysis.lifetime_fqns.contains(branch_fqn))
              })
            }),
          _ => false,
        };

        if needs_lifetime {
          analysis.lifetime_fqns.insert(fqn.clone());
          changed = true;
        }
      }

      if !changed {
        break;
      }
    }

    analysis.analyze_trait_derives(&def_by_fqn);
    analysis
  }

  /// Whether a definition FQN needs a `'buf` lifetime parameter.
  pub fn needs_lifetime(&self, fqn: &str) -> bool {
    self.lifetime_fqns.contains(fqn)
  }

  /// Whether a TypeDescriptor transitively contains borrowed data.
  pub fn type_needs_lifetime(&self, td: &TypeDescriptor) -> bool {
    let kind = match td.kind {
      Some(kind) => kind,
      None => return false,
    };

    match kind {
      TypeKind::String => true,
      TypeKind::Array => {
        if let Some(ref elem) = td.array_element {
          // byte arrays (Cow<'buf, [u8]>) and bulk scalar arrays (Cow<'buf, [T]>)
          // need lifetime because they use zero-copy Cow wrappers.
          if elem.kind == Some(TypeKind::Byte) || elem.kind.is_some_and(is_bulk_scalar) {
            return true;
          }
          self.type_needs_lifetime(elem)
        } else {
          false
        }
      }
      TypeKind::FixedArray => td
        .fixed_array_element
        .as_ref()
        .is_some_and(|elem| self.type_needs_lifetime(elem)),
      TypeKind::Map => {
        let key = td
          .map_key
          .as_ref()
          .is_some_and(|elem| self.type_needs_lifetime(elem));
        let value = td
          .map_value
          .as_ref()
          .is_some_and(|elem| self.type_needs_lifetime(elem));
        key || value
      }
      TypeKind::Defined => td
        .defined_fqn
        .as_deref()
        .is_some_and(|fqn| self.lifetime_fqns.contains(fqn)),
      _ => false,
    }
  }

  /// Returns true when this type can derive `Eq`.
  pub fn can_derive_eq(&self, fqn: &str) -> bool {
    self.eq_fqns.contains(fqn)
  }

  /// Returns true when this type can derive `Hash`.
  pub fn can_derive_hash(&self, fqn: &str) -> bool {
    self.hash_fqns.contains(fqn)
  }

  fn analyze_trait_derives<'a>(
    &mut self,
    def_by_fqn: &HashMap<String, &'a DefinitionDescriptor<'a>>,
  ) {
    let mut deps_by_fqn: HashMap<String, (DefinitionKind, TraitDeps)> = HashMap::new();
    for (fqn, def) in def_by_fqn {
      if let Some(kind) = def.kind {
        if kind_supports_derives(kind) {
          deps_by_fqn.insert(fqn.clone(), (kind, definition_deps(def)));
        }
      }
    }

    let mut resolved: HashMap<String, TypeTraits> = deps_by_fqn
      .iter()
      .map(|(fqn, (_, deps))| (fqn.clone(), deps.direct))
      .collect();

    let mut changed = true;
    while changed {
      changed = false;
      for (fqn, (_, deps)) in &deps_by_fqn {
        let mut traits = deps.direct;
        for dep in &deps.deps {
          if let Some(dep_traits) = resolved.get(dep) {
            traits = traits.combine(*dep_traits);
          } else {
            traits = traits.combine(TypeTraits {
              has_float: true,
              has_map: true,
            });
          }
        }
        if resolved.get(fqn).copied() != Some(traits) {
          resolved.insert(fqn.clone(), traits);
          changed = true;
        }
      }
    }

    for (fqn, (kind, _)) in &deps_by_fqn {
      if *kind == DefinitionKind::Enum {
        self.eq_fqns.insert(fqn.clone());
        self.hash_fqns.insert(fqn.clone());
        continue;
      }

      let traits = resolved.get(fqn).copied().unwrap_or_default();
      if !traits.has_float {
        self.eq_fqns.insert(fqn.clone());
      }
      if !traits.has_float && !traits.has_map {
        self.hash_fqns.insert(fqn.clone());
      }
    }
  }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct TypeTraits {
  has_float: bool,
  has_map: bool,
}

impl TypeTraits {
  fn combine(self, other: Self) -> Self {
    Self {
      has_float: self.has_float || other.has_float,
      has_map: self.has_map || other.has_map,
    }
  }
}

#[derive(Clone, Default)]
struct TraitDeps {
  direct: TypeTraits,
  deps: Vec<String>,
}

impl TraitDeps {
  fn combine(&mut self, other: Self) {
    self.direct = self.direct.combine(other.direct);
    for dep in other.deps {
      self.add_dep(dep);
    }
  }

  fn add_dep(&mut self, dep: String) {
    if !self.deps.iter().any(|existing| existing == &dep) {
      self.deps.push(dep);
    }
  }
}

fn kind_supports_derives(kind: DefinitionKind) -> bool {
  matches!(
    kind,
    DefinitionKind::Enum | DefinitionKind::Struct | DefinitionKind::Message | DefinitionKind::Union
  )
}

fn type_deps(td: &TypeDescriptor) -> TraitDeps {
  let kind = match td.kind {
    Some(kind) => kind,
    None => return TraitDeps::default(),
  };

  match kind {
    TypeKind::Float16 | TypeKind::Float32 | TypeKind::Float64 | TypeKind::Bfloat16 => TraitDeps {
      direct: TypeTraits {
        has_float: true,
        has_map: false,
      },
      deps: Vec::new(),
    },
    TypeKind::Array => td
      .array_element
      .as_deref()
      .map(type_deps)
      .unwrap_or_default(),
    TypeKind::FixedArray => td
      .fixed_array_element
      .as_deref()
      .map(type_deps)
      .unwrap_or_default(),
    TypeKind::Map => {
      let mut deps = TraitDeps {
        direct: TypeTraits {
          has_float: false,
          has_map: true,
        },
        deps: Vec::new(),
      };
      if let Some(key) = td.map_key.as_deref() {
        deps.combine(type_deps(key));
      }
      if let Some(value) = td.map_value.as_deref() {
        deps.combine(type_deps(value));
      }
      deps
    }
    TypeKind::Defined => {
      let mut deps = TraitDeps::default();
      if let Some(fqn) = td.defined_fqn.as_deref() {
        deps.add_dep(fqn.to_string());
      } else {
        deps.direct = TypeTraits {
          has_float: true,
          has_map: true,
        };
      }
      deps
    }
    _ => TraitDeps::default(),
  }
}

fn field_list_deps(fields: Option<&[FieldDescriptor]>) -> TraitDeps {
  fields
    .unwrap_or(&[])
    .iter()
    .fold(TraitDeps::default(), |mut acc, field| {
      if let Some(td) = field.r#type.as_ref() {
        acc.combine(type_deps(td));
      }
      acc
    })
}

fn definition_deps(def: &DefinitionDescriptor) -> TraitDeps {
  match def.kind {
    Some(DefinitionKind::Enum) => TraitDeps::default(),
    Some(DefinitionKind::Struct) => {
      field_list_deps(def.struct_def.as_ref().and_then(|sd| sd.fields.as_deref()))
    }
    Some(DefinitionKind::Message) => {
      field_list_deps(def.message_def.as_ref().and_then(|md| md.fields.as_deref()))
    }
    Some(DefinitionKind::Union) => {
      let mut deps = TraitDeps::default();
      for branch in def
        .union_def
        .as_ref()
        .and_then(|ud| ud.branches.as_deref())
        .unwrap_or(&[])
      {
        if let Some(branch_fqn) = branch
          .type_ref_fqn
          .as_deref()
          .or(branch.inline_fqn.as_deref())
        {
          deps.add_dep(branch_fqn.to_string());
        } else if let (Some(union_fqn), Some(branch_name)) =
          (def.fqn.as_deref(), branch.name.as_deref())
        {
          deps.add_dep(format!("{}.{}", union_fqn, naming::type_name(branch_name)));
        } else {
          deps.direct = deps.direct.combine(TypeTraits {
            has_float: true,
            has_map: true,
          });
        }
      }
      deps
    }
    _ => TraitDeps::default(),
  }
}

fn collect_definitions<'a>(
  defs: &'a [DefinitionDescriptor<'a>],
  def_by_fqn: &mut HashMap<String, &'a DefinitionDescriptor<'a>>,
) {
  for def in defs {
    if let Some(fqn) = def.fqn.as_deref() {
      def_by_fqn.insert(fqn.to_string(), def);
    }
    if let Some(nested) = def.nested.as_deref() {
      collect_definitions(nested, def_by_fqn);
    }
  }
}

#[cfg(test)]
mod tests {
  use std::borrow::Cow;

  use super::SchemaAnalysis;
  use crate::generated::{
    DecoratorUsage, DefinitionDescriptor, DefinitionKind, EnumDef, EnumMemberDescriptor,
    FieldDescriptor, MessageDef, SchemaDescriptor, StructDef, TypeDescriptor, TypeKind,
    UnionBranchDescriptor, UnionDef,
  };

  fn scalar_type(kind: TypeKind) -> TypeDescriptor<'static> {
    TypeDescriptor {
      kind: Some(kind),
      ..Default::default()
    }
  }

  fn map_type(
    key: TypeDescriptor<'static>,
    value: TypeDescriptor<'static>,
  ) -> TypeDescriptor<'static> {
    TypeDescriptor {
      kind: Some(TypeKind::Map),
      map_key: Some(Box::new(key)),
      map_value: Some(Box::new(value)),
      ..Default::default()
    }
  }

  fn array_type(element: TypeDescriptor<'static>) -> TypeDescriptor<'static> {
    TypeDescriptor {
      kind: Some(TypeKind::Array),
      array_element: Some(Box::new(element)),
      ..Default::default()
    }
  }

  fn defined_type(fqn: &'static str) -> TypeDescriptor<'static> {
    TypeDescriptor {
      kind: Some(TypeKind::Defined),
      defined_fqn: Some(Cow::Borrowed(fqn)),
      ..Default::default()
    }
  }

  fn field(
    name: &'static str,
    index: u32,
    ty: TypeDescriptor<'static>,
  ) -> FieldDescriptor<'static> {
    FieldDescriptor {
      name: Some(Cow::Borrowed(name)),
      index: Some(index),
      r#type: Some(ty),
      ..Default::default()
    }
  }

  fn forward_compatible_decorator() -> Vec<DecoratorUsage<'static>> {
    vec![DecoratorUsage {
      fqn: Some(Cow::Borrowed("bebop.forward_compatible")),
      ..Default::default()
    }]
  }

  fn build_recursive_union_schema() -> SchemaDescriptor<'static> {
    let json_null = DefinitionDescriptor {
      kind: Some(DefinitionKind::Struct),
      name: Some(Cow::Borrowed("JsonNull")),
      fqn: Some(Cow::Borrowed("JsonNull")),
      struct_def: Some(StructDef {
        fields: Some(vec![]),
        fixed_size: Some(0),
        ..Default::default()
      }),
      ..Default::default()
    };

    let bool_msg = DefinitionDescriptor {
      kind: Some(DefinitionKind::Message),
      name: Some(Cow::Borrowed("Bool")),
      fqn: Some(Cow::Borrowed("JsonValue.Bool")),
      message_def: Some(MessageDef {
        fields: Some(vec![field("value", 1, scalar_type(TypeKind::Bool))]),
      }),
      ..Default::default()
    };

    let number_msg = DefinitionDescriptor {
      kind: Some(DefinitionKind::Message),
      name: Some(Cow::Borrowed("Number")),
      fqn: Some(Cow::Borrowed("JsonValue.Number")),
      message_def: Some(MessageDef {
        fields: Some(vec![field("value", 1, scalar_type(TypeKind::Float64))]),
      }),
      ..Default::default()
    };

    let list_msg = DefinitionDescriptor {
      kind: Some(DefinitionKind::Message),
      name: Some(Cow::Borrowed("List")),
      fqn: Some(Cow::Borrowed("JsonValue.List")),
      message_def: Some(MessageDef {
        fields: Some(vec![field(
          "values",
          1,
          array_type(defined_type("JsonValue")),
        )]),
      }),
      ..Default::default()
    };

    let object_msg = DefinitionDescriptor {
      kind: Some(DefinitionKind::Message),
      name: Some(Cow::Borrowed("Object")),
      fqn: Some(Cow::Borrowed("JsonValue.Object")),
      message_def: Some(MessageDef {
        fields: Some(vec![field(
          "fields",
          1,
          map_type(scalar_type(TypeKind::String), defined_type("JsonValue")),
        )]),
      }),
      ..Default::default()
    };

    let json_value = DefinitionDescriptor {
      kind: Some(DefinitionKind::Union),
      name: Some(Cow::Borrowed("JsonValue")),
      fqn: Some(Cow::Borrowed("JsonValue")),
      union_def: Some(UnionDef {
        branches: Some(vec![
          UnionBranchDescriptor {
            discriminator: Some(1),
            name: Some(Cow::Borrowed("null")),
            type_ref_fqn: Some(Cow::Borrowed("JsonNull")),
            ..Default::default()
          },
          UnionBranchDescriptor {
            discriminator: Some(2),
            name: Some(Cow::Borrowed("bool")),
            type_ref_fqn: Some(Cow::Borrowed("JsonValue.Bool")),
            ..Default::default()
          },
          UnionBranchDescriptor {
            discriminator: Some(3),
            name: Some(Cow::Borrowed("number")),
            type_ref_fqn: Some(Cow::Borrowed("JsonValue.Number")),
            ..Default::default()
          },
          UnionBranchDescriptor {
            discriminator: Some(4),
            name: Some(Cow::Borrowed("list")),
            type_ref_fqn: Some(Cow::Borrowed("JsonValue.List")),
            ..Default::default()
          },
          UnionBranchDescriptor {
            discriminator: Some(5),
            name: Some(Cow::Borrowed("object")),
            type_ref_fqn: Some(Cow::Borrowed("JsonValue.Object")),
            ..Default::default()
          },
        ]),
      }),
      nested: Some(vec![bool_msg, number_msg, list_msg, object_msg]),
      decorators: Some(forward_compatible_decorator()),
      ..Default::default()
    };

    SchemaDescriptor {
      path: Some(Cow::Borrowed("recursive-union.bop")),
      definitions: Some(vec![json_null, json_value]),
      ..Default::default()
    }
  }

  #[test]
  fn lifetime_propagates_through_forward_references() {
    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("forward.bop")),
      definitions: Some(vec![
        DefinitionDescriptor {
          kind: Some(DefinitionKind::Struct),
          name: Some(Cow::Borrowed("A")),
          fqn: Some(Cow::Borrowed("test.A")),
          struct_def: Some(StructDef {
            fields: Some(vec![field("b", 0, defined_type("test.B"))]),
            ..Default::default()
          }),
          ..Default::default()
        },
        DefinitionDescriptor {
          kind: Some(DefinitionKind::Struct),
          name: Some(Cow::Borrowed("B")),
          fqn: Some(Cow::Borrowed("test.B")),
          struct_def: Some(StructDef {
            fields: Some(vec![field("name", 0, scalar_type(TypeKind::String))]),
            ..Default::default()
          }),
          ..Default::default()
        },
      ]),
      ..Default::default()
    };

    let analysis = SchemaAnalysis::build(std::slice::from_ref(&schema));
    assert!(analysis.needs_lifetime("test.B"));
    assert!(analysis.needs_lifetime("test.A"));
  }

  #[test]
  fn forward_compatible_unions_always_need_lifetime() {
    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("fc.bop")),
      definitions: Some(vec![
        DefinitionDescriptor {
          kind: Some(DefinitionKind::Struct),
          name: Some(Cow::Borrowed("Inner")),
          fqn: Some(Cow::Borrowed("test.Inner")),
          struct_def: Some(StructDef {
            fields: Some(vec![field("value", 0, scalar_type(TypeKind::Int32))]),
            fixed_size: Some(4),
            ..Default::default()
          }),
          ..Default::default()
        },
        DefinitionDescriptor {
          kind: Some(DefinitionKind::Union),
          name: Some(Cow::Borrowed("FcUnion")),
          fqn: Some(Cow::Borrowed("test.FcUnion")),
          union_def: Some(UnionDef {
            branches: Some(vec![UnionBranchDescriptor {
              discriminator: Some(1),
              name: Some(Cow::Borrowed("inner")),
              type_ref_fqn: Some(Cow::Borrowed("test.Inner")),
              ..Default::default()
            }]),
          }),
          decorators: Some(forward_compatible_decorator()),
          ..Default::default()
        },
      ]),
      ..Default::default()
    };

    let analysis = SchemaAnalysis::build(std::slice::from_ref(&schema));
    assert!(analysis.needs_lifetime("test.FcUnion"));
  }

  #[test]
  fn strict_scalar_only_unions_do_not_need_lifetime() {
    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("strict.bop")),
      definitions: Some(vec![
        DefinitionDescriptor {
          kind: Some(DefinitionKind::Struct),
          name: Some(Cow::Borrowed("Inner")),
          fqn: Some(Cow::Borrowed("test.Inner")),
          struct_def: Some(StructDef {
            fields: Some(vec![field("value", 0, scalar_type(TypeKind::Int32))]),
            fixed_size: Some(4),
            ..Default::default()
          }),
          ..Default::default()
        },
        DefinitionDescriptor {
          kind: Some(DefinitionKind::Union),
          name: Some(Cow::Borrowed("StrictUnion")),
          fqn: Some(Cow::Borrowed("test.StrictUnion")),
          union_def: Some(UnionDef {
            branches: Some(vec![UnionBranchDescriptor {
              discriminator: Some(1),
              name: Some(Cow::Borrowed("inner")),
              type_ref_fqn: Some(Cow::Borrowed("test.Inner")),
              ..Default::default()
            }]),
          }),
          ..Default::default()
        },
      ]),
      ..Default::default()
    };

    let analysis = SchemaAnalysis::build(std::slice::from_ref(&schema));
    assert!(!analysis.needs_lifetime("test.StrictUnion"));
  }

  #[test]
  fn bulk_scalar_arrays_need_lifetime() {
    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("bulk-scalars.bop")),
      definitions: Some(vec![DefinitionDescriptor {
        kind: Some(DefinitionKind::Struct),
        name: Some(Cow::Borrowed("Numbers")),
        fqn: Some(Cow::Borrowed("test.Numbers")),
        struct_def: Some(StructDef {
          fields: Some(vec![field(
            "values",
            0,
            array_type(scalar_type(TypeKind::Int32)),
          )]),
          ..Default::default()
        }),
        ..Default::default()
      }]),
      ..Default::default()
    };

    let analysis = SchemaAnalysis::build(std::slice::from_ref(&schema));
    assert!(analysis.type_needs_lifetime(&array_type(scalar_type(TypeKind::Int32))));
    assert!(analysis.needs_lifetime("test.Numbers"));
  }

  #[test]
  fn eq_and_hash_are_blocked_by_transitive_float_and_map_fields() {
    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("traits.bop")),
      definitions: Some(vec![
        DefinitionDescriptor {
          kind: Some(DefinitionKind::Struct),
          name: Some(Cow::Borrowed("FloatLeaf")),
          fqn: Some(Cow::Borrowed("test.FloatLeaf")),
          struct_def: Some(StructDef {
            fields: Some(vec![field("value", 0, scalar_type(TypeKind::Float32))]),
            ..Default::default()
          }),
          ..Default::default()
        },
        DefinitionDescriptor {
          kind: Some(DefinitionKind::Struct),
          name: Some(Cow::Borrowed("MapLeaf")),
          fqn: Some(Cow::Borrowed("test.MapLeaf")),
          struct_def: Some(StructDef {
            fields: Some(vec![field(
              "entries",
              0,
              map_type(scalar_type(TypeKind::String), scalar_type(TypeKind::Uint32)),
            )]),
            ..Default::default()
          }),
          ..Default::default()
        },
        DefinitionDescriptor {
          kind: Some(DefinitionKind::Struct),
          name: Some(Cow::Borrowed("FloatWrapper")),
          fqn: Some(Cow::Borrowed("test.FloatWrapper")),
          struct_def: Some(StructDef {
            fields: Some(vec![field("leaf", 0, defined_type("test.FloatLeaf"))]),
            ..Default::default()
          }),
          ..Default::default()
        },
        DefinitionDescriptor {
          kind: Some(DefinitionKind::Struct),
          name: Some(Cow::Borrowed("MapWrapper")),
          fqn: Some(Cow::Borrowed("test.MapWrapper")),
          struct_def: Some(StructDef {
            fields: Some(vec![field("leaf", 0, defined_type("test.MapLeaf"))]),
            ..Default::default()
          }),
          ..Default::default()
        },
      ]),
      ..Default::default()
    };

    let analysis = SchemaAnalysis::build(std::slice::from_ref(&schema));

    assert!(!analysis.can_derive_eq("test.FloatLeaf"));
    assert!(!analysis.can_derive_hash("test.FloatLeaf"));
    assert!(!analysis.can_derive_eq("test.FloatWrapper"));
    assert!(!analysis.can_derive_hash("test.FloatWrapper"));

    assert!(analysis.can_derive_eq("test.MapLeaf"));
    assert!(!analysis.can_derive_hash("test.MapLeaf"));
    assert!(analysis.can_derive_eq("test.MapWrapper"));
    assert!(!analysis.can_derive_hash("test.MapWrapper"));
  }

  #[test]
  fn enums_always_derive_eq_and_hash() {
    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("enum.bop")),
      definitions: Some(vec![DefinitionDescriptor {
        kind: Some(DefinitionKind::Enum),
        name: Some(Cow::Borrowed("Status")),
        fqn: Some(Cow::Borrowed("test.Status")),
        enum_def: Some(EnumDef {
          base_type: Some(TypeKind::Uint32),
          members: Some(vec![EnumMemberDescriptor {
            name: Some(Cow::Borrowed("Ok")),
            value: Some(1),
            ..Default::default()
          }]),
          is_flags: Some(false),
        }),
        ..Default::default()
      }]),
      ..Default::default()
    };

    let analysis = SchemaAnalysis::build(std::slice::from_ref(&schema));
    assert!(analysis.can_derive_eq("test.Status"));
    assert!(analysis.can_derive_hash("test.Status"));
  }

  #[test]
  fn recursive_union_float_propagates_into_derive_analysis() {
    let schema = build_recursive_union_schema();
    let analysis = SchemaAnalysis::build(std::slice::from_ref(&schema));

    assert!(!analysis.can_derive_eq("JsonValue.Number"));
    assert!(!analysis.can_derive_hash("JsonValue.Number"));
    assert!(!analysis.can_derive_eq("JsonValue"));
    assert!(!analysis.can_derive_hash("JsonValue"));
    assert!(!analysis.can_derive_eq("JsonValue.Object"));
    assert!(!analysis.can_derive_hash("JsonValue.Object"));
  }

  #[test]
  fn cross_schema_references_resolve_for_lifetimes() {
    let left = SchemaDescriptor {
      path: Some(Cow::Borrowed("left.bop")),
      definitions: Some(vec![DefinitionDescriptor {
        kind: Some(DefinitionKind::Struct),
        name: Some(Cow::Borrowed("A")),
        fqn: Some(Cow::Borrowed("left.A")),
        struct_def: Some(StructDef {
          fields: Some(vec![field("b", 0, defined_type("right.B"))]),
          ..Default::default()
        }),
        ..Default::default()
      }]),
      ..Default::default()
    };

    let right = SchemaDescriptor {
      path: Some(Cow::Borrowed("right.bop")),
      definitions: Some(vec![DefinitionDescriptor {
        kind: Some(DefinitionKind::Struct),
        name: Some(Cow::Borrowed("B")),
        fqn: Some(Cow::Borrowed("right.B")),
        struct_def: Some(StructDef {
          fields: Some(vec![field("name", 0, scalar_type(TypeKind::String))]),
          ..Default::default()
        }),
        ..Default::default()
      }]),
      ..Default::default()
    };

    let analysis = SchemaAnalysis::build(&[left, right]);
    assert!(analysis.needs_lifetime("right.B"));
    assert!(analysis.needs_lifetime("left.A"));
  }
}
