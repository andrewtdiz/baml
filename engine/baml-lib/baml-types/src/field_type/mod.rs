use crate::BamlMediaType;
use crate::Constraint;
use indexmap::IndexSet;
use itertools::Itertools;

mod builder;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, Eq, Hash)]
pub enum TypeValue {
    String,
    Int,
    Float,
    Bool,
    // Char,
    Null,
    Media(BamlMediaType),
}

impl std::str::FromStr for TypeValue {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "string" => TypeValue::String,
            "int" => TypeValue::Int,
            "float" => TypeValue::Float,
            "bool" => TypeValue::Bool,
            "null" => TypeValue::Null,
            "image" => TypeValue::Media(BamlMediaType::Image),
            "audio" => TypeValue::Media(BamlMediaType::Audio),
            _ => return Err(()),
        })
    }
}

impl std::fmt::Display for TypeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeValue::String => write!(f, "string"),
            TypeValue::Int => write!(f, "int"),
            TypeValue::Float => write!(f, "float"),
            TypeValue::Bool => write!(f, "bool"),
            TypeValue::Null => write!(f, "null"),
            TypeValue::Media(BamlMediaType::Image) => write!(f, "image"),
            TypeValue::Media(BamlMediaType::Audio) => write!(f, "audio"),
        }
    }
}

/// Subset of [`crate::BamlValue`] allowed for literal type definitions.
#[derive(serde::Serialize, Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum LiteralValue {
    String(String),
    Int(i64),
    Bool(bool),
}

impl LiteralValue {
    pub fn literal_base_type(&self) -> FieldType {
        match self {
            Self::String(_) => FieldType::string(),
            Self::Int(_) => FieldType::int(),
            Self::Bool(_) => FieldType::bool(),
        }
    }
}

impl std::fmt::Display for LiteralValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LiteralValue::String(str) => write!(f, "\"{str}\""),
            LiteralValue::Int(int) => write!(f, "{int}"),
            LiteralValue::Bool(bool) => write!(f, "{bool}"),
        }
    }
}

/// FieldType represents the type of either a class field or a function arg.
#[derive(serde::Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum FieldTypeWithMetadata<T> {
    Primitive(TypeValue, T),
    Enum(String, T),
    Literal(LiteralValue, T),
    Class(String, T),
    List(Box<FieldTypeWithMetadata<T>>, T),
    Map(
        Box<FieldTypeWithMetadata<T>>,
        Box<FieldTypeWithMetadata<T>>,
        T,
    ),
    Union(Vec<FieldTypeWithMetadata<T>>, T),
    Tuple(Vec<FieldTypeWithMetadata<T>>, T),
    Optional(Box<FieldTypeWithMetadata<T>>, T),
    RecursiveTypeAlias(String, T),
    Arrow(Box<Arrow>, T),
}

impl<T> FieldTypeWithMetadata<T> {
    fn meta(&self) -> &T {
        match self {
            Self::Primitive(_, meta) => meta,
            Self::Enum(_, meta) => meta,
            Self::Literal(_, meta) => meta,
            Self::Class(_, meta) => meta,
            Self::List(_, meta) => meta,
            Self::Map(_, _, meta) => meta,
            Self::Union(_, meta) => meta,
            Self::Tuple(_, meta) => meta,
            Self::Optional(_, meta) => meta,
            Self::RecursiveTypeAlias(_, meta) => meta,
            Self::Arrow(_, meta) => meta,
        }
    }
}

/// The metadata typically associated with a field type.
#[derive(serde::Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct FieldTypeMetadata {
    constraints: Vec<Constraint>,
    streaming_behavior: StreamingBehavior,
}

impl Default for FieldTypeMetadata {
    fn default() -> Self {
        FieldTypeMetadata {
            constraints: vec![],
            streaming_behavior: StreamingBehavior::default(),
        }
    }
}

impl FieldTypeMetadata {
    pub fn is_null(&self) -> bool {
        self.constraints.is_empty() && self.streaming_behavior == StreamingBehavior::default()
    }
}

impl std::fmt::Display for FieldTypeMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.constraints.is_empty() {
            for constraint in &self.constraints {
                write!(f, "{}, ", format!("{:?}", constraint))?;
            }
        }
        write!(f, "{}", format!("{:?}", self.streaming_behavior))
    }
}

type FieldType = FieldTypeWithMetadata<FieldTypeMetadata>;

pub trait HasFieldType {
    fn field_type<'a>(&'a self) -> &'a FieldType;
}

#[derive(serde::Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Arrow {
    pub param_types: Vec<FieldType>,
    pub return_type: FieldType,
}

// Impl display for FieldType
impl std::fmt::Display for FieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldType::Enum(name, _meta)
            | FieldType::Class(name, _meta)
            | FieldType::RecursiveTypeAlias(name, _meta) => write!(f, "{name}"),
            FieldType::Primitive(t, _meta) => write!(f, "{t}"),
            FieldType::Literal(v, _meta) => write!(f, "{v}"),
            FieldType::Union(choices, _meta) => {
                write!(
                    f,
                    "({})",
                    choices
                        .iter()
                        .map(|t| t.to_string())
                        .collect::<Vec<_>>()
                        .join(" | ")
                )
            }
            FieldType::Tuple(choices, _meta) => {
                write!(
                    f,
                    "({})",
                    choices
                        .iter()
                        .map(|t| t.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            FieldType::Map(k, v, _meta) => write!(f, "map<{k}, {v}>"),
            FieldType::List(t, _meta) => write!(f, "{t}[]"),
            FieldType::Optional(t, _meta) => write!(f, "{t}?"),
            FieldType::Arrow(arrow, _meta) => write!(
                f,
                "({}) -> {}",
                arrow
                    .param_types
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
                arrow.return_type.to_string()
            ),
        }?;
        if !self.meta().is_null() {
            write!(f, " ({})", self.meta())?;
        }
        Ok(())
    }
}

impl FieldType {
    fn flatten(&self) -> Vec<FieldType> {
        match self {
            FieldType::Union(inner, _) => inner.iter().flat_map(|t| t.flatten()).collect(),
            FieldType::Optional(inner._meta) => {
                let mut values = inner.flatten();
                values.push(FieldType::Primitive(TypeValue::Null));
                values
            }
            _ => vec![self.clone()],
        }
    }

    pub fn simplify(&self) -> FieldType {
        match self {
            FieldType::Union(inner) => {
                let flattened = inner.iter().flat_map(|t| t.flatten()).collect::<Vec<_>>();
                let unique = flattened.into_iter().unique().collect::<Vec<_>>();
                let has_null = unique.contains(&FieldType::Primitive(TypeValue::Null));
                // if the union contains null, we'll detect that here.
                let unique_without_null = unique
                    .into_iter()
                    .filter(|t| t != &FieldType::Primitive(TypeValue::Null))
                    .collect::<Vec<_>>();

                let simplified = match unique_without_null.len() {
                    0 => return FieldType::Primitive(TypeValue::Null),
                    1 => unique_without_null[0].clone(),
                    _ => FieldType::Union(unique_without_null),
                };

                if has_null {
                    FieldType::Optional(Box::new(simplified))
                } else {
                    simplified
                }
            }
            _ => self.clone(),
        }
    }

    pub fn is_primitive(&self) -> bool {
        match self {
            FieldType::Primitive(_) => true,
            FieldType::Optional(t) => t.is_primitive(),
            FieldType::List(t) => t.is_primitive(),
            FieldType::WithMetadata { base, .. } => base.is_primitive(),
            _ => false,
        }
    }

    pub fn is_optional(&self) -> bool {
        match self {
            FieldType::Optional(_) => true,
            FieldType::Primitive(TypeValue::Null) => true,
            FieldType::Union(types) => types.iter().any(FieldType::is_optional),
            FieldType::WithMetadata { base, .. } => base.is_optional(),
            _ => false,
        }
    }

    pub fn is_null(&self) -> bool {
        match self {
            FieldType::Primitive(TypeValue::Null) => true,
            FieldType::Optional(t) => t.is_null(),
            FieldType::WithMetadata { base, .. } => base.is_null(),
            _ => false,
        }
    }

    pub fn streaming_behavior(&self) -> Option<&StreamingBehavior> {
        match self {
            FieldType::WithMetadata {
                streaming_behavior, ..
            } => Some(streaming_behavior),
            _ => None,
        }
    }
}

pub trait ToUnionName {
    fn to_union_name(&self) -> String;
    fn find_union_types(&self) -> IndexSet<FieldType>;
}

impl ToUnionName for FieldType {
    fn find_union_types(&self) -> IndexSet<FieldType> {
        // TODO: its pretty hard to get type aliases here
        let value = self.simplify();
        match &value {
            FieldType::Union(_) => IndexSet::from_iter([value]),
            FieldType::List(inner) => inner.find_union_types(),
            FieldType::Map(field_type, field_type1) => {
                let mut set = field_type.find_union_types();
                set.extend(field_type1.find_union_types());
                set
            }
            FieldType::Primitive(_)
            | FieldType::Enum(_)
            | FieldType::Literal(_)
            | FieldType::Class(_)
            | FieldType::RecursiveTypeAlias(_)
            | FieldType::Arrow(_) => IndexSet::new(),
            FieldType::Tuple(inner) => inner.iter().flat_map(|t| t.find_union_types()).collect(),
            FieldType::Optional(inner) => inner.find_union_types(),
            FieldType::WithMetadata { base, .. } => base.find_union_types(),
        }
    }

    fn to_union_name(&self) -> String {
        match self {
            FieldType::Primitive(type_value) => type_value.to_string(),
            FieldType::Enum(name) => name.to_string(),
            FieldType::Literal(literal_value) => match literal_value {
                LiteralValue::String(value) => format!(
                    "string_{}",
                    value
                        .chars()
                        .map(|c| if c.is_alphanumeric() { c } else { '_' })
                        .collect::<String>()
                ),
                LiteralValue::Int(val) => format!("int_{}", val.to_string()),
                LiteralValue::Bool(val) => format!("bool_{}", val.to_string()),
            },
            FieldType::Class(name) => name.to_string(),
            FieldType::List(field_type) => {
                format!("List__{}", field_type.to_union_name())
            }
            FieldType::Map(field_type, field_type1) => {
                format!(
                    "Map__{}_{}",
                    field_type.to_union_name(),
                    field_type1.to_union_name()
                )
            }
            FieldType::Union(field_types) => format!(
                "Union__{}",
                field_types
                    .iter()
                    .map(|v| v.to_union_name())
                    .collect::<Vec<_>>()
                    .join("__")
                    .to_string()
            ),
            FieldType::Tuple(field_types) => format!(
                "Tuple__{}",
                field_types
                    .iter()
                    .map(|v| v.to_union_name())
                    .collect::<Vec<_>>()
                    .join("__")
                    .to_string()
            ),
            FieldType::Optional(field_type) => {
                format!("Optional__{}", field_type.to_union_name())
            }
            FieldType::RecursiveTypeAlias(name) => name.to_string(),
            FieldType::WithMetadata { base, .. } => base.to_union_name(),
            FieldType::Arrow(_) => "function".to_string(),
        }
    }
}

/// Metadata on a type that determines how it behaves under streaming conditions.
#[derive(Clone, Debug, PartialEq, serde::Serialize, Eq, Hash)]
pub struct StreamingBehavior {
    /// A type with the `done` property will not be visible in a stream until
    /// we are certain that it is completely available (i.e. the parser did
    /// not finalize it through any early termination, enough tokens were available
    /// from the LLM response to be certain that it is done).
    pub done: bool,

    /// A type with the `state` property will be represented in client code as
    /// a struct: `{value: T, streaming_state: "incomplete" | "complete"}`.
    pub state: bool,
}

impl StreamingBehavior {
    pub fn combine(&self, other: &StreamingBehavior) -> StreamingBehavior {
        StreamingBehavior {
            done: self.done || other.done,
            state: self.state || other.state,
        }
    }
}

impl Default for StreamingBehavior {
    fn default() -> Self {
        StreamingBehavior {
            done: false,
            state: false,
        }
    }
}

#[cfg(test)]
mod tests {}
