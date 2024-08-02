use std::path::PathBuf;

use clap::{Parser, ValueHint};
use render::{vitepress::VitePressRenderer, Renderer};
use state::parse_files;

mod annotation;
mod node_types;
mod processor;
mod render;
mod state;
mod treesitter;
mod types;

fn main() {
    let cli = Cli::parse();

    let mut files = Vec::new();

    if let Some(dir) = cli.dir {
        let walkdir = walkdir::WalkDir::new(dir);

        for dir in walkdir {
            let dir = match dir {
                Ok(dir) => dir,
                Err(err) => {
                    eprintln!("{err}");
                    continue;
                }
            };

            if dir.path().extension().is_some_and(|ext| ext == "lua") {
                files.push(dir.into_path());
            }
        }
    }

    files.extend(cli.files);

    let processor = parse_files(files).unwrap();

    VitePressRenderer::new().render(processor);
}

#[derive(clap::Parser, Debug)]
struct Cli {
    #[arg(short, long, value_name("DIR"), value_hint(ValueHint::DirPath))]
    dir: Option<PathBuf>,

    #[arg(short, long)]
    files: Vec<PathBuf>,
}
