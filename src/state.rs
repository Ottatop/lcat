use std::path::PathBuf;

use anyhow::Context;

use crate::{processor::Processor, treesitter::parse_blocks};

pub fn parse_files(paths: Vec<PathBuf>) -> anyhow::Result<Processor> {
    let mut ts_parser = tree_sitter::Parser::new();
    ts_parser.set_language(&tree_sitter_lua::language())?;

    let mut processor = Processor::default();

    for path in paths {
        let contents = std::fs::read_to_string(&path)?;

        let tree = ts_parser.parse(&contents, None).context("parse failed")?;
        let mut cursor = tree.walk();

        let blocks = parse_blocks(&mut cursor, contents.as_bytes(), false);

        processor.process_blocks(blocks);
    }

    Ok(processor)
}
