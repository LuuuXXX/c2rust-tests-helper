/* test_parse.c — representative cJSON parsing tests */

#include <assert.h>
#include <stdlib.h>
#include "cJSON.h"

/* Basic round-trip: parse a JSON object and verify a field. */
void test_parse_basic(void)
{
    const char *json = "{\"name\":\"Alice\",\"age\":30}";
    cJSON *root = cJSON_Parse(json);
    assert(root != NULL);
    cJSON *name = cJSON_GetObjectItem(root, "name");
    assert(name != NULL);
    cJSON_Delete(root);
}

/* Parsing an empty JSON object should succeed. */
void test_parse_empty_object(void)
{
    cJSON *root = cJSON_Parse("{}");
    assert(root != NULL);
    assert(cJSON_GetArraySize(root) == 0);
    cJSON_Delete(root);
}

/* Parsing a JSON array should return non-NULL. */
void test_parse_array(void)
{
    const char *json = "[1, 2, 3]";
    cJSON *arr = cJSON_Parse(json);
    assert(arr != NULL);
    assert(cJSON_GetArraySize(arr) == 3);
    cJSON *item = cJSON_GetArrayItem(arr, 0);
    assert(item != NULL);
    cJSON_Delete(arr);
}

/* Parsing invalid JSON should return NULL and record an error position. */
void test_parse_invalid_returns_null(void)
{
    cJSON *result = cJSON_Parse("{bad json}");
    assert(result == NULL);
    assert(cJSON_ParseError != NULL);
}

/* Case-insensitive lookup via cJSON_GetObjectItem. */
void test_parse_case_insensitive_lookup(void)
{
    cJSON *root = cJSON_Parse("{\"Key\":42}");
    assert(root != NULL);
    cJSON *item = cJSON_GetObjectItem(root, "key");
    assert(item != NULL);
    cJSON_Delete(root);
}

/* Case-sensitive lookup via cJSON_GetObjectItemCaseSensitive. */
void test_parse_case_sensitive_lookup(void)
{
    cJSON *root = cJSON_Parse("{\"Key\":42}");
    assert(root != NULL);
    cJSON *found     = cJSON_GetObjectItemCaseSensitive(root, "Key");
    cJSON *not_found = cJSON_GetObjectItemCaseSensitive(root, "key");
    assert(found != NULL);
    assert(not_found == NULL);
    cJSON_Delete(root);
}

/* cJSON_Print should produce a non-NULL printable string. */
void test_parse_and_print(void)
{
    cJSON *root = cJSON_Parse("{\"x\":1}");
    assert(root != NULL);
    char *text = cJSON_Print(root);
    assert(text != NULL);
    free(text);
    cJSON_Delete(root);
}
