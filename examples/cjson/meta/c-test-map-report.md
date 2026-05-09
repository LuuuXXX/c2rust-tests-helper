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
