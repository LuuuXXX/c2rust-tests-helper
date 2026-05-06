# c2rust-tests-helper

A test migration helper for [`c2rust-demo`](https://github.com/LuuuXXX/c2rust-demo) feature workspaces.

## Overview

`c2rust-tests-helper` works against the `.c2rust/<feature>/...` directory tree produced by
`c2rust-demo`. It understands the generated feature surface (selected source files, Rust modules,
functions, declarations, and variables) and will grow into a full test-migration workflow tool
across subsequent PRs.

## Prerequisites

- Rust toolchain (stable)
- A `c2rust-demo` project with at least one initialised feature workspace

## Building

```bash
cargo build --release
```

The binary is placed at `target/release/c2rust-tests-helper`.

## Configuration – `migration.yml`

Place a `migration.yml` file next to your `c2rust-demo` clone (or anywhere you like; paths in the
file are resolved relative to the config file itself).

```yaml
version: 1

project:
  root: ../c2rust-demo      # path to the c2rust-demo project root
  feature: default          # feature name

feature_source:
  kind: c2rust_feature
  root: ../c2rust-demo/.c2rust/default   # generated feature workspace

test_commands:
  c: "make test"
  rust: "cargo test"

discovery:
  paths:
    - tests/c
  extensions:
    - c
  patterns:
    - regex: 'void\s+(test_\w+)\s*\('
      framework: custom

tests: []
```

A ready-to-edit sample is included in the repository as [`migration.yml`](./migration.yml).

## Commands

### `surface`

Load and print the feature surface described in the config:

```bash
c2rust-tests-helper surface --config migration.yml
```

The command:
1. Reads and validates `migration.yml`.
2. Verifies that `project.root` and `feature_source.root` exist.
3. Loads the feature workspace:
   - `meta/selected_files.json` – source files selected during `c2rust-demo init`
   - `rust/src/mod_*/fun_*.rs` – generated function wrappers
   - `rust/src/mod_*/decl_*.rs` – generated declarations
   - `rust/src/mod_*/var_*.rs` – generated variables
4. Prints a human-readable summary.

**Example output**

```
Feature root : /home/user/c2rust-demo/.c2rust/default

Selected source files (3):
  src/math.c
  src/string_utils.c
  src/io.c

Modules (2):
  [mod_math]
    functions (4):
      fun_add
      fun_div
      fun_mul
      fun_sub
  [mod_string_utils]
    functions (2):
      fun_concat
      fun_trim
    vars (1):
      var_buffer_size
```

## Feature workspace layout

`c2rust-demo` generates the following structure under `.c2rust/<feature>/`:

```
.c2rust/<feature>/
  meta/
    build_cmd.txt
    selected_files.json
  rust/
    src/
      lib.rs
      mod_<file>/
        mod.rs
        fun_<symbol>.rs
        decl_<symbol>.rs
        var_<symbol>.rs
```

`c2rust-tests-helper` consumes this layout directly — no additional API-list files are required.

## Roadmap

- **PR 1 (this PR)** – Config schema, feature surface loading, `surface` subcommand.
- **PR 2** – Test discovery and mapping (link C tests to Rust wrappers / symbols).
- **PR 3** – Full migration workflow: `check`, `lint`, `report` commands.
