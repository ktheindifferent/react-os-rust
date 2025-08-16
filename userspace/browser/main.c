#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include "ui/window.h"
#include "../../kernel/src/browser/engine.h"

// Global browser instance
static browser_window_t* main_window = NULL;
static browser_engine_t* browser_engine = NULL;

// Signal handler for cleanup
void signal_handler(int sig) {
    if (sig == SIGINT || sig == SIGTERM) {
        printf("\nShutting down browser...\n");
        if (main_window) {
            browser_window_destroy(main_window);
        }
        if (browser_engine) {
            browser_engine_destroy(browser_engine);
        }
        exit(0);
    }
}

// Print usage information
void print_usage(const char* program_name) {
    printf("Usage: %s [options] [URL]\n", program_name);
    printf("\nOptions:\n");
    printf("  -h, --help           Show this help message\n");
    printf("  -v, --version        Show version information\n");
    printf("  -p, --private        Start in private browsing mode\n");
    printf("  -f, --fullscreen     Start in fullscreen mode\n");
    printf("  --width=<WIDTH>      Set window width (default: 1280)\n");
    printf("  --height=<HEIGHT>    Set window height (default: 720)\n");
    printf("  --profile=<PATH>     Use specified profile directory\n");
    printf("  --no-sandbox         Disable sandbox (not recommended)\n");
    printf("  --disable-gpu        Disable GPU acceleration\n");
    printf("  --disable-js         Disable JavaScript\n");
    printf("  --user-agent=<UA>    Set custom user agent\n");
    printf("  --proxy=<PROXY>      Use proxy server\n");
    printf("  --devtools           Open with developer tools\n");
    printf("\nExamples:\n");
    printf("  %s https://example.com\n", program_name);
    printf("  %s --private https://example.com\n", program_name);
    printf("  %s --width=1920 --height=1080 --fullscreen\n", program_name);
}

// Parse command line arguments
typedef struct {
    char* initial_url;
    bool private_mode;
    bool fullscreen;
    uint32_t width;
    uint32_t height;
    char* profile_path;
    bool no_sandbox;
    bool disable_gpu;
    bool disable_js;
    char* user_agent;
    char* proxy;
    bool show_devtools;
} browser_options_t;

browser_options_t parse_arguments(int argc, char* argv[]) {
    browser_options_t options = {
        .initial_url = NULL,
        .private_mode = false,
        .fullscreen = false,
        .width = 1280,
        .height = 720,
        .profile_path = NULL,
        .no_sandbox = false,
        .disable_gpu = false,
        .disable_js = false,
        .user_agent = NULL,
        .proxy = NULL,
        .show_devtools = false
    };
    
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "-h") == 0 || strcmp(argv[i], "--help") == 0) {
            print_usage(argv[0]);
            exit(0);
        } else if (strcmp(argv[i], "-v") == 0 || strcmp(argv[i], "--version") == 0) {
            printf("Web Browser Engine v1.0.0\n");
            printf("HTML5, CSS3, JavaScript ES2023+\n");
            printf("WebGL, WebRTC, WebAssembly support\n");
            exit(0);
        } else if (strcmp(argv[i], "-p") == 0 || strcmp(argv[i], "--private") == 0) {
            options.private_mode = true;
        } else if (strcmp(argv[i], "-f") == 0 || strcmp(argv[i], "--fullscreen") == 0) {
            options.fullscreen = true;
        } else if (strncmp(argv[i], "--width=", 8) == 0) {
            options.width = atoi(argv[i] + 8);
        } else if (strncmp(argv[i], "--height=", 9) == 0) {
            options.height = atoi(argv[i] + 9);
        } else if (strncmp(argv[i], "--profile=", 10) == 0) {
            options.profile_path = argv[i] + 10;
        } else if (strcmp(argv[i], "--no-sandbox") == 0) {
            options.no_sandbox = true;
        } else if (strcmp(argv[i], "--disable-gpu") == 0) {
            options.disable_gpu = true;
        } else if (strcmp(argv[i], "--disable-js") == 0) {
            options.disable_js = true;
        } else if (strncmp(argv[i], "--user-agent=", 13) == 0) {
            options.user_agent = argv[i] + 13;
        } else if (strncmp(argv[i], "--proxy=", 8) == 0) {
            options.proxy = argv[i] + 8;
        } else if (strcmp(argv[i], "--devtools") == 0) {
            options.show_devtools = true;
        } else if (argv[i][0] != '-') {
            // Assume it's a URL
            options.initial_url = argv[i];
        }
    }
    
    // Default URL if none provided
    if (!options.initial_url) {
        options.initial_url = "about:blank";
    }
    
    return options;
}

// Initialize browser engine
browser_engine_t* init_browser_engine(browser_options_t* options) {
    browser_config_t config = {
        .max_tabs = 100,
        .js_heap_size = 256 * 1024 * 1024,
        .cache_size = 100 * 1024 * 1024,
        .enable_gpu = !options->disable_gpu,
        .enable_webgl = !options->disable_gpu,
        .enable_webrtc = true,
        .enable_sandbox = !options->no_sandbox,
        .max_workers = 4
    };
    
    browser_engine_t* engine = browser_engine_create(&config);
    if (!engine) {
        fprintf(stderr, "Failed to create browser engine\n");
        return NULL;
    }
    
    if (browser_engine_init(engine) != 0) {
        fprintf(stderr, "Failed to initialize browser engine\n");
        browser_engine_destroy(engine);
        return NULL;
    }
    
    return engine;
}

// Initialize UI
browser_window_t* init_browser_ui(browser_options_t* options, browser_engine_t* engine) {
    browser_window_t* window;
    
    if (options->private_mode) {
        window = browser_ui_new_private_window();
    } else {
        window = browser_window_create(options->width, options->height);
    }
    
    if (!window) {
        fprintf(stderr, "Failed to create browser window\n");
        return NULL;
    }
    
    window->engine = engine;
    
    // Set window state
    if (options->fullscreen) {
        browser_window_set_state(window, WINDOW_STATE_FULLSCREEN);
    }
    
    // Apply settings
    browser_settings_t settings = {
        .javascript_enabled = !options->disable_js,
        .developer_mode = options->show_devtools,
        .show_devtools = options->show_devtools,
        .proxy_server = options->proxy,
        .proxy_enabled = (options->proxy != NULL),
        .user_agent = options->user_agent
    };
    browser_ui_apply_settings(window, &settings);
    
    // Show developer tools if requested
    if (options->show_devtools) {
        browser_ui_show_devtools(window);
    }
    
    return window;
}

// Main event loop
void run_event_loop(browser_window_t* window, browser_engine_t* engine) {
    printf("Browser started. Press Ctrl+C to exit.\n");
    
    // Main browser loop
    while (1) {
        // Process UI events (simplified - would use proper event system)
        // browser_ui_process_events(window);
        
        // Run JavaScript event loop for active tab
        browser_tab_t* active_tab = browser_get_active_tab(engine);
        if (active_tab && active_tab->js_context) {
            js_run_event_loop(active_tab->js_context);
        }
        
        // Render frame
        browser_render_frame(engine);
        
        // Sleep to maintain 60 FPS
        usleep(16666); // ~60 FPS
    }
}

// Main function
int main(int argc, char* argv[]) {
    printf("Web Browser Engine Starting...\n");
    
    // Set up signal handlers
    signal(SIGINT, signal_handler);
    signal(SIGTERM, signal_handler);
    
    // Parse command line arguments
    browser_options_t options = parse_arguments(argc, argv);
    
    // Initialize browser engine
    browser_engine = init_browser_engine(&options);
    if (!browser_engine) {
        return 1;
    }
    
    // Initialize UI
    main_window = init_browser_ui(&options, browser_engine);
    if (!main_window) {
        browser_engine_destroy(browser_engine);
        return 1;
    }
    
    // Create initial tab
    browser_tab_t* initial_tab = browser_create_tab(browser_engine);
    if (!initial_tab) {
        fprintf(stderr, "Failed to create initial tab\n");
        browser_window_destroy(main_window);
        browser_engine_destroy(browser_engine);
        return 1;
    }
    
    // Navigate to initial URL
    if (options.initial_url && strcmp(options.initial_url, "about:blank") != 0) {
        printf("Navigating to: %s\n", options.initial_url);
        browser_navigate(initial_tab, options.initial_url);
    }
    
    // Show window
    browser_window_show(main_window);
    
    // Run main event loop
    run_event_loop(main_window, browser_engine);
    
    // Cleanup (reached on signal)
    browser_window_destroy(main_window);
    browser_engine_destroy(browser_engine);
    
    return 0;
}

// Browser keyboard shortcuts handler
void handle_keyboard_shortcut(browser_window_t* window, int key, int modifiers) {
    if (!window || !window->engine) return;
    
    browser_tab_t* active_tab = browser_get_active_tab(window->engine);
    
    // Ctrl key shortcuts
    if (modifiers & 1) { // CTRL
        switch (key) {
            case 't': // New tab
                browser_ui_create_tab(window);
                break;
            case 'w': // Close tab
                if (active_tab) {
                    browser_close_tab(window->engine, active_tab->id);
                }
                break;
            case 'l': // Focus address bar
                browser_ui_focus_address_bar(window);
                break;
            case 'r': // Reload
                if (active_tab) {
                    browser_reload(active_tab);
                }
                break;
            case 'd': // Bookmark
                if (active_tab) {
                    browser_ui_add_bookmark(window, active_tab->url, active_tab->title);
                }
                break;
            case 'h': // History
                browser_ui_show_history(window);
                break;
            case 'j': // Downloads
                browser_ui_show_downloads(window);
                break;
            case 'f': // Find
                browser_ui_show_find_bar(window);
                break;
            case 'p': // Print
                browser_ui_print(window);
                break;
            case '+': // Zoom in
                browser_ui_zoom_in(window);
                break;
            case '-': // Zoom out
                browser_ui_zoom_out(window);
                break;
            case '0': // Reset zoom
                browser_ui_zoom_reset(window);
                break;
        }
    }
    
    // Alt key shortcuts
    if (modifiers & 2) { // ALT
        switch (key) {
            case 263: // Left arrow - Back
                if (active_tab) {
                    browser_go_back(active_tab);
                }
                break;
            case 262: // Right arrow - Forward
                if (active_tab) {
                    browser_go_forward(active_tab);
                }
                break;
            case 36: // Home
                browser_ui_go_home(window);
                break;
        }
    }
    
    // F-key shortcuts
    switch (key) {
        case 282: // F1 - Help
            browser_navigate(active_tab, "about:help");
            break;
        case 284: // F3 - Find next
            browser_ui_find_next(window, NULL);
            break;
        case 293: // F5 - Reload
            if (active_tab) {
                browser_reload(active_tab);
            }
            break;
        case 122: // F11 - Fullscreen
            if (window->state == WINDOW_STATE_FULLSCREEN) {
                browser_ui_exit_fullscreen(window);
            } else {
                browser_ui_enter_fullscreen(window);
            }
            break;
        case 123: // F12 - Developer tools
            browser_ui_toggle_devtools(window);
            break;
    }
}

// Mouse event handler
void handle_mouse_event(browser_window_t* window, int button, int action, int x, int y) {
    if (!window || !window->engine) return;
    
    browser_tab_t* active_tab = browser_get_active_tab(window->engine);
    if (!active_tab) return;
    
    // Right click - context menu
    if (button == 1 && action == 1) { // Right button pressed
        context_menu_t* menu = browser_ui_create_context_menu(window, x, y);
        browser_ui_show_context_menu(window, menu);
    }
    
    // Middle click on link - open in new tab
    if (button == 2 && action == 1) { // Middle button pressed
        // Hit test to find link
        // If link found, open in new tab
    }
}