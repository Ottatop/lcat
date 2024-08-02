# lcat

A Lua documentation generator that uses [Lua language server](https://github.com/luals/lua-language-server) annotations

## Rationale

There aren't any Lua documentation generators out there that are out-of-the-box compatible with LuaLS annotations.
I'd also rather not sacrifice the quality of language server completions in order to have decent documentation.

## Generated documentation

Currently, lcat generates markdown files for use with [VitePress](https://github.com/vuejs/vitepress). The raw markdown
may or may not be suitable for display on, say, GitHub.

## Requirements
- Rust and Cargo

## Building and running

Run `cargo build` to build the project.

Run lcat by running `./target/debug/lcat` after building or by simply using `cargo run`.

lcat currently has two CLI flags:
- `-d / --dir`: Set the root directory that lcat will use when searching for Lua files.
- `-f / --files`: Add one or more files that lcat will parse and generate documentation for.

When run, lcat will parse all given and found Lua files and generate a set of markdown files in the `lcat_out` directory.
You can then copy these files into your VitePress project to use them.

## Things to take note of

- Because of the way the headings are generated, you should ensure any headings used in documentation are h4 or above
(four or more `#`s).
- Additionally, take care when using angle brackets as they may be parsed by VitePress as an invalid HTML tag.
