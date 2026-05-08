# cJSON demo example

This directory is a self-contained example showing the full
`c2rust-tests-helper` workflow applied to **cJSON**
(a widely-used C JSON parsing library).

It demonstrates how the tool fits between a `c2rust-demo` translation
workspace and the actual writing of Rust tests.

---

## Directory layout

```text
examples/cjson/
├── meta/
│   ├── init-interface-report.md   # produced by: c2rust-demo init (cjson feature)
│   └── c-test-map-report.md       # produced by: c2rust-tests-helper map
├── tests/
│   ├── test_parse.c               # representative C parse tests
│   └── test_create.c              # representative C node-creation tests
├── c-test-discovery-report.md     # produced by: c2rust-tests-helper discover
└── README.md                      # this file
```

---

## Step 1 — Run c2rust-demo to obtain the interface report

In a real project, `c2rust-demo init` translates the C library into a Rust
workspace and emits `meta/init-interface-report.md`, listing every public
function and variable that the translated Rust code exposes through FFI.

For this demo the report is pre-filled at
[`meta/init-interface-report.md`](meta/init-interface-report.md)
with the 26 functions and 2 variables that `c2rust-demo` would extract from
[cJSON](https://github.com/DaveGamble/cJSON) (e.g. `cJSON_Parse`,
`cJSON_CreateObject`, `cJSON_Delete`, …).

---

## Step 2 — Discover C test functions

Run `c2rust-tests-helper discover` from this directory:

```bash
# from examples/cjson/
c2rust-tests-helper discover --dir tests
```

Expected output (also written to `c-test-discovery-report.md`):

```
# C Test Discovery Report

Scanned: `tests`

## Discovered Test Functions

| test function | file |
|---|---|
| test_create_array | tests/test_create.c |
| test_create_bool | tests/test_create.c |
| test_create_item_reference | tests/test_create.c |
| test_create_null | tests/test_create.c |
| test_create_number | tests/test_create.c |
| test_create_object | tests/test_create.c |
| test_create_object_helpers | tests/test_create.c |
| test_create_string | tests/test_create.c |
| test_parse_and_print | tests/test_parse.c |
| test_parse_array | tests/test_parse.c |
| test_parse_basic | tests/test_parse.c |
| test_parse_case_insensitive_lookup | tests/test_parse.c |
| test_parse_case_sensitive_lookup | tests/test_parse.c |
| test_parse_empty_object | tests/test_parse.c |
| test_parse_invalid_returns_null | tests/test_parse.c |

## Summary

| metric | value |
|---|---|
| total C tests | 15 |
```

---

## Step 3 — Map C tests to interface candidates

Run `c2rust-tests-helper map` from this directory:

```bash
# from examples/cjson/
c2rust-tests-helper map --report meta/init-interface-report.md --dir tests
```

Expected output (also written to `meta/c-test-map-report.md`):

```
# C Test to Interface Candidate Mapping Report

Scanned: `tests`

## Candidate Mappings

| test function | file | candidate symbols |
|---|---|---|
| test_create_array | tests/test_create.c | cJSON_AddItemToArray, cJSON_CreateArray, cJSON_CreateNumber, cJSON_Delete, cJSON_GetArraySize |
| test_create_bool | tests/test_create.c | cJSON_CreateBool, cJSON_CreateFalse, cJSON_CreateTrue, cJSON_Delete |
| test_create_item_reference | tests/test_create.c | cJSON_AddItemReferenceToArray, cJSON_CreateArray, cJSON_CreateString, cJSON_Delete |
| test_create_null | tests/test_create.c | cJSON_CreateNull, cJSON_Delete |
| test_create_number | tests/test_create.c | cJSON_CreateNumber, cJSON_Delete |
| test_create_object | tests/test_create.c | cJSON_AddItemToObject, cJSON_CreateNumber, cJSON_CreateObject, cJSON_Delete, cJSON_GetObjectItem |
| test_create_object_helpers | tests/test_create.c | cJSON_AddBoolToObject, cJSON_AddNullToObject, cJSON_AddNumberToObject, cJSON_AddStringToObject, cJSON_CreateObject, cJSON_Delete, cJSON_GetObjectItem |
| test_create_string | tests/test_create.c | cJSON_CreateString, cJSON_Delete |
| test_parse_and_print | tests/test_parse.c | cJSON_Delete, cJSON_Parse, cJSON_Print |
| test_parse_array | tests/test_parse.c | cJSON_Delete, cJSON_GetArrayItem, cJSON_GetArraySize, cJSON_Parse |
| test_parse_basic | tests/test_parse.c | cJSON_Delete, cJSON_GetObjectItem, cJSON_Parse |
| test_parse_case_insensitive_lookup | tests/test_parse.c | cJSON_Delete, cJSON_GetObjectItem, cJSON_Parse |
| test_parse_case_sensitive_lookup | tests/test_parse.c | cJSON_Delete, cJSON_GetObjectItemCaseSensitive, cJSON_Parse |
| test_parse_empty_object | tests/test_parse.c | cJSON_Delete, cJSON_GetArraySize, cJSON_Parse |
| test_parse_invalid_returns_null | tests/test_parse.c | cJSON_Parse, cJSON_ParseError |

## Summary

| metric | value |
|---|---|
| total C tests | 15 |
| total interface symbols | 28 |
```

Each row is matched against the **individual test function body only**, so
two functions in the same file receive independent candidate sets.
For instance `test_create_null` maps only to `cJSON_CreateNull, cJSON_Delete`
even though other functions in `test_create.c` reference many more symbols.

---

## What to do next

The candidate mapping report feeds directly into the Rust test-writing phase:
each row is a starting point for a `#[test]` function in the c2rust-demo
workspace that exercises the listed interface symbols.
