# c2rust-tests-helper

A test migration helper for [`c2rust-demo`](https://github.com/LuuuXXX/c2rust-demo) feature workspaces.

## Overview

`c2rust-tests-helper` works against the `.c2rust/<feature>/...` directory tree produced by
`c2rust-demo`. It understands the generated feature surface (selected source files, Rust modules,
functions, declarations, and variables) and provides a static migration-management workflow:

- **`collect`** – discover C tests and populate `migration.yml` automatically
- **`surface`** – inspect the feature surface to understand what has been generated
- **`lint`** – validate all manifest entries against the loaded feature surface
- **`report`** – view migration progress and coverage gaps at a glance

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

# project.feature must equal the last path component of feature_source.root.
# c2rust-tests-helper validates that the two fields are consistent at startup.
test_commands:
  c: "make test"
  rust: "cargo test"

discovery:
  paths:
    - tests/c               # directories to scan, relative to project.root
  extensions:
    - c
  patterns:
    - regex: 'void\s+(test_\w+)\s*\('
      framework: custom

tests: []
```

A ready-to-edit sample is included in the repository as [`migration.yml`](./migration.yml).

### Test entry schema

Each item in `tests` represents one C test and its migration state:

```yaml
tests:
  - c_test: test_add              # C test function name (unique key)
    source_file: tests/c/math.c   # relative to project.root
    status: ported                # pending | ported | skipped | not_applicable
    selected_file: src/math.c     # from meta/selected_files.json (optional)
    module: mod_math              # generated Rust module (optional)
    symbols:                      # symbols exercised by this test (optional)
      - add
    rust_tests:                   # required when status = ported
      - test_add_ported
    contract: "add(a,b)==a+b"     # optional invariant description
    notes: "reason for skipping"  # required when status = skipped
```

| Field | Required | Description |
|---|---|---|
| `c_test` | ✅ | Original C test function name – used as the unique key |
| `source_file` | | Path to the C source file, relative to `project.root` |
| `status` | | `pending` (default), `ported`, `skipped`, `not_applicable` |
| `selected_file` | | File from `meta/selected_files.json` this test covers |
| `module` | | Generated Rust module (e.g. `mod_math`) this test exercises |
| `symbols` | | Symbols within `module` covered by this test |
| `rust_tests` | | Rust test names – **required** when `status: ported` |
| `contract` | | Brief description of the tested invariant |
| `notes` | | Free-form notes – **required** when `status: skipped` |

## Commands

### `collect`

Discover C tests and merge them into the manifest:

```bash
c2rust-tests-helper collect --config migration.yml
```

The command:
1. Reads `migration.yml`.
2. Walks each directory listed in `discovery.paths` (relative to `project.root`).
3. Filters files by `discovery.extensions`.
4. Applies each regex pattern; capture group 1 is treated as the test name.
5. Appends newly discovered tests with `status: pending` – existing entries are **never modified**.
6. Writes the updated manifest back to `migration.yml`.

Running `collect` repeatedly is safe: duplicate entries are suppressed.

**Example**

```
collect: added 7 new test(s); manifest updated.
```

---

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
      add
      div
      mul
      sub
  [mod_string_utils]
    functions (2):
      concat
      trim
    vars (1):
      buffer_size
```

> **Note** — the listed source files come directly from `meta/selected_files.json`
> in the feature workspace; the example paths above are illustrative only.

> **Note** — if the feature workspace was initialised but code generation has not
> yet run (i.e. `rust/src/mod_*` directories are absent or empty), the `surface`
> command will still succeed and report zero modules.  This is normal during early
> workspace setup.

---

### `lint`

Validate the migration manifest against the loaded feature surface:

```bash
c2rust-tests-helper lint --config migration.yml
```

The command checks every entry in `tests` and reports:

| Check | Description |
|---|---|
| `selected_file` exists | File is present in `meta/selected_files.json` |
| `module` exists | Module directory `rust/src/<module>` exists |
| `symbols` exist | Each symbol is present in the named module |
| symbols need a module | An entry with `symbols` must also specify `module` |
| `ported` has `rust_tests` | Ported entries must name at least one Rust test |
| `skipped` has `notes` | Skipped entries must explain why |

All errors are collected before the command exits so you see the full picture at once.

**Example output (passing)**

```
lint: OK (12 entries checked)
```

**Example output (failing)**

```
lint error: [test_add] status is 'ported' but rust_tests is empty; add at least one Rust test name
lint error: [test_skip] status is 'skipped' but notes is missing; explain why this test is skipped
error: 2 lint error(s) found
```

---

### `report`

Print a migration-status and feature-coverage report:

```bash
c2rust-tests-helper report --config migration.yml
```

The report includes:

- **Migration Status** – counts by status (`pending`, `ported`, `skipped`, `not_applicable`)
- **Feature Surface** – totals for selected files, modules, and symbols
- **Mapping Coverage** – how many surface items are referenced by at least one test entry
- **Unmapped Gaps** – surface items not yet referenced by any test

**Example output**

```
=== Migration Status ===
  Total tests      : 12
  pending          : 7
  ported           : 3
  skipped          : 1
  not_applicable   : 1
  migration done   : 3/12 (25%)

=== Feature Surface ===
  Selected files : 4
  Modules        : 3
  Symbols        : 18

=== Mapping Coverage ===
  Selected files : 2/4 (50%)
  Modules        : 2/3 (67%)
  Symbols        : 5/18 (28%)

=== Unmapped Gaps ===
  Selected files with no mapped tests (2):
    src/io.c
    src/legacy.c
  Modules with no mapped tests (1):
    mod_legacy
  Symbols with no mapped tests (13):
    mod_math/div
    mod_math/mul
    ...
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

## Typical workflow

```bash
# 1. Inspect what the feature workspace contains.
c2rust-tests-helper surface --config migration.yml

# 2. Discover all C tests and populate migration.yml.
c2rust-tests-helper collect --config migration.yml

# 3. Edit migration.yml – add module/symbols/rust_tests mappings by hand.
$EDITOR migration.yml

# 4. Validate your edits.
c2rust-tests-helper lint --config migration.yml

# 5. Check overall progress.
c2rust-tests-helper report --config migration.yml
```

## Roadmap

- **PR 1** – Config schema, feature surface loading, `surface` subcommand.
- **PR 2 (this PR)** – `collect`, `lint`, and `report` subcommands; evolved `TestEntry` schema.
- **PR 3** – Full migration workflow: `check` command (run test suites and record results).
