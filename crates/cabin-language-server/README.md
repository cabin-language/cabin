# Cabin Language Server

The official Language Server Protocol implementation for the Cabin programming language.

## Installation

### Neovim

Cabin's language server is automatically installed and set up for Neovim in the [cabin.nvim](htts://github.com/cabin-language/cabin.nvim.git) plugin.

### Manual

You can install the Cabin language server manually through `cargo`:

```bash
cargo install cabin-language-server
```

It will become available under the `cabin-language-server` binary executable name.

## Performance

The cabin language server is written in Rust, and because the Cabin compiler itself is written in Rust, it doesn't need any kind of "bindings" to the language&mdash;it uses the actual compiler's source code to get information about Cabin code. This makes the language server *extremely* fast.
