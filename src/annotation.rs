use pest::{iterators::Pair, Parser};

use crate::{
    treesitter::FieldName,
    types::{self, Type},
};

#[derive(pest_derive::Parser)]
#[grammar = "parser.pest"]
pub struct PestParser;

pub fn parse_type(type_pair: Pair<Rule>) -> Type {
    assert_eq!(
        type_pair.as_rule(),
        Rule::ty,
        "called `parse_type` on a non-ty pair"
    );

    let mut types = Vec::new();
    let mut nullable = false;
    let is_union = type_pair
        .clone()
        .into_inner()
        .filter(|pair| pair.as_rule() == Rule::single_type)
        .count()
        > 1;

    for pair in type_pair.into_inner() {
        if pair.as_rule() == Rule::nullable {
            nullable = true;
            continue;
        }

        assert_eq!(pair.as_rule(), Rule::single_type);
        let mut ty = None;
        for pair in pair.into_inner() {
            match pair.as_rule() {
                Rule::function_def => ty = Some(parse_function(pair)),
                Rule::table_def => ty = Some(parse_table(pair)),
                Rule::tuple_def => ty = Some(parse_tuple(pair)),
                Rule::str_lit => ty = Some(Type::string_literal(pair.as_str())),
                Rule::int_lit => {
                    ty = Some(Type::integer_literal(pair.as_str().parse().unwrap()));
                }
                Rule::type_ident => {
                    ty = Some(match pair.as_str() {
                        types::NIL => Type::NIL,
                        types::ANY => Type::ANY,
                        types::BOOLEAN => Type::BOOLEAN,
                        types::STRING => Type::STRING,
                        types::NUMBER => Type::NUMBER,
                        types::INTEGER => Type::INTEGER,
                        types::TABLE => Type::TABLE,
                        types::THREAD => Type::THREAD,
                        types::USERDATA => Type::USERDATA,
                        types::LIGHT_USERDATA => Type::LIGHT_USERDATA,
                        types::literals::TRUE => Type::boolean_literal(true),
                        types::literals::FALSE => Type::boolean_literal(false),
                        user_defined => Type::user_defined(user_defined),
                    });
                }
                Rule::ty => ty = Some(parse_type(pair)),
                Rule::generics => {
                    let Some(ty) = ty.as_mut() else {
                        unreachable!();
                    };

                    for pair in pair.into_inner() {
                        ty.add_generic(parse_type(pair));
                    }
                }
                Rule::array => {
                    let Some(ty) = ty.as_mut() else {
                        unreachable!();
                    };

                    ty.make_array();
                }
                _ => unreachable!(),
            };
        }

        types.extend(ty);
    }

    let mut ty = if is_union {
        assert!(types.len() > 1);
        Type::union(types)
    } else {
        assert!(types.len() == 1);
        types.pop().unwrap()
    };

    if nullable {
        ty.make_nullable();
    }

    ty
}

fn parse_function(pair: Pair<Rule>) -> Type {
    assert_eq!(pair.as_rule(), Rule::function_def);

    let mut args = Vec::new();
    let mut ret = Vec::new();

    for pair in pair.into_inner() {
        match pair.as_rule() {
            Rule::function_args => {
                for pair in pair.into_inner() {
                    assert_eq!(pair.as_rule(), Rule::function_arg);

                    let mut ident = None;
                    let mut ty = None;
                    let mut nullable = false;

                    for pair in pair.into_inner() {
                        match pair.as_rule() {
                            Rule::ident => ident = Some(pair.as_str().to_string()),
                            Rule::nullable => nullable = true,
                            Rule::ty => ty = Some(parse_type(pair)),
                            _ => unreachable!(),
                        }
                    }

                    let mut ty = ty.unwrap_or(Type::ANY);

                    if nullable {
                        ty.make_nullable();
                    }

                    let Some(ident) = ident else {
                        unreachable!();
                    };

                    args.push((ident, ty));
                }
            }
            Rule::function_returns => {
                for pair in pair.into_inner() {
                    assert_eq!(pair.as_rule(), Rule::function_return);

                    let mut ident = None;
                    let mut ty = None;

                    for pair in pair.into_inner() {
                        match pair.as_rule() {
                            Rule::ident => ident = Some(pair.as_str().to_string()),
                            Rule::ty => ty = Some(parse_type(pair)),
                            _ => unreachable!(),
                        }
                    }

                    let Some(ty) = ty else {
                        unreachable!();
                    };

                    ret.push((ident, ty));
                }
            }
            _ => unreachable!(),
        }
    }

    Type::function(args, ret)
}

fn parse_table(pair: Pair<Rule>) -> Type {
    assert_eq!(pair.as_rule(), Rule::table_def);

    let pair = pair.into_inner().next().unwrap();

    assert_eq!(pair.as_rule(), Rule::table_fields);

    let mut fields = Vec::new();

    for pair in pair.into_inner() {
        assert_eq!(pair.as_rule(), Rule::table_field);

        let mut pairs = pair.into_inner();

        let field_name_or_type = pairs.next().unwrap();
        let mut field_name_or_type = match field_name_or_type.as_rule() {
            Rule::ty => parse_type(field_name_or_type),
            Rule::ident => Type::string_literal(field_name_or_type.as_str()),
            _ => unreachable!(),
        };

        let mut ty = None;

        for pair in pairs {
            match pair.as_rule() {
                Rule::ty => ty = Some(parse_type(pair)),
                Rule::nullable => field_name_or_type.make_nullable(),
                _ => unreachable!(),
            }
        }

        fields.push((field_name_or_type, ty.unwrap()));
    }

    Type::table(fields)
}

fn parse_tuple(pair: Pair<Rule>) -> Type {
    assert_eq!(pair.as_rule(), Rule::tuple_def);

    let types = pair.into_inner().map(|pair| {
        assert_eq!(pair.as_rule(), Rule::ty);
        parse_type(pair)
    });

    Type::tuple(types)
}

pub fn parse_class(class: &str, description: Option<String>) -> anyhow::Result<Class> {
    let mut class = PestParser::parse(Rule::class, class)?;

    let mut exact = false;
    let mut name = None;
    let mut parent = None;

    for pair in class.next().unwrap().into_inner() {
        match pair.as_rule() {
            Rule::class_exact => exact = true,
            Rule::type_ident => name = Some(pair.as_str().to_string()),
            Rule::ty => parent = Some(parse_type(pair)),
            _ => unreachable!(),
        }
    }

    Ok(Class {
        name: name.unwrap(),
        description,
        exact,
        parent,
        lsp_fields: Vec::new(),
        ts_fields: Vec::new(),
        is_module: false, // TODO:
    })
}

pub fn parse_field(field: &str, description: Option<String>) -> anyhow::Result<LspField> {
    let mut field = PestParser::parse(Rule::field, field)?;

    let mut ident_type = None;
    let mut ty = None;
    let mut scope = None;
    let mut eol_desc = None;

    let mut nullable = false;

    for pair in field.next().unwrap().into_inner() {
        match pair.as_rule() {
            Rule::field_scope => {
                scope = Some(match pair.as_str() {
                    "public" => Scope::Public,
                    "private" => Scope::Private,
                    "protected" => Scope::Protected,
                    "package" => Scope::Package,
                    _ => unreachable!(),
                });
            }
            Rule::ty => {
                if pair.as_node_tag() == Some("field_ty") {
                    ident_type = Some(parse_type(pair));
                } else {
                    ty = Some(parse_type(pair));
                }
            }
            Rule::ident => {
                ident_type = Some(Type::string_literal(pair.as_str()));
            }
            Rule::nullable => nullable = true,
            Rule::rest_of_line => {
                eol_desc = Some(pair.as_str().to_string());
            }
            _ => unreachable!(),
        }
    }

    if nullable {
        ty.as_mut().unwrap().make_nullable();
    }

    Ok(LspField {
        ident_type: ident_type.unwrap(),
        ty: ty.unwrap(),
        description: description.or(eol_desc),
        scope,
    })
}

pub fn parse_alias(alias: &str, description: Option<String>) -> anyhow::Result<Alias> {
    let mut alias = PestParser::parse(Rule::alias, alias)?;

    let mut name = None;
    let mut eol_desc = None;
    let mut inline_alias = None;

    for pair in alias.next().unwrap().into_inner() {
        match pair.as_rule() {
            Rule::type_ident => name = Some(pair.as_str().to_string()),
            Rule::ty => inline_alias = Some(parse_type(pair)),
            Rule::rest_of_line => eol_desc = Some(pair.as_str().to_string()),
            _ => unreachable!(),
        }
    }

    let mut aliases = Vec::new();
    aliases.extend(inline_alias.map(|alias| (alias, eol_desc)));

    Ok(Alias {
        name: name.unwrap(),
        description,
        types: aliases,
    })
}

pub fn parse_alias_line(
    line: &str,
    description: Option<String>,
) -> anyhow::Result<(Type, Option<String>)> {
    let mut line = PestParser::parse(Rule::alias_additional_type, line)?;

    let mut ty = None;
    let mut eol_desc = None;

    for pair in line.next().unwrap().into_inner() {
        match pair.as_rule() {
            Rule::ty => ty = Some(parse_type(pair)),
            Rule::rest_of_line => eol_desc = Some(pair.as_str().to_string()),
            _ => unreachable!(),
        }
    }

    Ok((ty.unwrap(), description.or(eol_desc)))
}

pub fn parse_param(param: &str) -> anyhow::Result<Param> {
    let mut param = PestParser::parse(Rule::param, param)?;

    let mut name = None;
    let mut ty = None;
    let mut description = None;

    let mut nullable = false;

    for pair in param.next().unwrap().into_inner() {
        match pair.as_rule() {
            Rule::ident => name = Some(pair.as_str().to_string()),
            Rule::nullable => nullable = true,
            Rule::ty => ty = Some(parse_type(pair)),
            Rule::rest_of_line => description = Some(pair.as_str().to_string()),
            _ => unreachable!(),
        }
    }

    if nullable {
        ty.as_mut().unwrap().make_nullable();
    }

    Ok(Param {
        name: name.unwrap(),
        ty: ty.unwrap(),
        description,
    })
}

pub fn parse_return(param: &str) -> anyhow::Result<Return> {
    let mut ret = PestParser::parse(Rule::ret, param)?;

    let mut name = None;
    let mut ty = None;
    let mut description = None;

    for pair in ret.next().unwrap().into_inner() {
        match pair.as_rule() {
            Rule::ty => ty = Some(parse_type(pair)),
            Rule::ident => name = Some(pair.as_str().to_string()),
            Rule::rest_of_line => description = Some(pair.as_str().to_string()),
            _ => unreachable!(),
        }
    }

    Ok(Return {
        name,
        ty: ty.unwrap(),
        description,
    })
}

pub fn parse_enum(r#enum: &str, description: Option<String>) -> anyhow::Result<Enum> {
    let mut r#enum = PestParser::parse(Rule::_enum, r#enum)?;

    let mut name = None;
    let mut is_key = false;

    for pair in r#enum.next().unwrap().into_inner() {
        match pair.as_rule() {
            Rule::enum_key => is_key = true,
            Rule::type_ident => name = Some(pair.as_str().to_string()),
            Rule::rest_of_line => (),
            _ => unreachable!(),
        }
    }

    Ok(Enum {
        name: name.unwrap(),
        description,
        is_key,
        fields: Vec::new(),
    })
}

pub fn parse_lcat(lcat: &str) -> Lcat {
    let options = lcat.split_whitespace();

    let mut opts = Vec::new();

    for opt in options {
        if opt.eq_ignore_ascii_case("nodoc") {
            opts.push(LcatOption::Nodoc);
        }
    }

    Lcat { options: opts }
}

pub fn parse_type_annotation(ty: &str) -> anyhow::Result<Type> {
    let mut type_annotation = PestParser::parse(Rule::type_annotation, ty)?;

    let ty = type_annotation.next().unwrap().into_inner().next().unwrap();

    assert_eq!(ty.as_rule(), Rule::ty);

    Ok(parse_type(ty))
}

pub fn parse_see(see: &str) -> anyhow::Result<See> {
    let mut see = PestParser::parse(Rule::see, see)?;

    let mut ident = None;
    let mut desc = None;

    for pair in see.next().unwrap().into_inner() {
        match pair.as_rule() {
            Rule::type_ident => ident = Some(pair.as_str().to_string()),
            Rule::rest_of_line => desc = Some(pair.as_str().to_string()),
            _ => unreachable!(),
        }
    }

    Ok(See {
        ident: ident.unwrap(),
        description: desc,
    })
}

#[derive(Debug, Clone)]
pub struct Alias {
    pub name: String,
    pub description: Option<String>,
    pub types: Vec<(Type, Option<String>)>,
}

impl Alias {
    pub fn add_type(&mut self, ty: Type, desc: Option<String>) {
        self.types.push((ty, desc));
    }
}

#[derive(Debug, Clone)]
pub struct Class {
    pub name: String,
    pub description: Option<String>,
    pub exact: bool,
    pub parent: Option<Type>,
    pub lsp_fields: Vec<LspField>,
    pub ts_fields: Vec<TsField>,
    pub is_module: bool,
}

#[derive(Debug, Clone)]
pub struct LspField {
    pub ident_type: Type,
    pub ty: Type,
    pub description: Option<String>,
    pub scope: Option<Scope>,
}

#[derive(Debug, Clone)]
pub struct TsField {
    pub name: Option<FieldName>,
    pub ty: Option<Type>,
    pub description: Option<String>,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct ClassField {
    pub ident_type: Type,
    pub ty: Option<Type>,
    pub description: Option<String>,
    pub scope: Option<Scope>,
    pub value: Option<String>,
}

impl Class {
    pub fn fields(&self) -> Vec<ClassField> {
        let mut fields = Vec::new();

        for lsp_field in self.lsp_fields.iter() {
            let class_field = ClassField {
                ident_type: lsp_field.ident_type.clone(),
                ty: Some(lsp_field.ty.clone()),
                description: lsp_field.description.clone(),
                scope: lsp_field.scope,
                value: None,
            };

            fields.push(class_field);
        }

        for ts_field in self.ts_fields.iter() {
            let class_field = fields
                .iter_mut()
                .find(|field| match ts_field.name.as_ref() {
                    Some(FieldName::Ident(ident)) => {
                        field.ident_type == Type::string_literal(ident)
                    }
                    _ => false,
                });

            if let Some(class_field) = class_field {
                if class_field.description.is_none() {
                    class_field.description = ts_field.description.clone();
                }

                class_field.value = Some(ts_field.value.clone());
            } else {
                let Some(FieldName::Ident(ident)) = ts_field.name.as_ref() else {
                    continue;
                };

                let class_field = ClassField {
                    ident_type: Type::string_literal(ident),
                    ty: ts_field.ty.clone(),
                    description: ts_field.description.clone(),
                    scope: None,
                    value: Some(ts_field.value.clone()),
                };

                fields.push(class_field);
            }
        }

        fields
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Scope {
    Public,
    Private,
    Protected,
    Package,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Type,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Return {
    pub name: Option<String>,
    pub ty: Type,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub table: Option<String>,
    pub params: Vec<Param>,
    pub returns: Vec<Return>,
    pub sees: Vec<See>,
    pub is_method: bool,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Enum {
    pub name: String,
    pub description: Option<String>,
    pub is_key: bool,
    pub fields: Vec<TsField>,
}

#[derive(Debug, Clone)]
pub struct Lcat {
    pub options: Vec<LcatOption>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LcatOption {
    Nodoc,
}

#[derive(Debug, Clone)]
pub struct See {
    pub ident: String,
    pub description: Option<String>,
}

#[cfg(test)]
mod tests {
    use pest::Parser;

    use super::*;

    fn parse(rule: Rule, input: &str) -> anyhow::Result<()> {
        let parsed_str = PestParser::parse(rule, input)?.as_str();
        anyhow::ensure!(input == parsed_str, "failed to parse whole input");
        Ok(())
    }

    mod types {
        use super::*;

        #[test]
        fn single_type_parses() {
            parse(Rule::ty, "string").unwrap();
        }

        #[test]
        fn type_in_parentheses_parses() {
            parse(Rule::ty, "(number)").unwrap();
        }

        #[test]
        fn function_defs_parse() -> anyhow::Result<()> {
            parse(Rule::function_def, "fun()")?;
            parse(Rule::function_def, "fun(): any")?;
            parse(Rule::function_def, "fun(arg1)")?;
            parse(Rule::function_def, "fun(arg1, arg2, arg3)")?;
            parse(
                Rule::function_def,
                "fun(arg1, arg2: nil, arg3, arg4: integer): string",
            )?;

            parse(
                Rule::function_def,
                "fun(arg1, arg2: fun(inner: integer, another)): string",
            )?;

            parse(
                Rule::function_def,
                "fun(arg1, arg2: fun(inner: integer, another)): string",
            )?;

            parse(
                Rule::function_def,
                "fun(arg1, arg2: fun(): integer, boolean): string",
            )?;

            parse(
                Rule::function_def,
                "fun(arg1, arg2: (fun(): integer), bool): string",
            )?;

            // Named returns

            parse(Rule::function_def, "fun(): name: string")?;
            parse(Rule::function_def, "fun(): name: string, err: string?")?;

            Ok(())
        }

        #[test]
        fn type_idents_parse() -> anyhow::Result<()> {
            parse(Rule::type_ident, "string")?;
            parse(Rule::type_ident, "nil")?;
            parse(Rule::type_ident, "namespace.Class")?;
            parse(Rule::type_ident, "__namespace__.__Class__")?;
            // For some reason valid in the language server
            parse(Rule::type_ident, "_..._nam.e.spa.ce.__.__Class__")?;

            Ok(())
        }

        #[test]
        #[should_panic]
        fn type_ident_starting_with_number_does_not_parse() {
            parse(Rule::type_ident, "4string").unwrap();
        }

        #[test]
        fn table_defs_parse() -> anyhow::Result<()> {
            parse(Rule::table_def, "{ }")?;
            parse(Rule::table_def, "{ [string]: integer }")?;
            parse(Rule::table_def, "{ x: integer, y: integer }")?;
            parse(Rule::table_def, "{ [integer]: string, str: integer }")?;

            Ok(())
        }

        #[test]
        fn tuple_defs_parse() -> anyhow::Result<()> {
            parse(Rule::tuple_def, "[ string, integer ]")?;
            parse(
                Rule::tuple_def,
                "[fun(): string, fun(p1, p2): string, string?]",
            )?;

            Ok(())
        }

        #[test]
        fn generics_parse() -> anyhow::Result<()> {
            parse(Rule::ty, "[string, integer]<A, B, C>")?;
            parse(Rule::ty, "table<integer, string>")?;

            Ok(())
        }

        #[test]
        fn unions_parse() -> anyhow::Result<()> {
            parse(Rule::ty, "string | integer | nil")?;
            parse(
                Rule::ty,
                "table<integer, string> | (fun(): string|nil) | nil<A, B> | number?",
            )?;

            Ok(())
        }
    }

    mod annotations {
        use super::*;

        #[test]
        fn alias_parses() -> anyhow::Result<()> {
            parse(Rule::alias, r#"thing.That "possible" | "impossible""#)?;
            parse(
                Rule::alias,
                r#"Rectangle "square" | "mongus" The description"#,
            )?;

            Ok(())
        }
    }
}
