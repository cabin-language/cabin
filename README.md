# Cabin

**Warning: Cabin is in pre-alpha. It is still in active development and *won't* work, not even for small or experimental projects. Note that items that are not checked off are either currently unfinished or not even started.**

A dead simple, highly performant, extremely safe programming language.

## Installation

Cabin is not yet available for installation, because it's *that* early on. It will release in 0.1 for alpha testing.

## Philosophy & Motivation

Cabin has three "core values":

- Simplicity
- Safety
- Performance

Above all else, Cabin aims to be a dead simple language that anyone can learn in almost no time, while not compromising on performance or safety.

Cabin aims to fill the "missing hole" in the intersection of these three values:

![motivation](./docs/motivation.png)

There are other attempts to fill this hole as well, such as Nim and V. This is just one of them.

Cabin is primarily inspired by Lua, Rust, Go, and Zig, except it aims to be type-safer than Lua, simpler than Rust, faster than Go, and memory-safer than Zig. That is the niche Cabin aims to fill.

## Tooling

By default, the Cabin compiler comes with the following tools:

- [x] Project Creator: Creates new Cabin projects with a set up config file and source folder.
- [x] Runner: Runs Cabin code without outputting any permanent files
- [x] Project Configurer: Changes compiler options for a given project
- [ ] Compiler: Compiles Cabin code to native binary executable
- [ ] Formatter: Formats Cabin code to a single unified style
- [ ] Transpiler: Transpiles Cabin code to C
- [ ] Linter: Provides code diagnostics including errors, warnings, hints, and information.
- [ ] Package Manager: Manages Cabin dependencies, publishes cabin packages, etc.

## Philosophy

If it can be done readably with existing syntax, it should be.

Readable is good; traditional is not necessarily so.

Software can be complete.
