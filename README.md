# lcat

A Lua documentation generator that uses [Lua language server](https://github.com/luals/lua-language-server) annotations

## Rationale

There aren't any Lua documentation generators out there that are out-of-the-box compatible with LuaLS annotations.
I would either have to:
1. Use LDoc annotations instead of LuaLS ones and have a degraded language server experience, or
2. Duplicate documentation for both LuaLS and LDoc, which is not happening.

So why not just make another docgen tool? Thus lcat was born.

## Generated documentation

Currently, lcat generates markdown files for use with [VitePress](https://github.com/vuejs/vitepress). The raw markdown
may or may not be suitable for display on, say, GitHub.

## Requirements
- Rust and Cargo
- Tree-sitter
- (optional) pnpm if you are using the [VitePress template](vitepress_template)

## Building and running

Run `cargo build` to build the project.

Run lcat by running `./target/debug/lcat` after building or by simply using `cargo run`.

lcat currently has two CLI flags:
- `-d / --dir`: Set the root directory that lcat will use when searching for Lua files.
- `-f / --files`: Add one or more files that lcat will parse and generate documentation for.

When run, lcat will parse all given and found Lua files and generate a set of markdown files in the `lcat_out` directory.
You can then copy the contained directories into your VitePress project to use them.

## Setting up a VitePress project

If you don't have a VitePress project, you can clone the [template](vitepress_template) and copy over the markdown.

Please note: this is a very minimal template and you will have to do some configuration to get links to point at the
right files and to customize anything VitePress related. See https://vitepress.dev for documentation.

## lcat-specific annotations

Add `---@lcat nodoc` before an annotation or item to remove it from the generated documentation.
Additionally, you can add it as a standalone line and anything after it won't be documented. To completely disable
docgen for a file, for example, add the annotation at the very top of the file separated by a newline:

```lua
---@lcat nodoc

-- the rest of the file
```

## Things to take note of

- Because of the way the headings are generated, you should ensure any headings used in documentation are h4 or above
(four or more `#`s) or the outline won't look great.
- Additionally, take care when using angle brackets as they may be parsed by VitePress as an invalid HTML tag.
- lcat currently does not document uncommented functions, so they will not show up in class documentation.
- Error messages don't show the actual location of the error due to the way comments are parsed. This will hopefully
  improve in the future.
