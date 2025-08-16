#ifndef BROWSER_ENGINE_H
#define BROWSER_ENGINE_H

#include <stdint.h>
#include <stdbool.h>

// Forward declarations
typedef struct dom_document dom_document_t;
typedef struct css_stylesheet css_stylesheet_t;
typedef struct js_context js_context_t;
typedef struct render_tree render_tree_t;
typedef struct browser_tab browser_tab_t;

// Browser engine configuration
typedef struct {
    uint32_t max_tabs;
    uint32_t js_heap_size;
    uint32_t cache_size;
    bool enable_gpu;
    bool enable_webgl;
    bool enable_webrtc;
    bool enable_sandbox;
    uint32_t max_workers;
} browser_config_t;

// Main browser engine structure
typedef struct browser_engine {
    browser_config_t config;
    struct {
        void* html_parser;
        void* css_parser;
        void* js_engine;
        void* render_engine;
    } parsers;
    
    struct {
        browser_tab_t** tabs;
        uint32_t tab_count;
        uint32_t active_tab;
    } tabs;
    
    struct {
        void* network_manager;
        void* cache_manager;
        void* security_manager;
        void* extension_manager;
    } managers;
    
    struct {
        uint64_t memory_usage;
        uint32_t frame_rate;
        uint32_t active_connections;
    } stats;
} browser_engine_t;

// Browser tab structure
struct browser_tab {
    uint32_t id;
    char* url;
    char* title;
    dom_document_t* document;
    js_context_t* js_context;
    render_tree_t* render_tree;
    
    struct {
        bool loading;
        bool secure;
        uint32_t progress;
    } state;
    
    struct {
        void** history;
        uint32_t history_index;
        uint32_t history_count;
    } navigation;
};

// Engine lifecycle
browser_engine_t* browser_engine_create(browser_config_t* config);
void browser_engine_destroy(browser_engine_t* engine);
int browser_engine_init(browser_engine_t* engine);
void browser_engine_shutdown(browser_engine_t* engine);

// Tab management
browser_tab_t* browser_create_tab(browser_engine_t* engine);
void browser_close_tab(browser_engine_t* engine, uint32_t tab_id);
void browser_switch_tab(browser_engine_t* engine, uint32_t tab_id);
browser_tab_t* browser_get_active_tab(browser_engine_t* engine);

// Navigation
int browser_navigate(browser_tab_t* tab, const char* url);
int browser_reload(browser_tab_t* tab);
int browser_go_back(browser_tab_t* tab);
int browser_go_forward(browser_tab_t* tab);
void browser_stop(browser_tab_t* tab);

// Content loading
int browser_load_html(browser_tab_t* tab, const char* html);
int browser_execute_script(browser_tab_t* tab, const char* script);
int browser_inject_css(browser_tab_t* tab, const char* css);

// Rendering
void browser_render_frame(browser_engine_t* engine);
void browser_paint(browser_tab_t* tab);
void browser_composite(browser_engine_t* engine);
void browser_present(browser_engine_t* engine);

// Events
typedef enum {
    BROWSER_EVENT_LOAD_START,
    BROWSER_EVENT_LOAD_COMPLETE,
    BROWSER_EVENT_LOAD_ERROR,
    BROWSER_EVENT_DOM_READY,
    BROWSER_EVENT_NAVIGATION,
    BROWSER_EVENT_SECURITY_WARNING,
    BROWSER_EVENT_DOWNLOAD_START,
    BROWSER_EVENT_DOWNLOAD_COMPLETE
} browser_event_type_t;

typedef void (*browser_event_handler_t)(browser_tab_t* tab, browser_event_type_t event, void* data);
void browser_register_event_handler(browser_engine_t* engine, browser_event_type_t event, browser_event_handler_t handler);

// Developer tools
void browser_enable_devtools(browser_engine_t* engine, bool enable);
void browser_inspect_element(browser_tab_t* tab, uint32_t x, uint32_t y);
void browser_show_console(browser_tab_t* tab);
void browser_profile_start(browser_engine_t* engine);
void browser_profile_stop(browser_engine_t* engine);

#endif