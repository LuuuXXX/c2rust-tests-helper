/* test_create.c — representative cJSON node-creation tests */

#include <assert.h>
#include <stdlib.h>
#include "cJSON.h"

/* cJSON_CreateNull should produce a non-NULL null node. */
void test_create_null(void)
{
    cJSON *node = cJSON_CreateNull();
    assert(node != NULL);
    cJSON_Delete(node);
}

/* cJSON_CreateBool with true / false values. */
void test_create_bool(void)
{
    cJSON *t = cJSON_CreateTrue();
    cJSON *f = cJSON_CreateFalse();
    assert(t != NULL);
    assert(f != NULL);
    cJSON *b = cJSON_CreateBool(1);
    assert(b != NULL);
    cJSON_Delete(t);
    cJSON_Delete(f);
    cJSON_Delete(b);
}

/* cJSON_CreateNumber should wrap a numeric value. */
void test_create_number(void)
{
    cJSON *node = cJSON_CreateNumber(3.14);
    assert(node != NULL);
    cJSON_Delete(node);
}

/* cJSON_CreateString should copy the given string. */
void test_create_string(void)
{
    cJSON *node = cJSON_CreateString("hello");
    assert(node != NULL);
    cJSON_Delete(node);
}

/* Build a simple JSON object with AddItemToObject. */
void test_create_object(void)
{
    cJSON *obj = cJSON_CreateObject();
    assert(obj != NULL);
    cJSON_AddItemToObject(obj, "count", cJSON_CreateNumber(1));
    assert(cJSON_GetObjectItem(obj, "count") != NULL);
    cJSON_Delete(obj);
}

/* Build a JSON object using the AddXxxToObject helpers. */
void test_create_object_helpers(void)
{
    cJSON *obj = cJSON_CreateObject();
    assert(obj != NULL);
    cJSON_AddStringToObject(obj, "name", "demo");
    cJSON_AddNumberToObject(obj, "value", 42);
    cJSON_AddBoolToObject(obj, "active", 1);
    cJSON_AddNullToObject(obj, "extra");
    assert(cJSON_GetObjectItem(obj, "name")   != NULL);
    assert(cJSON_GetObjectItem(obj, "value")  != NULL);
    assert(cJSON_GetObjectItem(obj, "active") != NULL);
    assert(cJSON_GetObjectItem(obj, "extra")  != NULL);
    cJSON_Delete(obj);
}

/* Build a JSON array with AddItemToArray. */
void test_create_array(void)
{
    cJSON *arr = cJSON_CreateArray();
    assert(arr != NULL);
    cJSON_AddItemToArray(arr, cJSON_CreateNumber(1));
    cJSON_AddItemToArray(arr, cJSON_CreateNumber(2));
    assert(cJSON_GetArraySize(arr) == 2);
    cJSON_Delete(arr);
}

/* Add items by reference — original nodes stay valid after array delete. */
void test_create_item_reference(void)
{
    cJSON *shared = cJSON_CreateString("shared");
    assert(shared != NULL);
    cJSON *arr = cJSON_CreateArray();
    assert(arr != NULL);
    cJSON_AddItemReferenceToArray(arr, shared);
    /* arr deletion must not double-free shared */
    cJSON_Delete(arr);
    cJSON_Delete(shared);
}
