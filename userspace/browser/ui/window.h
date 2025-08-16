#ifndef BROWSER_UI_WINDOW_H
#define BROWSER_UI_WINDOW_H

#include <stdint.h>
#include <stdbool.h>

// Forward declarations
typedef struct browser_engine browser_engine_t;
typedef struct browser_tab browser_tab_t;

// Window state
typedef enum {
    WINDOW_STATE_NORMAL,
    WINDOW_STATE_MINIMIZED,
    WINDOW_STATE_MAXIMIZED,
    WINDOW_STATE_FULLSCREEN
} window_state_t;

// Browser window
typedef struct browser_window {
    // Window properties
    uint32_t id;
    char* title;
    uint32_t width;
    uint32_t height;
    uint32_t x;
    uint32_t y;
    window_state_t state;
    bool visible;
    bool focused;
    
    // Browser engine
    browser_engine_t* engine;
    
    // UI components
    struct {
        void* toolbar;
        void* tabbar;
        void* viewport;
        void* statusbar;
        void* sidebar;
        void* devtools;
    } components;
    
    // Tabs
    struct {
        browser_tab_t** tabs;
        uint32_t tab_count;
        uint32_t active_tab_index;
    } tabs;
    
    // Native window handle
    void* native_handle;
} browser_window_t;

// UI toolbar
typedef struct {
    // Navigation buttons
    struct {
        bool back_enabled;
        bool forward_enabled;
        bool reload_visible;
        bool stop_visible;
    } navigation;
    
    // Address bar
    struct {
        char* url;
        char* display_url;
        bool secure;
        bool editing;
        struct {
            char** suggestions;
            uint32_t suggestion_count;
            int32_t selected_index;
        } autocomplete;
    } address_bar;
    
    // Buttons
    struct {
        bool menu_visible;
        bool downloads_active;
        bool extensions_visible;
        bool profile_visible;
    } buttons;
} browser_toolbar_t;

// Tab bar
typedef struct {
    struct {
        uint32_t id;
        char* title;
        char* url;
        void* favicon;
        bool loading;
        bool pinned;
        bool muted;
        bool active;
    }* tabs;
    uint32_t tab_count;
    bool show_add_button;
    uint32_t max_width;
} browser_tabbar_t;

// Status bar
typedef struct {
    char* status_text;
    char* hover_link;
    struct {
        uint32_t percent;
        bool visible;
    } loading;
    struct {
        float zoom_level;
        bool visible;
    } zoom;
} browser_statusbar_t;

// Context menu
typedef enum {
    MENU_ITEM_BACK,
    MENU_ITEM_FORWARD,
    MENU_ITEM_RELOAD,
    MENU_ITEM_CUT,
    MENU_ITEM_COPY,
    MENU_ITEM_PASTE,
    MENU_ITEM_SELECT_ALL,
    MENU_ITEM_SAVE_AS,
    MENU_ITEM_PRINT,
    MENU_ITEM_VIEW_SOURCE,
    MENU_ITEM_INSPECT,
    MENU_ITEM_COPY_LINK,
    MENU_ITEM_OPEN_LINK_NEW_TAB,
    MENU_ITEM_SAVE_IMAGE,
    MENU_ITEM_COPY_IMAGE,
    MENU_ITEM_SEPARATOR
} menu_item_type_t;

typedef struct {
    menu_item_type_t type;
    char* label;
    char* shortcut;
    bool enabled;
    bool checked;
    void (*handler)(void* data);
    void* handler_data;
} menu_item_t;

typedef struct {
    menu_item_t** items;
    uint32_t item_count;
    uint32_t x;
    uint32_t y;
} context_menu_t;

// Download manager
typedef enum {
    DOWNLOAD_STATE_PENDING,
    DOWNLOAD_STATE_IN_PROGRESS,
    DOWNLOAD_STATE_PAUSED,
    DOWNLOAD_STATE_COMPLETED,
    DOWNLOAD_STATE_CANCELLED,
    DOWNLOAD_STATE_FAILED
} download_state_t;

typedef struct {
    uint32_t id;
    char* url;
    char* filename;
    char* path;
    uint64_t total_bytes;
    uint64_t received_bytes;
    download_state_t state;
    uint32_t speed;
    uint32_t time_remaining;
    char* mime_type;
    bool dangerous;
} download_item_t;

typedef struct {
    download_item_t** items;
    uint32_t item_count;
    void (*on_download_start)(download_item_t* item);
    void (*on_download_progress)(download_item_t* item);
    void (*on_download_complete)(download_item_t* item);
} download_manager_t;

// History manager
typedef struct {
    uint32_t id;
    char* url;
    char* title;
    uint64_t visit_time;
    uint32_t visit_count;
    void* favicon;
} history_entry_t;

typedef struct {
    history_entry_t** entries;
    uint32_t entry_count;
    uint32_t max_entries;
    bool incognito_mode;
} history_manager_t;

// Bookmark manager
typedef struct bookmark {
    uint32_t id;
    char* title;
    char* url;
    uint64_t created_time;
    uint64_t modified_time;
    struct bookmark* parent;
    struct bookmark** children;
    uint32_t child_count;
    bool is_folder;
    void* favicon;
} bookmark_t;

typedef struct {
    bookmark_t* root;
    bookmark_t* bookmarks_bar;
    bookmark_t* other_bookmarks;
    bookmark_t* mobile_bookmarks;
} bookmark_manager_t;

// Password manager
typedef struct {
    char* origin;
    char* username;
    char* password;
    uint64_t created_time;
    uint64_t last_used_time;
    uint32_t times_used;
} password_entry_t;

typedef struct {
    password_entry_t** entries;
    uint32_t entry_count;
    bool enabled;
    bool auto_signin;
    uint8_t master_key[32];
} password_manager_t;

// Settings
typedef struct {
    // General
    char* homepage;
    bool restore_on_startup;
    char** startup_urls;
    uint32_t startup_url_count;
    
    // Privacy
    bool do_not_track;
    bool send_referer;
    bool save_passwords;
    bool autofill_enabled;
    enum {
        COOKIE_ALLOW_ALL,
        COOKIE_BLOCK_THIRD_PARTY,
        COOKIE_BLOCK_ALL
    } cookie_policy;
    
    // Content
    bool javascript_enabled;
    bool images_enabled;
    bool plugins_enabled;
    bool popups_blocked;
    char* default_font;
    uint32_t default_font_size;
    char* default_encoding;
    
    // Network
    char* proxy_server;
    uint32_t proxy_port;
    bool proxy_enabled;
    char* user_agent;
    uint32_t cache_size;
    
    // Developer
    bool developer_mode;
    bool show_devtools;
    bool disable_cache;
} browser_settings_t;

// Window management
browser_window_t* browser_window_create(uint32_t width, uint32_t height);
void browser_window_destroy(browser_window_t* window);
void browser_window_show(browser_window_t* window);
void browser_window_hide(browser_window_t* window);
void browser_window_set_title(browser_window_t* window, const char* title);
void browser_window_set_state(browser_window_t* window, window_state_t state);
void browser_window_resize(browser_window_t* window, uint32_t width, uint32_t height);
void browser_window_move(browser_window_t* window, uint32_t x, uint32_t y);

// Tab management UI
void browser_ui_create_tab(browser_window_t* window);
void browser_ui_close_tab(browser_window_t* window, uint32_t tab_index);
void browser_ui_switch_tab(browser_window_t* window, uint32_t tab_index);
void browser_ui_move_tab(browser_window_t* window, uint32_t from, uint32_t to);
void browser_ui_duplicate_tab(browser_window_t* window, uint32_t tab_index);
void browser_ui_pin_tab(browser_window_t* window, uint32_t tab_index, bool pinned);

// Navigation UI
void browser_ui_navigate(browser_window_t* window, const char* url);
void browser_ui_go_back(browser_window_t* window);
void browser_ui_go_forward(browser_window_t* window);
void browser_ui_reload(browser_window_t* window);
void browser_ui_stop(browser_window_t* window);
void browser_ui_go_home(browser_window_t* window);

// Address bar
void browser_ui_focus_address_bar(browser_window_t* window);
void browser_ui_update_address_bar(browser_window_t* window, const char* url);
void browser_ui_show_autocomplete(browser_window_t* window, char** suggestions, uint32_t count);
void browser_ui_hide_autocomplete(browser_window_t* window);

// Context menu
context_menu_t* browser_ui_create_context_menu(browser_window_t* window, uint32_t x, uint32_t y);
void browser_ui_show_context_menu(browser_window_t* window, context_menu_t* menu);
void browser_ui_hide_context_menu(browser_window_t* window);
void context_menu_destroy(context_menu_t* menu);

// Downloads UI
void browser_ui_show_downloads(browser_window_t* window);
void browser_ui_hide_downloads(browser_window_t* window);
void browser_ui_add_download(browser_window_t* window, download_item_t* item);
void browser_ui_update_download(browser_window_t* window, download_item_t* item);

// History UI
void browser_ui_show_history(browser_window_t* window);
void browser_ui_add_history_entry(browser_window_t* window, history_entry_t* entry);
void browser_ui_clear_history(browser_window_t* window);

// Bookmarks UI
void browser_ui_show_bookmarks(browser_window_t* window);
void browser_ui_add_bookmark(browser_window_t* window, const char* url, const char* title);
void browser_ui_edit_bookmark(browser_window_t* window, bookmark_t* bookmark);
void browser_ui_remove_bookmark(browser_window_t* window, bookmark_t* bookmark);

// Settings UI
void browser_ui_show_settings(browser_window_t* window);
void browser_ui_apply_settings(browser_window_t* window, browser_settings_t* settings);
browser_settings_t* browser_ui_get_settings(browser_window_t* window);

// Find in page
void browser_ui_show_find_bar(browser_window_t* window);
void browser_ui_hide_find_bar(browser_window_t* window);
void browser_ui_find_next(browser_window_t* window, const char* text);
void browser_ui_find_previous(browser_window_t* window, const char* text);

// Notifications
typedef enum {
    NOTIFICATION_INFO,
    NOTIFICATION_WARNING,
    NOTIFICATION_ERROR,
    NOTIFICATION_SUCCESS
} notification_type_t;

void browser_ui_show_notification(browser_window_t* window, const char* message, notification_type_t type);
void browser_ui_show_permission_prompt(browser_window_t* window, const char* origin, const char* permission);

// Print preview
void browser_ui_show_print_preview(browser_window_t* window);
void browser_ui_print(browser_window_t* window);

// Full screen
void browser_ui_enter_fullscreen(browser_window_t* window);
void browser_ui_exit_fullscreen(browser_window_t* window);

// Zoom
void browser_ui_zoom_in(browser_window_t* window);
void browser_ui_zoom_out(browser_window_t* window);
void browser_ui_zoom_reset(browser_window_t* window);
void browser_ui_set_zoom(browser_window_t* window, float level);

// Developer tools UI
void browser_ui_show_devtools(browser_window_t* window);
void browser_ui_hide_devtools(browser_window_t* window);
void browser_ui_toggle_devtools(browser_window_t* window);
void browser_ui_devtools_inspect_element(browser_window_t* window, uint32_t x, uint32_t y);

// View source
void browser_ui_view_source(browser_window_t* window);
void browser_ui_view_source_selection(browser_window_t* window);

// Private browsing
browser_window_t* browser_ui_new_private_window(void);
bool browser_ui_is_private(browser_window_t* window);

#endif