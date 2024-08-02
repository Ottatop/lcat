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

    VitePressRenderer::new(cli.out_dir.unwrap_or("./lcat_out".into()), cli.base_url)
        .render(processor);
}

#[derive(clap::Parser, Debug)]
struct Cli {
    /// Set the root search directory that lcat will look for Lua files in
    #[arg(short, long, value_name("DIR"), value_hint(ValueHint::DirPath))]
    dir: Option<PathBuf>,

    /// Add one or more Lua files to generate documentation for
    #[arg(short, long)]
    files: Vec<PathBuf>,

    /// Set the output directory (defaults to `ldoc_gen`)
    #[arg(short, long, value_name("DIR"), value_hint(ValueHint::DirPath))]
    out_dir: Option<PathBuf>,

    /// Set the base url.
    ///
    /// If you are using VitePress with GitHub pages, you need to add the repository as
    /// a base url. Because lcat uses <a> for links instead of markdown links,
    /// you also need to specify the base url here.
    #[arg(short, long)]
    base_url: Option<String>,
}
