#ifndef WEBAPI_WEBSOCKET_H
#define WEBAPI_WEBSOCKET_H

#include <stdint.h>
#include <stdbool.h>

// Forward declarations
typedef struct js_value js_value_t;
typedef struct js_engine js_engine_t;

// WebSocket ready states
typedef enum {
    WS_CONNECTING = 0,
    WS_OPEN = 1,
    WS_CLOSING = 2,
    WS_CLOSED = 3
} websocket_ready_state_t;

// WebSocket close codes
typedef enum {
    WS_CLOSE_NORMAL = 1000,
    WS_CLOSE_GOING_AWAY = 1001,
    WS_CLOSE_PROTOCOL_ERROR = 1002,
    WS_CLOSE_UNSUPPORTED_DATA = 1003,
    WS_CLOSE_NO_STATUS = 1005,
    WS_CLOSE_ABNORMAL = 1006,
    WS_CLOSE_INVALID_DATA = 1007,
    WS_CLOSE_POLICY_VIOLATION = 1008,
    WS_CLOSE_MESSAGE_TOO_BIG = 1009,
    WS_CLOSE_EXTENSION_ERROR = 1010,
    WS_CLOSE_INTERNAL_ERROR = 1011,
    WS_CLOSE_SERVICE_RESTART = 1012,
    WS_CLOSE_TRY_AGAIN_LATER = 1013,
    WS_CLOSE_BAD_GATEWAY = 1014,
    WS_CLOSE_TLS_HANDSHAKE_FAILED = 1015
} websocket_close_code_t;

// WebSocket frame types
typedef enum {
    WS_FRAME_CONTINUATION = 0x0,
    WS_FRAME_TEXT = 0x1,
    WS_FRAME_BINARY = 0x2,
    WS_FRAME_CLOSE = 0x8,
    WS_FRAME_PING = 0x9,
    WS_FRAME_PONG = 0xA
} websocket_frame_type_t;

// WebSocket binary type
typedef enum {
    WS_BINARY_TYPE_BLOB,
    WS_BINARY_TYPE_ARRAYBUFFER
} websocket_binary_type_t;

// WebSocket event types
typedef enum {
    WS_EVENT_OPEN,
    WS_EVENT_MESSAGE,
    WS_EVENT_ERROR,
    WS_EVENT_CLOSE
} websocket_event_type_t;

// WebSocket event
typedef struct {
    websocket_event_type_t type;
    union {
        struct {
            // No additional data for open event
        } open;
        
        struct {
            void* data;
            uint32_t length;
            bool is_binary;
        } message;
        
        struct {
            char* message;
            uint32_t code;
        } error;
        
        struct {
            uint16_t code;
            char* reason;
            bool was_clean;
        } close;
    } data;
} websocket_event_t;

// WebSocket connection
typedef struct websocket {
    // Connection info
    char* url;
    char** protocols;
    uint32_t protocol_count;
    char* selected_protocol;
    char** extensions;
    uint32_t extension_count;
    
    // State
    websocket_ready_state_t ready_state;
    uint64_t buffered_amount;
    websocket_binary_type_t binary_type;
    
    // Network
    void* socket;
    void* tls_context;
    bool is_secure;
    
    // Frame handling
    struct {
        uint8_t* buffer;
        uint32_t buffer_size;
        uint32_t buffer_used;
        websocket_frame_type_t current_type;
        bool is_fragmented;
        bool is_masked;
        uint8_t mask_key[4];
    } frame;
    
    // Event handlers
    void (*on_open)(struct websocket* ws);
    void (*on_message)(struct websocket* ws, void* data, uint32_t length, bool is_binary);
    void (*on_error)(struct websocket* ws, const char* error);
    void (*on_close)(struct websocket* ws, uint16_t code, const char* reason);
    
    // User data
    void* user_data;
} websocket_t;

// WebSocket API functions
websocket_t* websocket_create(const char* url, const char** protocols, uint32_t protocol_count);
void websocket_destroy(websocket_t* ws);
void websocket_connect(websocket_t* ws);
void websocket_send_text(websocket_t* ws, const char* data);
void websocket_send_binary(websocket_t* ws, const void* data, uint32_t length);
void websocket_close(websocket_t* ws, uint16_t code, const char* reason);

// Frame operations
typedef struct {
    bool fin;
    bool rsv1, rsv2, rsv3;
    websocket_frame_type_t opcode;
    bool masked;
    uint8_t mask_key[4];
    uint64_t payload_length;
    uint8_t* payload;
} websocket_frame_t;

websocket_frame_t* websocket_parse_frame(const uint8_t* data, uint32_t length);
uint8_t* websocket_build_frame(websocket_frame_t* frame, uint32_t* out_length);
void websocket_frame_destroy(websocket_frame_t* frame);

// Handshake
typedef struct {
    char* host;
    uint16_t port;
    char* path;
    char* origin;
    char* key;
    char* accept;
    char** protocols;
    uint32_t protocol_count;
    char** extensions;
    uint32_t extension_count;
} websocket_handshake_t;

websocket_handshake_t* websocket_create_handshake(const char* url, const char** protocols, uint32_t protocol_count);
char* websocket_build_handshake_request(websocket_handshake_t* handshake);
bool websocket_validate_handshake_response(websocket_handshake_t* handshake, const char* response);
void websocket_handshake_destroy(websocket_handshake_t* handshake);

// Protocol handling
bool websocket_handle_ping(websocket_t* ws, const uint8_t* data, uint32_t length);
bool websocket_handle_pong(websocket_t* ws, const uint8_t* data, uint32_t length);
bool websocket_handle_close(websocket_t* ws, const uint8_t* data, uint32_t length);
void websocket_send_ping(websocket_t* ws, const uint8_t* data, uint32_t length);
void websocket_send_pong(websocket_t* ws, const uint8_t* data, uint32_t length);

// Message fragmentation
typedef struct {
    websocket_frame_type_t type;
    uint8_t* buffer;
    uint32_t buffer_size;
    uint32_t buffer_used;
    bool is_complete;
} websocket_message_t;

websocket_message_t* websocket_message_create(void);
void websocket_message_append(websocket_message_t* msg, websocket_frame_t* frame);
bool websocket_message_is_complete(websocket_message_t* msg);
void websocket_message_destroy(websocket_message_t* msg);

// Extensions
typedef struct {
    char* name;
    void* (*negotiate)(const char* params);
    void (*process_incoming)(void* context, websocket_frame_t* frame);
    void (*process_outgoing)(void* context, websocket_frame_t* frame);
    void (*destroy)(void* context);
} websocket_extension_t;

// Compression extension (permessage-deflate)
typedef struct {
    bool server_no_context_takeover;
    bool client_no_context_takeover;
    uint8_t server_max_window_bits;
    uint8_t client_max_window_bits;
    void* deflate_context;
    void* inflate_context;
} websocket_compression_t;

websocket_compression_t* websocket_compression_create(const char* params);
void websocket_compression_compress(websocket_compression_t* comp, uint8_t* data, uint32_t* length);
void websocket_compression_decompress(websocket_compression_t* comp, uint8_t* data, uint32_t* length);
void websocket_compression_destroy(websocket_compression_t* comp);

// JavaScript bindings
js_value_t* websocket_create_js(js_engine_t* engine, const char* url, js_value_t* protocols);
void websocket_bind_events(js_engine_t* engine, websocket_t* ws, js_value_t* js_ws);
js_value_t* websocket_send_js(js_engine_t* engine, websocket_t* ws, js_value_t* data);
js_value_t* websocket_close_js(js_engine_t* engine, websocket_t* ws, js_value_t* code, js_value_t* reason);

// Event handling
typedef void (*websocket_event_handler_t)(websocket_t* ws, websocket_event_t* event);

void websocket_add_event_listener(websocket_t* ws, websocket_event_type_t type, websocket_event_handler_t handler);
void websocket_remove_event_listener(websocket_t* ws, websocket_event_type_t type, websocket_event_handler_t handler);
void websocket_dispatch_event(websocket_t* ws, websocket_event_t* event);

// Connection pool
typedef struct {
    websocket_t** connections;
    uint32_t connection_count;
    uint32_t max_connections;
    uint32_t max_per_host;
} websocket_pool_t;

websocket_pool_t* websocket_pool_create(uint32_t max_connections);
websocket_t* websocket_pool_connect(websocket_pool_t* pool, const char* url, const char** protocols, uint32_t protocol_count);
void websocket_pool_close(websocket_pool_t* pool, websocket_t* ws);
void websocket_pool_close_all(websocket_pool_t* pool);
void websocket_pool_destroy(websocket_pool_t* pool);

// Auto-reconnect
typedef struct {
    websocket_t* ws;
    bool enabled;
    uint32_t retry_count;
    uint32_t max_retries;
    uint32_t retry_delay;
    uint32_t max_retry_delay;
    double backoff_factor;
    void (*on_reconnect)(websocket_t* ws);
    void (*on_give_up)(websocket_t* ws);
} websocket_reconnect_t;

websocket_reconnect_t* websocket_reconnect_create(websocket_t* ws);
void websocket_reconnect_enable(websocket_reconnect_t* reconnect);
void websocket_reconnect_disable(websocket_reconnect_t* reconnect);
void websocket_reconnect_destroy(websocket_reconnect_t* reconnect);

#endif