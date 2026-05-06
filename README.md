# c2rust-tests-helper

A minimal CLI tool for managing the migration of C tests to Rust FFI-based
tests.

When porting a C project's test suite to Rust (while keeping the C
implementation unchanged), you need to:

1. **Track** which C tests exist and which have been ported.
2. **Run** both the original C tests and the new Rust tests to ensure they
   agree.
3. **Report** progress over time.

`c2rust-tests-helper` handles all three steps with a single YAML file and
three sub-commands.

---

## Installation

```bash
cargo install --path .
```

Or build locally:

```bash
cargo build --release
# binary is at ./target/release/c2rust-tests-helper
```

---

## Quick Start

### 1. Create / customise `helper.yml`

Copy the bundled `helper.yml` to your project root and edit the `config`
section to point at your C source tree and test commands:

```yaml
config:
  discovery:
    paths:
      - tests/c          # directories to scan for C test files
    patterns:
      - regex: "void\\s+(test_\\w+)\\s*\\("   # matches "void test_foo("
        framework: custom
  c_test_command: "make test"
  rust_test_command: "cargo test"
```

### 2. Discover C tests

```bash
c2rust-tests-helper collect --config helper.yml
```

This walks the configured `paths`, applies every regex pattern, and
**merges** newly found test names into the `tests:` list in `helper.yml`.
Existing entries are never overwritten, so hand-edited `status`, `rust_tests`,
and `notes` fields are preserved.

### 3. Run both test suites and check alignment

```bash
c2rust-tests-helper check --config helper.yml
```

This:

* Validates the manifest (fails with a clear error on bad data).
* Runs `c_test_command` and `rust_test_command`.
* Prints a summary of pass/fail for each suite.
* Writes a small results file (`helper-results.yml` by default) so that
  `report` can display the last outcome without re-running tests.

### 4. Report migration progress

```bash
c2rust-tests-helper report --config helper.yml
```

Prints:

* Totals by migration status (ported / pending / skipped / n/a).
* A to-do list of pending entries.
* The last C and Rust test run results (if `helper-results.yml` exists).

---

## File layout

| File | Purpose |
|------|---------|
| `helper.yml` | Primary manifest **and** config — single source of truth. |
| `helper-results.yml` | Written by `check`; read by `report`. Keeps last run outcome so `report` is cheap. |

> **Why two files?**  Running the test suite can be slow. Caching the
> outcome in a separate file lets `report` display results instantly without
> re-executing anything.  The results file is intentionally kept separate so
> it can be ignored by version control if desired (add it to `.gitignore`).

---

## Manifest format

Each entry in `tests:` tracks one C test:

```yaml
tests:
  - name: test_add_positive        # C test function name
    source_file: tests/c/test_math.c
    status: ported                 # pending | ported | skipped | not_applicable
    rust_tests:
      - test_add_positive_numbers  # Rust test(s) that cover this C test
    notes: "Direct 1:1 port via public add() FFI wrapper."
```

### Migration statuses

| Status | Meaning |
|--------|---------|
| `pending` | No Rust test written yet (default for newly discovered entries). |
| `ported` | A Rust test covering this C test has been written. |
| `skipped` | Intentionally not ported (e.g. tests internal code not reachable via public FFI). |
| `not_applicable` | Not a real test (helper / setup function). |

---

## Custom C test frameworks

The `discovery.patterns` list accepts any number of regexes. The first capture
group is extracted as the test name. You can add entries for any custom or
self-made C test framework:

```yaml
patterns:
  # Unity
  - regex: "(?:^|\\s)TEST\\(\\s*(\\w+)\\s*\\)"
    framework: unity
  # Home-grown macro MY_TEST(suite, name) — capture full "suite_name"
  - regex: "MY_TEST\\(\\s*(\\w+)\\s*,\\s*(\\w+)\\s*\\)"
    framework: my_framework
```

> **Note on multi-group patterns:** only the **first** capture group is used
> as the test name. If your framework uses two groups (suite + name), either
> combine them in a non-capturing group or adjust the regex so that group 1
> contains the full identifier you want to track.

---

## CLI reference

```
c2rust-tests-helper <COMMAND> [OPTIONS]

Commands:
  collect   Scan C source files and merge new test entries into the manifest
  check     Validate manifest, run C and Rust tests, write results, print report
  report    Print migration summary (and optional last run results)

Options (all commands):
  -c, --config <FILE>    Path to the manifest/config YAML [default: helper.yml]

Options (check only):
  --results <FILE>       Override path for the results output file

Options (report only):
  --results <FILE>       Load results from a specific file instead of the default
```

---

## Extending the tool

The code is structured for easy extension:

| Module | Responsibility |
|--------|---------------|
| `src/manifest.rs` | Data structures, YAML I/O, validation |
| `src/collect.rs` | Discovery logic (add new parsers here) |
| `src/check.rs` | Command execution and result capture |
| `src/report.rs` | Reporting (add richer formatters here) |
| `src/runner.rs` | Shell command runner (cross-platform) |
| `src/cli.rs` | CLI definitions (add sub-commands here) |
