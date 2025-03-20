use std::collections::HashMap;

use replace_with::replace_with;

pub const NIL: &str = "nil";
pub const ANY: &str = "any";
pub const BOOLEAN: &str = "boolean";
pub const STRING: &str = "string";
pub const NUMBER: &str = "number";
pub const INTEGER: &str = "integer";
pub const TABLE: &str = "table";
pub const THREAD: &str = "thread";
pub const USERDATA: &str = "userdata";
pub const LIGHT_USERDATA: &str = "lightuserdata";

pub mod literals {
    pub const TRUE: &str = "true";
    pub const FALSE: &str = "false";
}

#[derive(Debug, Clone, PartialEq)]
pub struct Type {
    pub inner: TypeInner,
    pub generics: Vec<Type>,
    pub nullable: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum Metatype {
    Class,
    Alias,
    Enum,
}

impl Type {
    pub const NIL: Self = Type {
        inner: TypeInner::Nil,
        generics: Vec::new(),
        nullable: false,
    };
    pub const ANY: Self = Type {
        inner: TypeInner::Any,
        generics: Vec::new(),
        nullable: false,
    };
    pub const BOOLEAN: Self = Type {
        inner: TypeInner::Boolean,
        generics: Vec::new(),
        nullable: false,
    };
    pub const STRING: Self = Type {
        inner: TypeInner::String,
        generics: Vec::new(),
        nullable: false,
    };
    pub const NUMBER: Self = Type {
        inner: TypeInner::Number,
        generics: Vec::new(),
        nullable: false,
    };
    pub const INTEGER: Self = Type {
        inner: TypeInner::Integer,
        generics: Vec::new(),
        nullable: false,
    };
    pub const TABLE: Self = Type {
        inner: TypeInner::Table,
        generics: Vec::new(),
        nullable: false,
    };
    pub const THREAD: Self = Type {
        inner: TypeInner::Thread,
        generics: Vec::new(),
        nullable: false,
    };
    pub const USERDATA: Self = Type {
        inner: TypeInner::Userdata,
        generics: Vec::new(),
        nullable: false,
    };
    pub const LIGHT_USERDATA: Self = Type {
        inner: TypeInner::LightUserdata,
        generics: Vec::new(),
        nullable: false,
    };

    pub fn make_array(&mut self) {
        replace_with(
            self,
            || Type::NIL,
            |ty| Type {
                inner: TypeInner::Array(Box::new(ty)),
                generics: Vec::new(),
                nullable: false,
            },
        )
    }

    pub fn make_nullable(&mut self) {
        self.nullable = true;
    }

    pub fn union(types: impl IntoIterator<Item = Type>) -> Self {
        Self {
            inner: TypeInner::Union(types.into_iter().collect()),
            generics: Vec::new(),
            nullable: false,
        }
    }

    pub fn string_literal(string: impl ToString) -> Self {
        Self {
            inner: TypeInner::Literal(Literal::String(string.to_string())),
            generics: Vec::new(),
            nullable: false,
        }
    }

    pub fn integer_literal(integer: i64) -> Self {
        Self {
            inner: TypeInner::Literal(Literal::Integer(integer)),
            generics: Vec::new(),
            nullable: false,
        }
    }

    pub fn boolean_literal(boolean: bool) -> Self {
        Self {
            inner: TypeInner::Literal(Literal::Boolean(boolean)),
            generics: Vec::new(),
            nullable: false,
        }
    }

    pub fn user_defined(ty: impl ToString) -> Self {
        Self {
            inner: TypeInner::UserDefined(ty.to_string()),
            generics: Vec::new(),
            nullable: false,
        }
    }

    pub fn function(args: Vec<(String, Type)>, returns: Vec<(Option<String>, Type)>) -> Self {
        Self {
            inner: TypeInner::Function { args, ret: returns },
            generics: Vec::new(),
            nullable: false,
        }
    }

    pub fn table(fields: Vec<(Type, Type)>) -> Self {
        Self {
            inner: TypeInner::TableDef(TableDef { fields }),
            generics: Vec::new(),
            nullable: false,
        }
    }

    pub fn tuple(types: impl IntoIterator<Item = Type>) -> Self {
        Self {
            inner: TypeInner::Tuple(types.into_iter().collect()),
            generics: Vec::new(),
            nullable: false,
        }
    }

    pub fn add_generic(&mut self, generic: Type) {
        self.generics.push(generic);
    }

    pub fn format_as_table_field_name(&self) -> String {
        if !self.generics.is_empty() {
            format!("[{self}]")
        } else {
            match &self.inner {
                TypeInner::Nil
                | TypeInner::Any
                | TypeInner::Boolean
                | TypeInner::String
                | TypeInner::Number
                | TypeInner::Integer
                | TypeInner::Table
                | TypeInner::Literal(Literal::Boolean(_))
                | TypeInner::Literal(Literal::Number(_))
                | TypeInner::Literal(Literal::Integer(_))
                | TypeInner::Function { .. }
                | TypeInner::Thread
                | TypeInner::Userdata
                | TypeInner::LightUserdata
                | TypeInner::Union(_)
                | TypeInner::Array(_)
                | TypeInner::Tuple(_)
                | TypeInner::TableDef(_) => format!("[{self}]"),
                TypeInner::UserDefined(_) | TypeInner::Literal(Literal::String(_)) => {
                    self.to_string()
                }
            }
        }
    }

    pub fn format_with_links(
        &self,
        ident_lookup: &HashMap<String, Metatype>,
        base_url: &str,
    ) -> String {
        let repr = match &self.inner {
            TypeInner::Nil => "nil".into(),
            TypeInner::Any => "any".into(),
            TypeInner::Boolean => "boolean".into(),
            TypeInner::String => "string".into(),
            TypeInner::Number => "number".into(),
            TypeInner::Integer => "integer".into(),
            TypeInner::Table => "table".into(),
            TypeInner::Literal(lit) => match lit {
                Literal::Boolean(boolean) => boolean.to_string(),
                Literal::String(string) => string.clone(),
                Literal::Number(number) => number.to_string(),
                Literal::Integer(integer) => integer.to_string(),
            },
            TypeInner::Function { args, ret } => {
                let args = args
                    .iter()
                    .map(|(name, ty)| {
                        let nullable = ty.nullable.then_some("?").unwrap_or_default();
                        format!(
                            "{name}{nullable}: {}",
                            ty.format_with_links(ident_lookup, base_url)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");

                let mut returns = ret
                    .iter()
                    .map(|(name, ty)| {
                        let nullable = ty.nullable.then_some("?").unwrap_or_default();
                        format!(
                            "{}{}{nullable}",
                            name.as_ref()
                                .map(|name| format!("{name}: "))
                                .unwrap_or_default(),
                            ty.format_with_links(ident_lookup, base_url)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");

                if !returns.is_empty() {
                    returns = format!(": {returns}");
                }

                format!("fun({args}){returns}")
            }
            TypeInner::Thread => "thread".into(),
            TypeInner::Userdata => "userdata".into(),
            TypeInner::LightUserdata => "lightuserdata".into(),
            TypeInner::Union(union) => union
                .iter()
                .map(|ty| ty.format_with_links(ident_lookup, base_url))
                .collect::<Vec<_>>()
                .join(" | "),
            TypeInner::Array(ty) => {
                format!("{}[]", ty.format_with_links(ident_lookup, base_url))
            }
            TypeInner::Tuple(tuple) => {
                let tys = tuple
                    .iter()
                    .map(|ty| ty.format_with_links(ident_lookup, base_url))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{tys}]")
            }
            TypeInner::TableDef(table) => {
                let fields = table
                    .fields
                    .iter()
                    .map(|(name, ty)| {
                        let nullable = ty.nullable.then_some("?").unwrap_or_default();

                        // TODO: add links to name
                        format!(
                            "{}{nullable}: {}",
                            name.format_as_table_field_name(),
                            ty.format_with_links(ident_lookup, base_url)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");

                format!("{{ {fields} }}")
            }
            TypeInner::UserDefined(name) => {
                if let Some(metatype) = ident_lookup.get(name) {
                    let path = match metatype {
                        // TODO: support arbitrary (nested) sections
                        Metatype::Class => "classes",
                        Metatype::Alias => "aliases",
                        Metatype::Enum => "enums",
                    };
                    // ???????? VitePress throws an element has missing tag error if the character
                    // directly after a tag is an underscore
                    let sanitized_name = if name.chars().next().is_some_and(|ch| ch == '_') {
                        let mut clone = name.clone();
                        clone.replace_range(0..1, "&#95;");
                        clone
                    } else {
                        name.clone()
                    };
                    format!(r#"<a href="{base_url}{path}/{name}">{sanitized_name}</a>"#)
                } else {
                    name.clone()
                }
            }
        };

        let mut generics = self
            .generics
            .iter()
            .map(|ty| ty.format_with_links(ident_lookup, base_url))
            .collect::<Vec<_>>()
            .join(", ");

        if !generics.is_empty() {
            generics = format!("&lt;{generics}>");
        }

        format!("{repr}{generics}")
    }

    pub fn is_user_defined(&self) -> bool {
        matches!(&self.inner, TypeInner::UserDefined(_))
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = match &self.inner {
            TypeInner::Nil => "nil".into(),
            TypeInner::Any => "any".into(),
            TypeInner::Boolean => "boolean".into(),
            TypeInner::String => "string".into(),
            TypeInner::Number => "number".into(),
            TypeInner::Integer => "integer".into(),
            TypeInner::Table => "table".into(),
            TypeInner::Literal(lit) => match lit {
                Literal::Boolean(boolean) => boolean.to_string(),
                Literal::String(string) => string.clone(),
                Literal::Number(number) => number.to_string(),
                Literal::Integer(integer) => integer.to_string(),
            },
            TypeInner::Function { args, ret } => {
                let args = args
                    .iter()
                    .map(|(name, ty)| format!("{name}: {ty}"))
                    .collect::<Vec<_>>()
                    .join(", ");

                let mut returns = ret
                    .iter()
                    .map(|(name, ty)| {
                        format!(
                            "{}{ty}",
                            name.as_ref()
                                .map(|name| format!("{name}: "))
                                .unwrap_or_default()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");

                if !returns.is_empty() {
                    returns = format!(": {returns}");
                }

                format!("fun({args}){returns}")
            }
            TypeInner::Thread => "thread".into(),
            TypeInner::Userdata => "userdata".into(),
            TypeInner::LightUserdata => "lightuserdata".into(),
            TypeInner::Union(union) => union
                .iter()
                .map(|ty| ty.to_string())
                .collect::<Vec<_>>()
                .join(" | "),
            TypeInner::Array(ty) => {
                format!("{ty}[]")
            }
            TypeInner::Tuple(tuple) => {
                let tys = tuple
                    .iter()
                    .map(|ty| ty.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{tys}]")
            }
            TypeInner::TableDef(table) => {
                let fields = table
                    .fields
                    .iter()
                    .map(|(name, ty)| {
                        // WARN: might be cyclic
                        format!("{}: {ty}", name.format_as_table_field_name())
                    })
                    .collect::<Vec<_>>()
                    .join(", ");

                format!("{{ {fields} }}")
            }
            TypeInner::UserDefined(name) => name.clone(),
        };

        let mut generics = self
            .generics
            .iter()
            .map(|ty| ty.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        if !generics.is_empty() {
            generics = format!("<{generics}>");
        }

        write!(f, "{repr}{generics}")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeInner {
    Nil,
    Any,
    Boolean,
    String,
    Number,
    Integer,
    Table,
    Literal(Literal),
    Function {
        args: Vec<(String, Type)>,
        ret: Vec<(Option<String>, Type)>,
    },
    Thread,
    Userdata,
    LightUserdata,
    Union(Vec<Type>),
    Array(Box<Type>),
    Tuple(Vec<Type>),
    TableDef(TableDef),
    UserDefined(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Boolean(bool),
    String(String),
    Number(f64),
    Integer(i64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableDef {
    pub fields: Vec<(Type, Type)>,
}
