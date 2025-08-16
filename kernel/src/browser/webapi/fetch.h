#ifndef WEBAPI_FETCH_H
#define WEBAPI_FETCH_H

#include <stdint.h>
#include <stdbool.h>

// Forward declarations
typedef struct js_value js_value_t;
typedef struct js_engine js_engine_t;

// Request methods
typedef enum {
    HTTP_GET,
    HTTP_POST,
    HTTP_PUT,
    HTTP_DELETE,
    HTTP_HEAD,
    HTTP_OPTIONS,
    HTTP_PATCH,
    HTTP_CONNECT,
    HTTP_TRACE
} http_method_t;

// Request mode
typedef enum {
    REQUEST_MODE_SAME_ORIGIN,
    REQUEST_MODE_NO_CORS,
    REQUEST_MODE_CORS,
    REQUEST_MODE_NAVIGATE
} request_mode_t;

// Request credentials
typedef enum {
    REQUEST_CREDENTIALS_OMIT,
    REQUEST_CREDENTIALS_SAME_ORIGIN,
    REQUEST_CREDENTIALS_INCLUDE
} request_credentials_t;

// Request cache
typedef enum {
    REQUEST_CACHE_DEFAULT,
    REQUEST_CACHE_NO_STORE,
    REQUEST_CACHE_RELOAD,
    REQUEST_CACHE_NO_CACHE,
    REQUEST_CACHE_FORCE_CACHE,
    REQUEST_CACHE_ONLY_IF_CACHED
} request_cache_t;

// Request redirect
typedef enum {
    REQUEST_REDIRECT_FOLLOW,
    REQUEST_REDIRECT_ERROR,
    REQUEST_REDIRECT_MANUAL
} request_redirect_t;

// HTTP headers
typedef struct {
    char* name;
    char* value;
} http_header_t;

// Headers object
typedef struct {
    http_header_t** headers;
    uint32_t header_count;
    bool immutable;
} headers_t;

// Request object
typedef struct {
    char* url;
    http_method_t method;
    headers_t* headers;
    void* body;
    uint32_t body_size;
    request_mode_t mode;
    request_credentials_t credentials;
    request_cache_t cache;
    request_redirect_t redirect;
    char* referrer;
    char* referrer_policy;
    char* integrity;
    bool keepalive;
    void* signal;
} request_t;

// Response object
typedef struct {
    char* url;
    uint16_t status;
    char* status_text;
    headers_t* headers;
    void* body;
    uint32_t body_size;
    bool ok;
    bool redirected;
    enum {
        RESPONSE_TYPE_BASIC,
        RESPONSE_TYPE_CORS,
        RESPONSE_TYPE_DEFAULT,
        RESPONSE_TYPE_ERROR,
        RESPONSE_TYPE_OPAQUE,
        RESPONSE_TYPE_OPAQUEREDIRECT
    } type;
} response_t;

// Body mixin
typedef struct {
    void* stream;
    bool body_used;
    enum {
        BODY_TYPE_NONE,
        BODY_TYPE_ARRAYBUFFER,
        BODY_TYPE_BLOB,
        BODY_TYPE_FORMDATA,
        BODY_TYPE_TEXT,
        BODY_TYPE_URLSEARCHPARAMS
    } body_type;
} body_mixin_t;

// Fetch API functions
js_value_t* fetch_api_fetch(js_engine_t* engine, const char* url, js_value_t* init);
request_t* fetch_create_request(const char* url, js_value_t* init);
response_t* fetch_create_response(void* body, js_value_t* init);
headers_t* fetch_create_headers(js_value_t* init);

// Headers operations
void headers_append(headers_t* headers, const char* name, const char* value);
void headers_delete(headers_t* headers, const char* name);
char* headers_get(headers_t* headers, const char* name);
bool headers_has(headers_t* headers, const char* name);
void headers_set(headers_t* headers, const char* name, const char* value);
http_header_t** headers_entries(headers_t* headers, uint32_t* count);

// Request operations
request_t* request_clone(request_t* request);
js_value_t* request_array_buffer(js_engine_t* engine, request_t* request);
js_value_t* request_blob(js_engine_t* engine, request_t* request);
js_value_t* request_form_data(js_engine_t* engine, request_t* request);
js_value_t* request_json(js_engine_t* engine, request_t* request);
js_value_t* request_text(js_engine_t* engine, request_t* request);

// Response operations
response_t* response_clone(response_t* response);
response_t* response_error(void);
response_t* response_redirect(const char* url, uint16_t status);
js_value_t* response_array_buffer(js_engine_t* engine, response_t* response);
js_value_t* response_blob(js_engine_t* engine, response_t* response);
js_value_t* response_form_data(js_engine_t* engine, response_t* response);
js_value_t* response_json(js_engine_t* engine, response_t* response);
js_value_t* response_text(js_engine_t* engine, response_t* response);

// Network operations
typedef struct {
    request_t* request;
    response_t* response;
    void (*on_progress)(uint64_t loaded, uint64_t total);
    void (*on_complete)(response_t* response);
    void (*on_error)(const char* error);
    bool aborted;
} fetch_operation_t;

fetch_operation_t* fetch_start(request_t* request);
void fetch_abort(fetch_operation_t* operation);
void fetch_operation_destroy(fetch_operation_t* operation);

// CORS
typedef struct {
    char** allowed_origins;
    uint32_t origin_count;
    char** allowed_methods;
    uint32_t method_count;
    char** allowed_headers;
    uint32_t allowed_header_count;
    char** exposed_headers;
    uint32_t exposed_header_count;
    bool allow_credentials;
    uint32_t max_age;
} cors_config_t;

bool cors_check_request(request_t* request, cors_config_t* config);
void cors_apply_headers(response_t* response, cors_config_t* config);

// Cache API
typedef struct {
    char* name;
    void* storage;
} cache_storage_t;

typedef struct {
    request_t* request;
    response_t* response;
    uint64_t timestamp;
} cache_entry_t;

cache_storage_t* cache_storage_open(const char* name);
void cache_storage_close(cache_storage_t* cache);
js_value_t* cache_match(js_engine_t* engine, cache_storage_t* cache, request_t* request);
js_value_t* cache_match_all(js_engine_t* engine, cache_storage_t* cache, request_t* request);
void cache_put(cache_storage_t* cache, request_t* request, response_t* response);
bool cache_delete(cache_storage_t* cache, request_t* request);
char** cache_keys(cache_storage_t* cache, uint32_t* count);

// Service Worker integration
typedef struct {
    char* scope;
    char* script_url;
    enum {
        SW_STATE_INSTALLING,
        SW_STATE_INSTALLED,
        SW_STATE_ACTIVATING,
        SW_STATE_ACTIVATED,
        SW_STATE_REDUNDANT
    } state;
    js_engine_t* worker_context;
} service_worker_t;

service_worker_t* service_worker_register(const char* script_url, const char* scope);
void service_worker_unregister(service_worker_t* worker);
response_t* service_worker_fetch(service_worker_t* worker, request_t* request);
void service_worker_post_message(service_worker_t* worker, js_value_t* message);

// Abort controller
typedef struct {
    bool aborted;
    void (*on_abort)(void* data);
    void* callback_data;
} abort_signal_t;

typedef struct {
    abort_signal_t* signal;
} abort_controller_t;

abort_controller_t* abort_controller_create(void);
void abort_controller_abort(abort_controller_t* controller);
void abort_controller_destroy(abort_controller_t* controller);
void abort_signal_add_listener(abort_signal_t* signal, void (*callback)(void*), void* data);

// Streaming
typedef struct {
    void* internal;
    bool locked;
    bool disturbed;
} readable_stream_t;

typedef struct {
    readable_stream_t* stream;
    bool closed;
} readable_stream_reader_t;

readable_stream_t* readable_stream_create(void* source);
readable_stream_reader_t* readable_stream_get_reader(readable_stream_t* stream);
js_value_t* readable_stream_read(js_engine_t* engine, readable_stream_reader_t* reader);
void readable_stream_cancel(readable_stream_t* stream, js_value_t* reason);
void readable_stream_close(readable_stream_t* stream);

// FormData
typedef struct {
    struct {
        char* name;
        enum {
            FORM_DATA_TEXT,
            FORM_DATA_FILE
        } type;
        union {
            char* text;
            struct {
                void* data;
                uint32_t size;
                char* filename;
                char* content_type;
            } file;
        } value;
    }* entries;
    uint32_t entry_count;
} form_data_t;

form_data_t* form_data_create(void);
void form_data_append(form_data_t* data, const char* name, js_value_t* value);
void form_data_delete(form_data_t* data, const char* name);
js_value_t* form_data_get(js_engine_t* engine, form_data_t* data, const char* name);
js_value_t** form_data_get_all(js_engine_t* engine, form_data_t* data, const char* name, uint32_t* count);
bool form_data_has(form_data_t* data, const char* name);
void form_data_set(form_data_t* data, const char* name, js_value_t* value);
void form_data_destroy(form_data_t* data);

#endif