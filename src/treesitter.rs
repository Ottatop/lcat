use tree_sitter::{Node, TreeCursor};

use crate::node_types::NodeType;

/// Parse a comment block starting with `---` and position the cursor at the following node.
///
/// If the following node doesn't exist, returns false
///
/// If parse_anyway is true, if the current node is not a comment, it will still return a block.
/// This is useful for table fields.
fn parse_lsp_comment_block<'a>(
    cursor: &mut TreeCursor<'a>,
    source: &[u8],
    parse_anyway: bool,
) -> (Option<LspCommentBlock<'a>>, bool) {
    let mut current = cursor.node();

    let mut current_end_line = current.range().end_point.row;

    if current.kind() != NodeType::COMMENT {
        let block = parse_anyway.then_some(LspCommentBlock {
            comments: Vec::new(),
            commented_node: Some(current),
        });
        return (block, cursor.goto_next_sibling());
    }

    let mut comments = Vec::new();
    let mut commented_node = None;

    let current_text = current.utf8_text(source).unwrap();

    if !current_text.starts_with("---") {
        return (None, cursor.goto_next_sibling());
    }

    comments.push(current_text.strip_prefix("---").unwrap().to_string());

    let still_stuff_left = loop {
        if !cursor.goto_next_sibling() {
            break false;
        };
        let next = cursor.node();
        let next_start_line = next.range().start_point.row;

        // Only parse consecutive nodes (no newline in between)
        if current_end_line + 1 != next_start_line {
            break true;
        }

        current = next;
        current_end_line = next.range().end_point.row;

        if current.kind() != NodeType::COMMENT {
            commented_node = Some(current);
            break cursor.goto_next_sibling();
        }

        let Ok(text) = current.utf8_text(source) else {
            continue;
        };

        if text.starts_with("---") {
            comments.push(text.strip_prefix("---").unwrap().to_string());
        }
    };

    (
        Some(LspCommentBlock {
            comments,
            commented_node: commented_node.filter(|node| node.is_named()),
        }),
        still_stuff_left,
    )
}

#[derive(Debug)]
struct LspCommentBlock<'a> {
    comments: Vec<String>,
    commented_node: Option<Node<'a>>,
}

#[derive(Debug, Clone)]
pub enum Block {
    Table(TableBlock),
    Field(FieldBlock),
    Function(FunctionBlock),
    Free(FreeBlock),
}

#[derive(Debug, Clone)]
pub struct FreeBlock {
    pub annotations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TableBlock {
    pub annotations: Vec<String>,
    pub name: String,
    pub fields: Vec<Block>,
}

#[derive(Debug, Clone)]
pub struct FieldBlock {
    pub annotations: Vec<String>,
    pub name: Option<FieldName>,
    pub value: String,
}

#[derive(Debug, Clone)]
pub enum FieldName {
    Ident(String),
    Value(String),
}

impl std::fmt::Display for FieldName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = match self {
            FieldName::Ident(name) => name.to_string(),
            FieldName::Value(val) => format!("[{val}]"),
        };

        write!(f, "{repr}")
    }
}

#[derive(Debug, Clone)]
pub struct FunctionBlock {
    pub annotations: Vec<String>,
    pub table: Option<String>,
    pub name: String,
    pub params: Vec<FunctionParam>,
    pub is_method: bool,
}

#[derive(Debug, Clone)]
pub enum FunctionParam {
    Ident(String),
    Varargs,
}

pub fn parse_blocks(cursor: &mut TreeCursor, source: &[u8], parse_all: bool) -> Vec<Block> {
    let mut blocks = Vec::new();

    loop {
        let (block, still_stuff_left) = parse_lsp_comment_block(cursor, source, parse_all);
        if let Some(block) = block {
            if let Some(node) = block.commented_node {
                if let Some(table_block) = parse_table_block(node, source, &block.comments) {
                    blocks.push(Block::Table(table_block));
                } else if let Some(fn_block) = parse_function_block(node, source, &block.comments) {
                    blocks.push(Block::Function(fn_block));
                    let mut child_cursor = node.walk();
                    if child_cursor.goto_first_child() {
                        blocks.extend(parse_blocks(&mut child_cursor, source, false));
                    }
                } else if let Some(field_block) = parse_field_block(node, source, &block.comments) {
                    blocks.push(Block::Field(field_block));
                } else {
                    if !block.comments.is_empty() {
                        blocks.push(Block::Free(FreeBlock {
                            annotations: block.comments,
                        }));
                    }

                    let mut child_cursor = node.walk();
                    if child_cursor.goto_first_child() {
                        blocks.extend(parse_blocks(&mut child_cursor, source, false));
                    }
                }
            } else {
                blocks.push(Block::Free(FreeBlock {
                    annotations: block.comments,
                }));
            }
        } else {
            let mut child_cursor = cursor.node().walk();
            if child_cursor.goto_first_child() {
                blocks.extend(parse_blocks(&mut child_cursor, source, false));
            }
        }

        if !still_stuff_left {
            break;
        }
    }

    blocks
}

macro_rules! ensure {
    ($bool:expr) => {
        if !$bool {
            return None;
        }
    };
}

pub fn parse_table_block(
    mut node: Node,
    source: &[u8],
    annotations: &[String],
) -> Option<TableBlock> {
    if node.kind() == NodeType::VARIABLE_DECLARATION {
        let asm_stmt = node.named_child(0)?;
        ensure!(asm_stmt.kind() == NodeType::ASSIGNMENT_STATEMENT);
        node = asm_stmt;
    }

    if node.kind() == NodeType::ASSIGNMENT_STATEMENT {
        let var_list = node.named_child(0)?;
        ensure!(var_list.kind() == NodeType::VARIABLE_LIST);
        let expr_list = node.named_child(1)?;
        ensure!(expr_list.kind() == NodeType::EXPRESSION_LIST);
        let name = var_list.child_by_field_name("name")?;
        let value = expr_list.child_by_field_name("value")?;
        ensure!(value.kind() == NodeType::TABLE_CONSTRUCTOR);
        let mut cursor = value.walk();
        let fields = if !cursor.goto_first_child() {
            Vec::new()
        } else {
            parse_blocks(&mut cursor, source, true)
        };
        return Some(TableBlock {
            annotations: annotations.to_vec(),
            name: name.utf8_text(source).unwrap().to_string(),
            fields,
        });
    }

    if node.kind() == NodeType::FIELD {
        let name = node.child_by_field_name("name")?;
        let value = node.child_by_field_name("value")?;
        ensure!(value.kind() == NodeType::TABLE_CONSTRUCTOR);
        let mut cursor = value.walk();
        let fields = if !cursor.goto_first_child() {
            Vec::new()
        } else {
            parse_blocks(&mut cursor, source, true)
        };
        return Some(TableBlock {
            annotations: annotations.to_vec(),
            name: name.utf8_text(source).unwrap().to_string(),
            fields,
        });
    }

    None
}

pub fn parse_field_block(node: Node, source: &[u8], annotations: &[String]) -> Option<FieldBlock> {
    ensure!(node.kind() == NodeType::FIELD);
    let name = node.child_by_field_name("name");
    let value = node.child_by_field_name("value")?;

    let field_name = name.map(|name| {
        if name.kind() == NodeType::IDENTIFIER {
            FieldName::Ident(name.utf8_text(source).unwrap().to_string())
        } else {
            FieldName::Value(name.utf8_text(source).unwrap().to_string())
        }
    });

    Some(FieldBlock {
        annotations: annotations.to_vec(),
        name: field_name,
        value: value.utf8_text(source).unwrap().to_string(),
    })
}

pub fn parse_function_block(
    mut node: Node,
    source: &[u8],
    annotations: &[String],
) -> Option<FunctionBlock> {
    let parse_function_definition = |node: Node, table: Option<Node>, name: Node| {
        ensure!(node.kind() == NodeType::FUNCTION_DEFINITION);
        let parameters = node.child_by_field_name("parameters")?;
        assert_eq!(parameters.kind(), NodeType::PARAMETERS);
        let mut cursor = parameters.walk();
        let params = parameters
            .named_children(&mut cursor)
            .flat_map(|param| match param.kind() {
                NodeType::IDENTIFIER => Some(FunctionParam::Ident(
                    param.utf8_text(source).unwrap().to_string(),
                )),
                NodeType::VARARG_EXPRESSION => Some(FunctionParam::Varargs),
                _ => None,
            });
        Some(FunctionBlock {
            annotations: annotations.to_vec(),
            table: table.map(|table| table.utf8_text(source).unwrap().to_string()),
            name: name.utf8_text(source).unwrap().to_string(),
            params: params.collect(),
            is_method: false,
        })
    };

    if node.kind() == NodeType::VARIABLE_DECLARATION {
        let asm_stmt = node.named_child(0)?;
        ensure!(asm_stmt.kind() == NodeType::ASSIGNMENT_STATEMENT);
        node = asm_stmt;
    }

    if node.kind() == NodeType::ASSIGNMENT_STATEMENT {
        let var_list = node.named_child(0)?;
        ensure!(var_list.kind() == NodeType::VARIABLE_LIST);
        let expr_list = node.named_child(1)?;
        ensure!(expr_list.kind() == NodeType::EXPRESSION_LIST);
        let mut name = var_list.child_by_field_name("name")?;

        let table = if name.kind() == NodeType::DOT_INDEX_EXPRESSION {
            let table = name.child_by_field_name("table")?;
            name = name.child_by_field_name("field")?;
            Some(table)
        } else {
            None
        };

        let value = expr_list.child_by_field_name("value")?;
        return parse_function_definition(value, table, name);
    }

    if node.kind() == NodeType::FUNCTION_DECLARATION {
        let mut name = node.child_by_field_name("name")?;
        let (table, is_method) = match name.kind() {
            NodeType::DOT_INDEX_EXPRESSION => {
                let table = name.child_by_field_name("table")?;
                name = name.child_by_field_name("field")?;
                (Some(table), false)
            }
            NodeType::METHOD_INDEX_EXPRESSION => {
                let table = name.child_by_field_name("table")?;
                name = name.child_by_field_name("method")?;
                (Some(table), true)
            }
            _ => (None, false),
        };

        let parameters = node.child_by_field_name("parameters")?;
        assert_eq!(parameters.kind(), NodeType::PARAMETERS);
        let mut cursor = parameters.walk();
        let params = parameters
            .named_children(&mut cursor)
            .flat_map(|param| match param.kind() {
                NodeType::IDENTIFIER => Some(FunctionParam::Ident(
                    param.utf8_text(source).unwrap().to_string(),
                )),
                NodeType::VARARG_EXPRESSION => Some(FunctionParam::Varargs),
                _ => None,
            });
        return Some(FunctionBlock {
            annotations: annotations.to_vec(),
            table: table.map(|table| table.utf8_text(source).unwrap().to_string()),
            name: name.utf8_text(source).unwrap().to_string(),
            params: params.collect(),
            is_method,
        });
    }

    if node.kind() == NodeType::FIELD {
        let name = node.child_by_field_name("name")?;
        let value = node.child_by_field_name("value")?;
        ensure!(value.kind() == NodeType::FUNCTION_DEFINITION);
        return parse_function_definition(value, None, name);
    }

    None
}
