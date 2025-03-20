use std::{collections::HashMap, path::PathBuf};

use markdown::ParseOptions;

use crate::{annotation::Function, processor::Processor, treesitter::FieldName, types::Metatype};

use super::Renderer;

pub struct VitePressRenderer {
    out_dir: PathBuf,
    base_url: String,
}

impl VitePressRenderer {
    pub fn new(out_dir: PathBuf, base_url: Option<String>) -> Self {
        Self {
            out_dir,
            base_url: base_url.unwrap_or("/".into()),
        }
    }
}

impl Renderer for VitePressRenderer {
    type Output = ();

    fn render(&mut self, processor: Processor) -> Self::Output {
        let dir = tempfile::tempdir().unwrap();
        let root_dir = dir.path();
        let class_dir = root_dir.join("classes");
        let alias_dir = root_dir.join("aliases");
        let enum_dir = root_dir.join("enums");
        std::fs::create_dir_all(&class_dir).unwrap();
        std::fs::create_dir_all(&alias_dir).unwrap();
        std::fs::create_dir_all(&enum_dir).unwrap();

        let Processor {
            classes,
            aliases,
            mut functions,
            enums,
        } = processor;

        let ident_lookup = {
            let mut map = HashMap::new();

            for class in classes.iter() {
                map.insert(class.name.clone(), Metatype::Class);
            }

            for alias in aliases.iter() {
                map.insert(alias.name.clone(), Metatype::Alias);
            }

            for en in enums.iter() {
                map.insert(en.name.clone(), Metatype::Enum);
            }

            map
        };

        for class in classes {
            let name = class.name.clone();
            let desc = class.description.clone().unwrap_or_default();
            let parent = class
                .parent
                .as_ref()
                .map(|ty| {
                    format!(
                        " : <code>{}</code>",
                        ty.format_with_links(&ident_lookup, &self.base_url)
                    )
                })
                .unwrap_or_default();

            let mut class_functions = Vec::new();
            functions.retain(|func| {
                if func.table.as_ref().is_some_and(|table| table == &name) {
                    class_functions.push(func.clone());
                    false
                } else {
                    true
                }
            });

            let mut fields =
                class
                    .fields()
                    .into_iter()
                    .map(|field| {
                        let description = field.description.unwrap_or_default();
                        let badge = field
                            .ty
                            .as_ref()
                            .and_then(|ty| {
                                ty.nullable
                                    .then_some(r#" <Badge type="danger" text="nullable" />"#)
                            })
                            .unwrap_or_default();
                        let nullable = field
                            .ty
                            .as_ref()
                            .and_then(|ty| ty.nullable.then_some("?"))
                            .unwrap_or_default();
                        let name = field.ident_type.format_as_table_field_name();
                        let value = field
                            .value
                            .map(|value| format!(" = `{value}`"))
                            .unwrap_or_default();
                        let ty = field
                            .ty
                            .map(|ty| {
                                format!(
                                    ": <code>{}</code>",
                                    ty.format_with_links(&ident_lookup, &self.base_url)
                                )
                            })
                            .unwrap_or_default();

                        format!(
                            "### {name}{badge}\n\n`{name}{nullable}`{ty}{value}\n\n{description}\n",
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

            if !fields.is_empty() {
                fields = format!("## Fields\n\n{fields}")
            }

            let mut class_functions = class_functions
                .into_iter()
                .map(|func| generate_function_block(&func, &ident_lookup, &self.base_url))
                .collect::<Vec<_>>()
                .join("\n");

            if !class_functions.is_empty() {
                class_functions = format!("## Functions\n\n{class_functions}");
            }

            let exact_badge = class
                .exact
                .then_some(r#"<Badge type="tip" text="exact" />"#)
                .unwrap_or_default();

            let mut contents = format!(
                r#"---
outline: [2, 3]
---

# Class `{name}`{parent}
{exact_badge}

{desc}

{fields}

{class_functions}"#
            );

            contents = sanitize_angle_brackets(contents);

            let write_to = class_dir.join(format!("{name}.md"));
            std::fs::write(write_to, contents).unwrap();
        }

        for alias in aliases {
            let name = alias.name.clone();
            let desc = alias.description.clone().unwrap_or_default();

            let types_short = alias
                .types
                .iter()
                .map(|(ty, _desc)| {
                    format!(
                        "<code>{}</code>",
                        ty.format_with_links(&ident_lookup, &self.base_url)
                    )
                })
                .collect::<Vec<_>>()
                .join(" | ");

            let mut types = alias
                .types
                .into_iter()
                .map(|(ty, desc)| {
                    format!(
                        "### <code>{}</code>\n\n{}\n",
                        ty.format_with_links(&ident_lookup, &self.base_url),
                        desc.unwrap_or_default()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");

            if !types.is_empty() {
                types = format!("## Aliased types\n\n{types}");
            }

            let contents = format!(
                r#"---
outline: [2, 3]
---

# Alias `{name}`

{types_short}

{desc}

{types}"#
            );

            let write_to = alias_dir.join(format!("{name}.md"));
            std::fs::write(write_to, contents).unwrap();
        }

        for en in enums {
            let name = en.name.clone();
            let desc = en.description.clone().unwrap_or_default();
            let key = en.is_key;

            let key_badge = key
                .then_some(r#"<Badge type="tip" text="key" />"#)
                .unwrap_or_default();

            let values_short = key
                .then(|| {
                    en.fields
                        .iter()
                        .filter_map(|field| {
                            if let Some(FieldName::Ident(ident)) = field.name.as_ref() {
                                Some(format!(r#"`"{}"`"#, ident))
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" | ")
                })
                .unwrap_or_default();

            let body = if key {
                let mut values = en
                    .fields
                    .iter()
                    .filter_map(|field| {
                        if let Some(FieldName::Ident(ident)) = field.name.as_ref() {
                            Some(format!(
                                "### `\"{}\"`\n\n{}\n",
                                ident,
                                field.description.as_deref().unwrap_or_default()
                            ))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                if !values.is_empty() {
                    values = format!("## Values\n\n{values}");
                }

                values
            } else {
                let mut fields = en
                    .fields
                    .iter()
                    .filter_map(|field| {
                        if let Some(FieldName::Ident(ident)) = field.name.as_ref() {
                            let short_form = format!("`{name}.{ident}` = `{}`", field.value);
                            Some(format!(
                                "### `{}`\n\n{short_form}\n\n{}\n",
                                ident,
                                field.description.as_deref().unwrap_or_default()
                            ))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                if !fields.is_empty() {
                    fields = format!("## Fields\n\n{fields}");
                }

                fields
            };

            let contents = format!(
                r"---
outline: [2, 3]
---

# Enum `{name}`
{key_badge}

{values_short}

{desc}

{body}
"
            );

            let write_to = enum_dir.join(format!("{name}.md"));
            std::fs::write(write_to, contents).unwrap();
        }

        let _ = std::fs::remove_dir_all(self.out_dir.join("classes"));
        let _ = std::fs::remove_dir_all(self.out_dir.join("enums"));
        let _ = std::fs::remove_dir_all(self.out_dir.join("aliases"));

        dircpy::copy_dir_advanced(
            root_dir,
            &self.out_dir,
            true,
            true,
            true,
            Vec::new(),
            vec![".md".to_string()],
        )
        .unwrap();
    }
}

fn sanitize_angle_brackets(markdown: impl ToString) -> String {
    let mut markdown = markdown.to_string();

    let node = markdown::to_mdast(&markdown, &ParseOptions::default()).unwrap();

    use markdown::mdast::Node;

    fn process(node: &Node, md: &str, indices: &mut Vec<usize>) {
        match node {
            Node::Code(_) | Node::InlineCode(_) | Node::Html(_) => (),
            other => {
                let has_children =
                    matches!(other.children(), Some(children) if !children.is_empty());

                if let Some(pos) = other.position() {
                    if !has_children {
                        let start_pos = pos.start.offset;
                        let end_pos = pos.end.offset;

                        let to_replace_indices = md[start_pos..end_pos]
                            .match_indices('<')
                            .map(|(i, _)| i + start_pos);

                        indices.extend(to_replace_indices);
                    }
                }

                if let Some(children) = other.children() {
                    for node in children {
                        process(node, md, indices);
                    }
                }
            }
        }
    }

    let mut indices = Vec::new();

    process(&node, &markdown, &mut indices);

    for (num_replaced, index) in indices.into_iter().enumerate() {
        assert!(
            markdown.get((index + num_replaced * 3)..(index + num_replaced * 3 + 1)) == Some("<")
        );
        markdown.replace_range(
            (index + num_replaced * 3)..(index + num_replaced * 3 + 1),
            "&lt;",
        );
    }

    markdown
}

fn generate_function_block(
    func: &Function,
    ident_lookup: &HashMap<String, Metatype>,
    base_url: &str,
) -> String {
    let is_method = func.is_method;
    let badge = if is_method {
        r#"<Badge type="method" text="method" />"#.to_string()
    } else {
        r#"<Badge type="function" text="function" />"#.to_string()
    };
    let description = func.description.clone().unwrap_or_default();

    let params_short = func
        .params
        .iter()
        .map(|param| {
            let nullable = param.ty.nullable.then_some("?").unwrap_or_default();
            let ty = param.ty.format_with_links(ident_lookup, base_url);
            format!("{}{nullable}: {}", param.name, ty)
        })
        .collect::<Vec<_>>()
        .join(", ");

    let mut returns_short = func
        .returns
        .iter()
        .map(|ret| {
            let name = ret
                .name
                .as_ref()
                .map(|name| format!("{name}: "))
                .unwrap_or_default();
            // let ty = super::sanitize_angle_brackets(&ret.ty.to_string());
            let nullable = ret.ty.nullable.then_some("?").unwrap_or_default();
            let ty = ret.ty.format_with_links(ident_lookup, base_url);
            format!("{name}{ty}{nullable}")
        })
        .collect::<Vec<_>>()
        .join(", ");

    if !returns_short.is_empty() {
        returns_short = format!("\n    -> {returns_short}");
    }

    let mut params = func
        .params
        .iter()
        .map(|param| {
            let description = param
                .description
                .as_ref()
                .map(|desc| format!(" - {desc}"))
                .unwrap_or_default();
            let nullable = param.ty.nullable.then_some("?").unwrap_or_default();
            format!(
                "`{}{nullable}`: <code>{}</code>{}",
                param.name,
                param.ty.format_with_links(ident_lookup, base_url),
                description
            )
        })
        .collect::<Vec<_>>()
        .join("<br>\n");

    if !params.is_empty() {
        params = format!("#### Parameters\n\n{params}\n\n");
    }

    let mut returns = func
        .returns
        .iter()
        .enumerate()
        .map(|(i, ret)| {
            let name = ret
                .name
                .as_ref()
                .map(|name| format!("`{name}`: "))
                .unwrap_or_default();
            let description = ret
                .description
                .as_ref()
                .map(|desc| format!(" - {desc}"))
                .unwrap_or_default();
            format!(
                "{}. {name}<code>{}</code>{description}",
                i + 1,
                ret.ty.format_with_links(ident_lookup, base_url)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    if !returns.is_empty() {
        returns = format!("#### Returns\n\n{returns}\n\n");
    }

    let mut sees = func
        .sees
        .iter()
        .filter_map(|see| {
            let mut belonging_type = Vec::<&str>::new();
            let mut split = see.ident.split('.').peekable();
            while let Some(segment) = split.peek() {
                let test = belonging_type
                    .iter()
                    .copied()
                    .chain([*segment])
                    .collect::<Vec<_>>()
                    .join(".");
                let exists = ident_lookup.get(&test).is_some();
                if exists {
                    belonging_type.push(segment);
                    split.next();
                } else {
                    break;
                }
            }

            let belonging_type = belonging_type.join(".");

            let path = match ident_lookup.get(&belonging_type)? {
                Metatype::Class => "classes",
                Metatype::Alias => "aliases",
                Metatype::Enum => "enums",
            };

            let mut rest = split.collect::<Vec<_>>().join(".");
            let mut rest_with_dot = String::new();

            if !rest.is_empty() {
                rest_with_dot = format!(".{rest}");
                rest = format!("#{rest}");
            }

            let desc = see
                .description
                .as_ref()
                .map(|desc| format!(": {desc}"))
                .unwrap_or_default();

            Some(format!(
                "- <code><a href=\"{base_url}{path}/{belonging_type}{rest}\">\
                {belonging_type}{rest_with_dot}</a></code>{desc}",
            ))
        })
        .collect::<Vec<_>>()
        .join(".");

    if !sees.is_empty() {
        sees = format!("#### See also\n\n{sees}");
    }

    let table = func
        .table
        .as_ref()
        .map(|table| {
            let connector = if is_method { ":" } else { "." };
            format!("{table}{connector}")
        })
        .unwrap_or_default();

    let fn_name = &func.name;

    #[rustfmt::skip]
    let ret = format!(
r#"### {badge} {fn_name}

<div class="language-lua"><pre><code>function {table}{fn_name}({params_short}){returns_short}</code></pre></div>

{description}

{params}

{returns}

{sees}"#,
    );

    ret
}
