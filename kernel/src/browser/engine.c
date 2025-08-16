#include "engine.h"
#include "html/parser.h"
#include "html/dom.h"
#include "css/parser.h"
#include "css/style.h"
#include "js/engine.h"
#include "render/engine.h"
#include "webapi/fetch.h"
#include "webapi/websocket.h"
#include "security/csp.h"
#include "network/http.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

// Create browser engine
browser_engine_t* browser_engine_create(browser_config_t* config) {
    browser_engine_t* engine = calloc(1, sizeof(browser_engine_t));
    if (!engine) return NULL;
    
    // Copy configuration
    if (config) {
        engine->config = *config;
    } else {
        // Default configuration
        engine->config.max_tabs = 100;
        engine->config.js_heap_size = 256 * 1024 * 1024; // 256MB
        engine->config.cache_size = 100 * 1024 * 1024; // 100MB
        engine->config.enable_gpu = true;
        engine->config.enable_webgl = true;
        engine->config.enable_webrtc = true;
        engine->config.enable_sandbox = true;
        engine->config.max_workers = 4;
    }
    
    // Allocate tab array
    engine->tabs.tabs = calloc(engine->config.max_tabs, sizeof(browser_tab_t*));
    if (!engine->tabs.tabs) {
        free(engine);
        return NULL;
    }
    
    return engine;
}

// Initialize browser engine
int browser_engine_init(browser_engine_t* engine) {
    if (!engine) return -1;
    
    // Initialize HTML parser
    engine->parsers.html_parser = html_parser_create();
    if (!engine->parsers.html_parser) return -1;
    
    // Initialize CSS parser
    engine->parsers.css_parser = calloc(1, sizeof(void*));
    if (!engine->parsers.css_parser) return -1;
    
    // Initialize JavaScript engine
    engine->parsers.js_engine = js_engine_create(engine->config.js_heap_size);
    if (!engine->parsers.js_engine) return -1;
    js_engine_init(engine->parsers.js_engine);
    
    // Initialize rendering engine
    engine->parsers.render_engine = calloc(1, sizeof(render_pipeline_t));
    if (!engine->parsers.render_engine) return -1;
    
    // Initialize network manager
    engine->managers.network_manager = calloc(1, sizeof(void*));
    
    // Initialize cache manager
    engine->managers.cache_manager = calloc(1, sizeof(void*));
    
    // Initialize security manager
    engine->managers.security_manager = calloc(1, sizeof(void*));
    
    // Initialize extension manager
    engine->managers.extension_manager = calloc(1, sizeof(void*));
    
    // Bind Web APIs to JavaScript engine
    js_bind_fetch_api(engine->parsers.js_engine);
    js_bind_websocket_api(engine->parsers.js_engine);
    js_bind_canvas_api(engine->parsers.js_engine);
    js_bind_webgl_api(engine->parsers.js_engine);
    js_bind_storage_api(engine->parsers.js_engine);
    js_bind_worker_api(engine->parsers.js_engine);
    
    return 0;
}

// Create new tab
browser_tab_t* browser_create_tab(browser_engine_t* engine) {
    if (!engine || engine->tabs.tab_count >= engine->config.max_tabs) {
        return NULL;
    }
    
    browser_tab_t* tab = calloc(1, sizeof(browser_tab_t));
    if (!tab) return NULL;
    
    // Initialize tab
    tab->id = engine->tabs.tab_count;
    tab->url = strdup("about:blank");
    tab->title = strdup("New Tab");
    
    // Create JavaScript context for tab
    tab->js_context = js_engine_create(64 * 1024 * 1024); // 64MB heap per tab
    if (!tab->js_context) {
        free(tab->url);
        free(tab->title);
        free(tab);
        return NULL;
    }
    
    // Create empty document
    tab->document = dom_document_create();
    if (!tab->document) {
        free(tab->url);
        free(tab->title);
        free(tab->js_context);
        free(tab);
        return NULL;
    }
    
    // Bind DOM to JavaScript
    js_bind_dom(tab->js_context, tab->document);
    
    // Initialize navigation history
    tab->navigation.history = calloc(100, sizeof(void*));
    tab->navigation.history_index = 0;
    tab->navigation.history_count = 0;
    
    // Add tab to engine
    engine->tabs.tabs[engine->tabs.tab_count] = tab;
    engine->tabs.tab_count++;
    engine->tabs.active_tab = engine->tabs.tab_count - 1;
    
    return tab;
}

// Navigate to URL
int browser_navigate(browser_tab_t* tab, const char* url) {
    if (!tab || !url) return -1;
    
    // Update tab state
    tab->state.loading = true;
    tab->state.progress = 0;
    
    // Parse URL and determine protocol
    bool is_secure = strncmp(url, "https://", 8) == 0;
    tab->state.secure = is_secure;
    
    // Free old URL and set new one
    free(tab->url);
    tab->url = strdup(url);
    
    // Add to history
    if (tab->navigation.history_index < tab->navigation.history_count - 1) {
        // Clear forward history
        for (uint32_t i = tab->navigation.history_index + 1; i < tab->navigation.history_count; i++) {
            free(tab->navigation.history[i]);
        }
        tab->navigation.history_count = tab->navigation.history_index + 1;
    }
    
    if (tab->navigation.history_count < 100) {
        tab->navigation.history[tab->navigation.history_count] = strdup(url);
        tab->navigation.history_count++;
        tab->navigation.history_index = tab->navigation.history_count - 1;
    }
    
    // Fetch the resource
    request_t* request = fetch_create_request(url, NULL);
    if (!request) {
        tab->state.loading = false;
        return -1;
    }
    
    // Start fetch operation
    fetch_operation_t* fetch_op = fetch_start(request);
    if (!fetch_op) {
        free(request);
        tab->state.loading = false;
        return -1;
    }
    
    // Wait for response (simplified - should be async)
    // In real implementation, this would be handled asynchronously
    response_t* response = fetch_op->response;
    if (response && response->ok) {
        // Parse HTML content
        char* html_content = (char*)response->body;
        browser_load_html(tab, html_content);
    }
    
    fetch_operation_destroy(fetch_op);
    tab->state.loading = false;
    tab->state.progress = 100;
    
    return 0;
}

// Load HTML content
int browser_load_html(browser_tab_t* tab, const char* html) {
    if (!tab || !html) return -1;
    
    // Parse HTML
    html_parser_t* parser = html_parser_create();
    if (!parser) return -1;
    
    // Free old document
    if (tab->document) {
        dom_document_destroy(tab->document);
    }
    
    // Parse new document
    tab->document = html_parse(parser, html, strlen(html));
    html_parser_destroy(parser);
    
    if (!tab->document) return -1;
    
    // Update title from document
    dom_element_t* title_elem = dom_element_query_selector(tab->document->head, "title");
    if (title_elem) {
        char* title_text = dom_node_get_text_content((dom_node_t*)title_elem);
        if (title_text) {
            free(tab->title);
            tab->title = title_text;
        }
    }
    
    // Bind new DOM to JavaScript
    js_bind_dom(tab->js_context, tab->document);
    
    // Process scripts
    uint32_t script_count;
    dom_element_t** scripts = dom_element_get_by_tag_name(
        tab->document->document_element, "script", &script_count
    );
    
    for (uint32_t i = 0; i < script_count; i++) {
        char* script_src = dom_element_get_attribute(scripts[i], "src");
        if (script_src) {
            // Fetch external script
            request_t* script_request = fetch_create_request(script_src, NULL);
            fetch_operation_t* script_fetch = fetch_start(script_request);
            if (script_fetch && script_fetch->response && script_fetch->response->ok) {
                browser_execute_script(tab, (char*)script_fetch->response->body);
            }
            fetch_operation_destroy(script_fetch);
        } else {
            // Execute inline script
            char* script_content = dom_node_get_text_content((dom_node_t*)scripts[i]);
            if (script_content) {
                browser_execute_script(tab, script_content);
                free(script_content);
            }
        }
    }
    
    free(scripts);
    
    // Build render tree
    if (tab->render_tree) {
        free(tab->render_tree);
    }
    
    render_pipeline_t* pipeline = (render_pipeline_t*)tab->render_tree;
    tab->render_tree = pipeline->layout.build_render_tree(tab->document->document_element);
    
    return 0;
}

// Execute JavaScript
int browser_execute_script(browser_tab_t* tab, const char* script) {
    if (!tab || !script || !tab->js_context) return -1;
    
    // Check CSP
    csp_policy_t* csp = NULL; // Would get from response headers
    if (csp && !csp_allows_eval(csp)) {
        printf("CSP: Script execution blocked\n");
        return -1;
    }
    
    // Execute script
    js_value_t* result = js_eval(tab->js_context, script, tab->url);
    
    if (result) {
        // Handle result if needed
        js_value_release(result);
    }
    
    return 0;
}

// Render frame
void browser_render_frame(browser_engine_t* engine) {
    if (!engine || engine->tabs.tab_count == 0) return;
    
    browser_tab_t* active_tab = browser_get_active_tab(engine);
    if (!active_tab || !active_tab->render_tree) return;
    
    render_pipeline_t* pipeline = (render_pipeline_t*)engine->parsers.render_engine;
    render_tree_t* tree = active_tab->render_tree;
    
    // Compute layout
    pipeline->layout.compute_layout(tree, 1920.0f, 1080.0f); // Viewport size
    
    // Paint
    browser_paint(active_tab);
    
    // Composite layers
    browser_composite(engine);
    
    // Present to screen
    browser_present(engine);
    
    // Update stats
    engine->stats.frame_rate = 60; // Target 60 FPS
}

// Paint tab content
void browser_paint(browser_tab_t* tab) {
    if (!tab || !tab->render_tree) return;
    
    render_tree_t* tree = tab->render_tree;
    render_pipeline_t* pipeline = (render_pipeline_t*)tree;
    
    // Build layer tree
    paint_layer_t* root_layer = pipeline->paint.build_layer_tree(tree);
    
    // Paint each layer
    display_list_t* display_list = pipeline->paint.paint(root_layer);
    
    // Execute display list
    void* raster_context = pipeline->raster.create_context(1920, 1080);
    pipeline->raster.execute_display_list(raster_context, display_list);
    pipeline->raster.flush(raster_context);
    
    destroy_display_list(display_list);
}

// Get active tab
browser_tab_t* browser_get_active_tab(browser_engine_t* engine) {
    if (!engine || engine->tabs.tab_count == 0) return NULL;
    if (engine->tabs.active_tab >= engine->tabs.tab_count) return NULL;
    return engine->tabs.tabs[engine->tabs.active_tab];
}

// Close tab
void browser_close_tab(browser_engine_t* engine, uint32_t tab_id) {
    if (!engine) return;
    
    // Find tab by ID
    browser_tab_t* tab = NULL;
    uint32_t tab_index = 0;
    for (uint32_t i = 0; i < engine->tabs.tab_count; i++) {
        if (engine->tabs.tabs[i]->id == tab_id) {
            tab = engine->tabs.tabs[i];
            tab_index = i;
            break;
        }
    }
    
    if (!tab) return;
    
    // Free tab resources
    free(tab->url);
    free(tab->title);
    if (tab->document) dom_document_destroy(tab->document);
    if (tab->js_context) js_engine_destroy(tab->js_context);
    if (tab->render_tree) free(tab->render_tree);
    
    // Free navigation history
    for (uint32_t i = 0; i < tab->navigation.history_count; i++) {
        free(tab->navigation.history[i]);
    }
    free(tab->navigation.history);
    
    free(tab);
    
    // Remove from tabs array
    for (uint32_t i = tab_index; i < engine->tabs.tab_count - 1; i++) {
        engine->tabs.tabs[i] = engine->tabs.tabs[i + 1];
    }
    engine->tabs.tab_count--;
    
    // Adjust active tab if necessary
    if (engine->tabs.active_tab >= engine->tabs.tab_count && engine->tabs.tab_count > 0) {
        engine->tabs.active_tab = engine->tabs.tab_count - 1;
    }
}

// Browser back navigation
int browser_go_back(browser_tab_t* tab) {
    if (!tab || tab->navigation.history_index == 0) return -1;
    
    tab->navigation.history_index--;
    const char* url = tab->navigation.history[tab->navigation.history_index];
    
    // Navigate without adding to history
    tab->state.loading = true;
    free(tab->url);
    tab->url = strdup(url);
    
    // Load the page
    request_t* request = fetch_create_request(url, NULL);
    fetch_operation_t* fetch_op = fetch_start(request);
    if (fetch_op && fetch_op->response && fetch_op->response->ok) {
        browser_load_html(tab, (char*)fetch_op->response->body);
    }
    fetch_operation_destroy(fetch_op);
    
    tab->state.loading = false;
    return 0;
}

// Browser forward navigation
int browser_go_forward(browser_tab_t* tab) {
    if (!tab || tab->navigation.history_index >= tab->navigation.history_count - 1) return -1;
    
    tab->navigation.history_index++;
    const char* url = tab->navigation.history[tab->navigation.history_index];
    
    // Navigate without adding to history
    tab->state.loading = true;
    free(tab->url);
    tab->url = strdup(url);
    
    // Load the page
    request_t* request = fetch_create_request(url, NULL);
    fetch_operation_t* fetch_op = fetch_start(request);
    if (fetch_op && fetch_op->response && fetch_op->response->ok) {
        browser_load_html(tab, (char*)fetch_op->response->body);
    }
    fetch_operation_destroy(fetch_op);
    
    tab->state.loading = false;
    return 0;
}

// Reload current page
int browser_reload(browser_tab_t* tab) {
    if (!tab || !tab->url) return -1;
    
    // Clear cache for this URL
    // cache_delete(tab->url);
    
    // Reload
    return browser_navigate(tab, tab->url);
}

// Stop loading
void browser_stop(browser_tab_t* tab) {
    if (!tab) return;
    tab->state.loading = false;
    tab->state.progress = 0;
    // Cancel any pending network requests
}

// Composite layers
void browser_composite(browser_engine_t* engine) {
    if (!engine) return;
    
    render_pipeline_t* pipeline = (render_pipeline_t*)engine->parsers.render_engine;
    
    // Collect all layers from active tab
    browser_tab_t* active_tab = browser_get_active_tab(engine);
    if (!active_tab || !active_tab->render_tree) return;
    
    // Composite layers using GPU if available
    if (engine->config.enable_gpu && pipeline->acceleration.enabled) {
        // GPU compositing
        // pipeline->compositor.composite(layers, layer_count);
    } else {
        // Software compositing
    }
}

// Present frame to screen
void browser_present(browser_engine_t* engine) {
    if (!engine) return;
    
    render_pipeline_t* pipeline = (render_pipeline_t*)engine->parsers.render_engine;
    
    // Present the composited frame
    // pipeline->raster.present(raster_context);
    
    // Update frame counter
    static uint64_t frame_count = 0;
    frame_count++;
}

// Shutdown browser engine
void browser_engine_shutdown(browser_engine_t* engine) {
    if (!engine) return;
    
    // Close all tabs
    while (engine->tabs.tab_count > 0) {
        browser_close_tab(engine, engine->tabs.tabs[0]->id);
    }
    
    // Shutdown JavaScript engine
    if (engine->parsers.js_engine) {
        js_engine_shutdown(engine->parsers.js_engine);
        js_engine_destroy(engine->parsers.js_engine);
    }
    
    // Free parsers
    if (engine->parsers.html_parser) {
        html_parser_destroy(engine->parsers.html_parser);
    }
    
    free(engine->parsers.css_parser);
    free(engine->parsers.render_engine);
    
    // Free managers
    free(engine->managers.network_manager);
    free(engine->managers.cache_manager);
    free(engine->managers.security_manager);
    free(engine->managers.extension_manager);
}

// Destroy browser engine
void browser_engine_destroy(browser_engine_t* engine) {
    if (!engine) return;
    
    browser_engine_shutdown(engine);
    free(engine->tabs.tabs);
    free(engine);
}

// Developer tools
void browser_enable_devtools(browser_engine_t* engine, bool enable) {
    if (!engine) return;
    // Enable/disable developer tools
}

void browser_inspect_element(browser_tab_t* tab, uint32_t x, uint32_t y) {
    if (!tab || !tab->render_tree) return;
    
    render_pipeline_t* pipeline = (render_pipeline_t*)tab->render_tree;
    layout_box_t* box = NULL;
    
    // Hit test to find element at coordinates
    if (pipeline->layout.hit_test(tab->render_tree, (float)x, (float)y, &box)) {
        if (box && box->element) {
            // Highlight element and show properties
            printf("Inspecting element: %s\n", box->element->tag_name);
        }
    }
}

void browser_show_console(browser_tab_t* tab) {
    if (!tab) return;
    // Show JavaScript console
}

void browser_profile_start(browser_engine_t* engine) {
    if (!engine) return;
    // Start performance profiling
}

void browser_profile_stop(browser_engine_t* engine) {
    if (!engine) return;
    // Stop profiling and show results
}