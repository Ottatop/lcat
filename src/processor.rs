use std::collections::HashMap;

use pest::Parser;

use crate::{
    annotation::{
        parse_alias, parse_alias_line, parse_class, parse_enum, parse_field, parse_lcat,
        parse_param, parse_return, parse_see, parse_type_annotation, Alias, Class, Enum, Function,
        LcatOption, Param, PestParser, Return, Rule, See, TsField,
    },
    treesitter::Block,
    types::Type,
};

#[derive(Debug, Default)]
pub struct Processor {
    pub classes: Vec<Class>,
    pub aliases: Vec<Alias>,
    pub functions: Vec<Function>,
    pub enums: Vec<Enum>,
}

#[derive(Default)]
struct FunctionAnnotations {
    params: Vec<Param>,
    returns: Vec<Return>,
    sees: Vec<See>,
}

impl FunctionAnnotations {
    fn clear(&mut self) {
        self.params.clear();
        self.returns.clear();
        self.sees.clear();
    }
}

impl Processor {
    pub fn process_blocks(&mut self, blocks: Vec<Block>) {
        // A map of table names to class names for mapping
        let mut table_class_map = HashMap::<String, String>::new();

        for block in blocks {
            if self.process_block(block, None, None, &mut table_class_map) {
                break;
            }
        }
    }

    /// Returns true if parsing should be stopped.
    #[must_use]
    fn process_block(
        &mut self,
        mut block: Block,
        mut parent_class: Option<&mut Class>,
        parent_enum: Option<&mut Enum>,
        table_class_map: &mut HashMap<String, String>,
    ) -> bool {
        enum LastDeclared {
            Class(Class),
            Alias(Alias),
            Enum(Enum),
            Type(Type),
        }

        let mut nodoc = false;

        let mut last_declared: Option<LastDeclared> = None;

        let mut fn_annotations = FunctionAnnotations::default();

        let mut doc_comments = Vec::new();

        let annotations = match &mut block {
            Block::Table(table) => std::mem::take(&mut table.annotations),
            Block::Field(field) => std::mem::take(&mut field.annotations),
            Block::Function(func) => std::mem::take(&mut func.annotations),
            Block::Free(free) => std::mem::take(&mut free.annotations),
        };

        for comment in annotations {
            match try_parse_annotation(&comment) {
                None => {
                    if let Some(LastDeclared::Alias(alias)) = last_declared.as_mut() {
                        if let Some(alias_line) = try_parse_alias_line(&comment) {
                            if let Some(alias_line) = alias_line {
                                let description =
                                    (!doc_comments.is_empty()).then(|| doc_comments.join("\n"));
                                let additional_type = parse_alias_line(&alias_line, description);
                                match additional_type {
                                    Ok((ty, ty_desc)) => {
                                        doc_comments.clear();

                                        alias.add_type(ty, ty_desc);
                                    }
                                    Err(_) => todo!(),
                                }
                            }
                            continue;
                        }
                    }
                    doc_comments.push(comment)
                }
                Some((Annotation::Class, class)) => {
                    let description = (!doc_comments.is_empty()).then(|| doc_comments.join("\n"));
                    let class = parse_class(&class, description);
                    match class {
                        Ok(class) => {
                            doc_comments.clear();

                            if nodoc {
                                nodoc = false;
                                continue;
                            }

                            let last_declared = last_declared.replace(LastDeclared::Class(class));

                            match last_declared {
                                Some(LastDeclared::Class(class)) => {
                                    self.classes.push(class);
                                }
                                Some(LastDeclared::Alias(alias)) => {
                                    self.aliases.push(alias);
                                }
                                Some(LastDeclared::Enum(r#enum)) => {
                                    self.enums.push(r#enum);
                                }
                                _ => (),
                            }

                            fn_annotations.clear();
                        }
                        Err(_) => {
                            // TODO: miette error here
                        }
                    }
                }
                Some((Annotation::Field, field)) => {
                    match last_declared.as_mut() {
                        Some(LastDeclared::Class(class)) => {
                            let description =
                                (!doc_comments.is_empty()).then(|| doc_comments.join("\n"));
                            let field = parse_field(&field, description);
                            match field {
                                Ok(field) => {
                                    doc_comments.clear();

                                    if nodoc {
                                        nodoc = false;
                                        continue;
                                    }

                                    class.lsp_fields.push(field);
                                    fn_annotations.clear();
                                }
                                Err(_) => {
                                    // TODO: miette
                                }
                            }
                        }
                        _ => continue, // TODO: warn
                    }
                }
                Some((Annotation::Alias, alias)) => {
                    let description = (!doc_comments.is_empty()).then(|| doc_comments.join("\n"));
                    let alias = parse_alias(&alias, description);
                    match alias {
                        Ok(alias) => {
                            doc_comments.clear();

                            if nodoc {
                                nodoc = false;
                                continue;
                            }

                            let last_declared = last_declared.replace(LastDeclared::Alias(alias));

                            match last_declared {
                                Some(LastDeclared::Class(class)) => {
                                    self.classes.push(class);
                                }
                                Some(LastDeclared::Alias(alias)) => {
                                    self.aliases.push(alias);
                                }
                                Some(LastDeclared::Enum(r#enum)) => {
                                    self.enums.push(r#enum);
                                }
                                _ => (),
                            }
                            fn_annotations.clear();
                        }
                        Err(_) => {
                            // TODO:
                        }
                    }
                }
                Some((Annotation::Param, param)) => {
                    let param = parse_param(&param);
                    match param {
                        Ok(param) => {
                            if nodoc {
                                nodoc = false;
                                continue;
                            }

                            fn_annotations.params.push(param);

                            match last_declared.take() {
                                Some(LastDeclared::Class(class)) => {
                                    self.classes.push(class);
                                }
                                Some(LastDeclared::Alias(alias)) => {
                                    self.aliases.push(alias);
                                }
                                Some(LastDeclared::Enum(r#enum)) => {
                                    self.enums.push(r#enum);
                                }
                                _ => (),
                            }
                        }
                        Err(err) => eprintln!("{err}"),
                    }
                }
                Some((Annotation::Return, ret)) => {
                    let ret = parse_return(&ret);
                    match ret {
                        Ok(ret) => {
                            if nodoc {
                                nodoc = false;
                                continue;
                            }

                            fn_annotations.returns.push(ret);

                            match last_declared.take() {
                                Some(LastDeclared::Class(class)) => {
                                    self.classes.push(class);
                                }
                                Some(LastDeclared::Alias(alias)) => {
                                    self.aliases.push(alias);
                                }
                                Some(LastDeclared::Enum(r#enum)) => {
                                    self.enums.push(r#enum);
                                }
                                _ => (),
                            }
                        }
                        Err(_) => todo!(),
                    }
                }
                Some((Annotation::Enum, r#enum)) => {
                    let description = (!doc_comments.is_empty()).then(|| doc_comments.join("\n"));
                    let r#enum = parse_enum(&r#enum, description);
                    match r#enum {
                        Ok(r#enum) => {
                            doc_comments.clear();

                            if nodoc {
                                nodoc = false;
                                continue;
                            }

                            let last_declared = last_declared.replace(LastDeclared::Enum(r#enum));

                            match last_declared {
                                Some(LastDeclared::Class(class)) => {
                                    self.classes.push(class);
                                }
                                Some(LastDeclared::Alias(alias)) => {
                                    self.aliases.push(alias);
                                }
                                Some(LastDeclared::Enum(r#enum)) => {
                                    self.enums.push(r#enum);
                                }
                                _ => (),
                            }
                            fn_annotations.clear();
                        }
                        Err(err) => eprintln!("{err}"),
                    }
                }
                Some((Annotation::Lcat, lcat)) => {
                    let lcat = parse_lcat(&lcat);

                    if lcat.options.contains(&LcatOption::Nodoc) {
                        nodoc = true;
                    }
                }
                Some((Annotation::Type, ty)) => {
                    let ty = parse_type_annotation(&ty);

                    match ty {
                        Ok(ty) => {
                            if nodoc {
                                nodoc = false;
                                continue;
                            }

                            let last_declared = last_declared.replace(LastDeclared::Type(ty));

                            match last_declared {
                                Some(LastDeclared::Class(class)) => {
                                    self.classes.push(class);
                                }
                                Some(LastDeclared::Alias(alias)) => {
                                    self.aliases.push(alias);
                                }
                                Some(LastDeclared::Enum(r#enum)) => {
                                    self.enums.push(r#enum);
                                }
                                _ => (),
                            }
                            fn_annotations.clear();
                        }
                        Err(_) => todo!(),
                    }
                }
                Some((Annotation::See, see)) => {
                    let see = parse_see(&see);

                    match see {
                        Ok(see) => {
                            if nodoc {
                                nodoc = false;
                                continue;
                            }

                            fn_annotations.sees.push(see);

                            match last_declared.take() {
                                Some(LastDeclared::Class(class)) => {
                                    self.classes.push(class);
                                }
                                Some(LastDeclared::Alias(alias)) => {
                                    self.aliases.push(alias);
                                }
                                Some(LastDeclared::Enum(r#enum)) => {
                                    self.enums.push(r#enum);
                                }
                                _ => (),
                            }
                        }
                        Err(_) => todo!(),
                    }
                }
                Some((Annotation::Unknown(_unknown), _)) => {
                    // TODO: warn
                }
            }
        }

        if let Some(parent_class) = parent_class.as_mut() {
            if let Block::Field(field_block) = &mut block {
                if nodoc {
                    return false;
                }

                let ty = if let Some(LastDeclared::Type(ty)) = last_declared.as_ref() {
                    Some(ty.clone())
                } else {
                    None
                };

                let field = TsField {
                    name: field_block.name.clone(),
                    ty,
                    description: (!doc_comments.is_empty()).then(|| doc_comments.join("\n")),
                    value: field_block.value.clone(),
                };

                parent_class.ts_fields.push(field);
            }
        }

        if let Some(parent_enum) = parent_enum {
            if let Block::Field(field_block) = &mut block {
                if nodoc {
                    return false;
                }

                let ty = if let Some(LastDeclared::Type(ty)) = last_declared.as_ref() {
                    Some(ty.clone())
                } else {
                    None
                };

                let field = TsField {
                    name: field_block.name.clone(),
                    ty,
                    description: (!doc_comments.is_empty()).then(|| doc_comments.join("\n")),
                    value: field_block.value.clone(),
                };

                parent_enum.fields.push(field);
            }
        }

        match last_declared.take() {
            Some(LastDeclared::Class(mut class)) => {
                if nodoc {
                    return false;
                }
                if let Block::Table(table_block) = &mut block {
                    table_class_map.insert(table_block.name.clone(), class.name.clone());

                    for block in table_block.fields.clone() {
                        if self.process_block(block, Some(&mut class), None, table_class_map) {
                            break;
                        }
                    }
                }

                self.classes.push(class);
            }
            Some(LastDeclared::Alias(alias)) => {
                if nodoc {
                    return false;
                }
                self.aliases.push(alias);
            }
            Some(LastDeclared::Enum(mut r#enum)) => {
                if nodoc {
                    return false;
                }

                if let Block::Table(table_block) = &mut block {
                    for block in table_block.fields.clone() {
                        if self.process_block(block, None, Some(&mut r#enum), table_class_map) {
                            break;
                        }
                    }
                }

                self.enums.push(r#enum);
            }
            _ => (),
        }

        if let Block::Function(function_block) = &mut block {
            if nodoc {
                return false;
            }

            let mut table = function_block.table.clone();

            if let Some(table) = table.as_mut() {
                if let Some(class_name) = table_class_map.get(table) {
                    table.clone_from(class_name);
                }
            }

            if let Some(parent_class) = parent_class {
                if let Some(table) = table.as_mut() {
                    *table = format!("{table}.{}", parent_class.name);
                } else {
                    table = Some(parent_class.name.clone());
                }
            }

            self.functions.push(Function {
                name: function_block.name.clone(),
                params: fn_annotations.params,
                returns: fn_annotations.returns,
                sees: fn_annotations.sees,
                table,
                is_method: function_block.is_method,
                description: (!doc_comments.is_empty()).then(|| doc_comments.join("\n")),
            });
        }

        nodoc
    }
}

enum Annotation {
    Class,
    Field,
    Alias,
    Param,
    Return,
    Enum,
    Lcat,
    Type,
    See,
    Unknown(String),
}

fn try_parse_annotation(line: &str) -> Option<(Annotation, String)> {
    let mut annotation = PestParser::parse(Rule::annotation, line).ok()?;

    let pairs = annotation.next().unwrap().into_inner();

    let mut ident = None;
    let mut rest_of_line = None;

    for pair in pairs {
        match pair.as_rule() {
            Rule::ident => ident = Some(pair.as_str().to_string()),
            Rule::rest_of_line => rest_of_line = Some(pair.as_str().to_string()),
            _ => unreachable!(),
        }
    }

    Some((
        match ident.unwrap().as_str() {
            "alias" => Annotation::Alias,
            "class" => Annotation::Class,
            "field" => Annotation::Field,
            "param" => Annotation::Param,
            "return" => Annotation::Return,
            "enum" => Annotation::Enum,
            "lcat" => Annotation::Lcat,
            "type" => Annotation::Type,
            "see" => Annotation::See,
            unknown => Annotation::Unknown(unknown.to_string()),
        },
        rest_of_line.unwrap_or_default(),
    ))
}

fn try_parse_alias_line(line: &str) -> Option<Option<String>> {
    let mut alias_line = PestParser::parse(Rule::piped_line, line).ok()?;

    let rest_of_line = alias_line.next().unwrap().into_inner().next();

    Some(rest_of_line.map(|line| line.as_str().to_string()))
}
