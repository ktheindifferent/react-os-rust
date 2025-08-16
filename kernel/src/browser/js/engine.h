#ifndef JS_ENGINE_H
#define JS_ENGINE_H

#include <stdint.h>
#include <stdbool.h>

// Forward declarations
struct dom_node;
struct dom_document;
struct dom_element;

// JavaScript value types
typedef enum {
    JS_TYPE_UNDEFINED,
    JS_TYPE_NULL,
    JS_TYPE_BOOLEAN,
    JS_TYPE_NUMBER,
    JS_TYPE_STRING,
    JS_TYPE_SYMBOL,
    JS_TYPE_BIGINT,
    JS_TYPE_OBJECT,
    JS_TYPE_FUNCTION,
    JS_TYPE_ARRAY,
    JS_TYPE_DATE,
    JS_TYPE_REGEXP,
    JS_TYPE_MAP,
    JS_TYPE_SET,
    JS_TYPE_WEAKMAP,
    JS_TYPE_WEAKSET,
    JS_TYPE_PROMISE,
    JS_TYPE_PROXY,
    JS_TYPE_ARRAYBUFFER,
    JS_TYPE_TYPEDARRAY
} js_value_type_t;

// JavaScript value
typedef struct js_value {
    js_value_type_t type;
    union {
        bool boolean;
        double number;
        char* string;
        void* object;
        int64_t bigint;
        struct {
            char* description;
            uint64_t id;
        } symbol;
    } value;
    uint32_t ref_count;
} js_value_t;

// JavaScript object
typedef struct js_object {
    js_value_t base;
    struct js_object* prototype;
    struct {
        char* key;
        js_value_t* value;
        struct {
            bool writable;
            bool enumerable;
            bool configurable;
            js_value_t* (*getter)(struct js_object*);
            void (*setter)(struct js_object*, js_value_t*);
        } descriptor;
    }* properties;
    uint32_t property_count;
    void* internal_slots;
    bool extensible;
} js_object_t;

// JavaScript function
typedef struct {
    js_object_t base;
    enum {
        FUNCTION_NORMAL,
        FUNCTION_ARROW,
        FUNCTION_ASYNC,
        FUNCTION_GENERATOR,
        FUNCTION_ASYNC_GENERATOR,
        FUNCTION_CONSTRUCTOR,
        FUNCTION_NATIVE
    } kind;
    char* name;
    char** parameters;
    uint32_t parameter_count;
    void* bytecode;
    uint32_t bytecode_size;
    js_value_t* (*native_impl)(js_value_t** args, uint32_t argc);
    struct js_context* bound_context;
    js_value_t* bound_this;
    js_value_t** bound_args;
    uint32_t bound_arg_count;
} js_function_t;

// JavaScript execution context
typedef struct js_context {
    struct js_context* parent;
    js_object_t* global_object;
    js_object_t* this_binding;
    struct {
        char* name;
        js_value_t* value;
    }* variables;
    uint32_t variable_count;
    struct {
        js_value_t** stack;
        uint32_t stack_size;
        uint32_t stack_pointer;
    } execution_stack;
    struct {
        void* current_function;
        uint32_t instruction_pointer;
        bool strict_mode;
    } execution_state;
} js_context_t;

// JavaScript engine
typedef struct {
    // Memory management
    struct {
        void* heap;
        uint64_t heap_size;
        uint64_t heap_used;
        uint32_t gc_threshold;
        bool gc_running;
    } memory;
    
    // Execution
    js_context_t* global_context;
    js_context_t* current_context;
    struct {
        js_context_t** contexts;
        uint32_t context_count;
    } context_stack;
    
    // Compilation
    struct {
        void* parser;
        void* compiler;
        void* optimizer;
        bool jit_enabled;
        uint32_t optimization_level;
    } compilation;
    
    // Built-in objects
    struct {
        js_object_t* Object;
        js_object_t* Function;
        js_object_t* Array;
        js_object_t* String;
        js_object_t* Number;
        js_object_t* Boolean;
        js_object_t* Date;
        js_object_t* RegExp;
        js_object_t* Map;
        js_object_t* Set;
        js_object_t* Promise;
        js_object_t* Symbol;
        js_object_t* BigInt;
        js_object_t* Math;
        js_object_t* JSON;
        js_object_t* console;
    } builtins;
    
    // Module system
    struct {
        struct {
            char* specifier;
            js_object_t* namespace;
            enum {
                MODULE_UNLINKED,
                MODULE_LINKING,
                MODULE_LINKED,
                MODULE_EVALUATING,
                MODULE_EVALUATED
            } status;
        }* modules;
        uint32_t module_count;
    } modules;
    
    // Event loop
    struct {
        struct {
            void (*callback)(void*);
            void* data;
            uint64_t timestamp;
        }* tasks;
        uint32_t task_count;
        struct {
            void (*callback)(void*);
            void* data;
        }* microtasks;
        uint32_t microtask_count;
        bool running;
    } event_loop;
    
    // Error handling
    struct {
        js_value_t* last_exception;
        char* stack_trace;
        void (*uncaught_handler)(js_value_t*);
    } error;
} js_engine_t;

// Engine lifecycle
js_engine_t* js_engine_create(uint64_t heap_size);
void js_engine_destroy(js_engine_t* engine);
int js_engine_init(js_engine_t* engine);
void js_engine_shutdown(js_engine_t* engine);

// Script execution
js_value_t* js_eval(js_engine_t* engine, const char* code, const char* filename);
js_value_t* js_eval_module(js_engine_t* engine, const char* code, const char* specifier);
js_value_t* js_call_function(js_engine_t* engine, js_function_t* func, js_value_t* this_arg, js_value_t** args, uint32_t argc);

// Value operations
js_value_t* js_create_undefined(void);
js_value_t* js_create_null(void);
js_value_t* js_create_boolean(bool value);
js_value_t* js_create_number(double value);
js_value_t* js_create_string(const char* value);
js_value_t* js_create_symbol(const char* description);
js_value_t* js_create_bigint(int64_t value);
js_value_t* js_create_object(js_engine_t* engine);
js_value_t* js_create_array(js_engine_t* engine, uint32_t length);
js_value_t* js_create_function(js_engine_t* engine, const char* name, js_value_t* (*impl)(js_value_t**, uint32_t));

// Type conversions
bool js_to_boolean(js_value_t* value);
double js_to_number(js_value_t* value);
char* js_to_string(js_value_t* value);
js_object_t* js_to_object(js_engine_t* engine, js_value_t* value);

// Property operations
js_value_t* js_get_property(js_object_t* object, const char* key);
void js_set_property(js_object_t* object, const char* key, js_value_t* value);
bool js_has_property(js_object_t* object, const char* key);
bool js_delete_property(js_object_t* object, const char* key);
char** js_get_property_names(js_object_t* object, uint32_t* count);

// Array operations
uint32_t js_array_length(js_value_t* array);
js_value_t* js_array_get(js_value_t* array, uint32_t index);
void js_array_set(js_value_t* array, uint32_t index, js_value_t* value);
void js_array_push(js_value_t* array, js_value_t* value);
js_value_t* js_array_pop(js_value_t* array);

// DOM bindings
void js_bind_dom(js_engine_t* engine, struct dom_document* document);
js_object_t* js_wrap_dom_node(js_engine_t* engine, struct dom_node* node);
struct dom_node* js_unwrap_dom_node(js_value_t* value);

// Web API bindings
void js_bind_fetch_api(js_engine_t* engine);
void js_bind_websocket_api(js_engine_t* engine);
void js_bind_canvas_api(js_engine_t* engine);
void js_bind_webgl_api(js_engine_t* engine);
void js_bind_audio_api(js_engine_t* engine);
void js_bind_storage_api(js_engine_t* engine);
void js_bind_worker_api(js_engine_t* engine);

// Event loop
void js_run_event_loop(js_engine_t* engine);
void js_queue_task(js_engine_t* engine, void (*callback)(void*), void* data);
void js_queue_microtask(js_engine_t* engine, void (*callback)(void*), void* data);
uint32_t js_set_timeout(js_engine_t* engine, js_function_t* callback, uint32_t delay);
uint32_t js_set_interval(js_engine_t* engine, js_function_t* callback, uint32_t interval);
void js_clear_timeout(js_engine_t* engine, uint32_t id);

// Promises
js_value_t* js_create_promise(js_engine_t* engine);
void js_promise_resolve(js_value_t* promise, js_value_t* value);
void js_promise_reject(js_value_t* promise, js_value_t* reason);
js_value_t* js_promise_then(js_value_t* promise, js_function_t* on_fulfilled, js_function_t* on_rejected);
js_value_t* js_promise_catch(js_value_t* promise, js_function_t* on_rejected);
js_value_t* js_promise_finally(js_value_t* promise, js_function_t* on_finally);

// Async/await
js_value_t* js_await(js_engine_t* engine, js_value_t* promise);
js_function_t* js_create_async_function(js_engine_t* engine, const char* name, js_value_t* (*impl)(js_value_t**, uint32_t));

// Modules
js_value_t* js_import_module(js_engine_t* engine, const char* specifier);
void js_export_value(js_engine_t* engine, const char* name, js_value_t* value);
js_value_t* js_import_value(js_engine_t* engine, const char* module, const char* name);

// Garbage collection
void js_gc_run(js_engine_t* engine);
void js_gc_mark(js_value_t* value);
void js_gc_sweep(js_engine_t* engine);
void js_value_retain(js_value_t* value);
void js_value_release(js_value_t* value);

// Error handling
js_value_t* js_create_error(js_engine_t* engine, const char* message);
js_value_t* js_create_type_error(js_engine_t* engine, const char* message);
js_value_t* js_create_reference_error(js_engine_t* engine, const char* message);
js_value_t* js_create_syntax_error(js_engine_t* engine, const char* message);
void js_throw(js_engine_t* engine, js_value_t* error);
js_value_t* js_try_catch(js_engine_t* engine, js_function_t* try_block, js_function_t* catch_block);

// Debugging
void js_debugger_attach(js_engine_t* engine);
void js_debugger_detach(js_engine_t* engine);
void js_debugger_break(js_engine_t* engine);
void js_debugger_step(js_engine_t* engine);
void js_debugger_continue(js_engine_t* engine);
void js_debugger_set_breakpoint(js_engine_t* engine, const char* file, uint32_t line);

#endif