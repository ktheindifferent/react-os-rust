#ifndef RENDER_ENGINE_H
#define RENDER_ENGINE_H

#include <stdint.h>
#include <stdbool.h>

// Forward declarations
struct dom_element;
struct css_computed_style;

// Layout box types
typedef enum {
    BOX_BLOCK,
    BOX_INLINE,
    BOX_INLINE_BLOCK,
    BOX_TABLE,
    BOX_TABLE_ROW,
    BOX_TABLE_CELL,
    BOX_FLEX,
    BOX_GRID,
    BOX_TEXT,
    BOX_REPLACED,
    BOX_ANONYMOUS
} layout_box_type_t;

// Rectangle structure
typedef struct {
    float x, y, width, height;
} rect_t;

// Layout box
typedef struct layout_box {
    layout_box_type_t type;
    struct dom_element* element;
    struct css_computed_style* style;
    
    // Geometry
    rect_t content_rect;
    rect_t padding_rect;
    rect_t border_rect;
    rect_t margin_rect;
    
    // Box model
    struct {
        float top, right, bottom, left;
    } margin, padding, border;
    
    // Positioning
    struct {
        float x, y;
    } position;
    bool is_positioned;
    bool is_floating;
    
    // Tree structure
    struct layout_box* parent;
    struct layout_box* first_child;
    struct layout_box* last_child;
    struct layout_box* prev_sibling;
    struct layout_box* next_sibling;
    uint32_t child_count;
    
    // Text specific
    struct {
        char* text;
        uint32_t length;
        struct {
            uint32_t start;
            uint32_t end;
            float width;
            float height;
        }* runs;
        uint32_t run_count;
    } text_data;
    
    // Flex container
    struct {
        enum {
            FLEX_ROW, FLEX_ROW_REVERSE,
            FLEX_COLUMN, FLEX_COLUMN_REVERSE
        } direction;
        bool wrap;
        float main_size;
        float cross_size;
        struct {
            struct layout_box* box;
            float flex_grow;
            float flex_shrink;
            float flex_basis;
            float main_size;
            float cross_size;
        }* items;
        uint32_t item_count;
    } flex;
    
    // Grid container
    struct {
        struct {
            float size;
            bool is_fr;
        }* columns;
        uint32_t column_count;
        struct {
            float size;
            bool is_fr;
        }* rows;
        uint32_t row_count;
        struct {
            struct layout_box* box;
            uint32_t column_start, column_end;
            uint32_t row_start, row_end;
        }* items;
        uint32_t item_count;
    } grid;
    
    // Painting
    bool needs_paint;
    uint32_t paint_order;
    struct {
        bool has_transform;
        float transform_matrix[16];
        float opacity;
        bool has_filter;
    } paint_properties;
} layout_box_t;

// Render tree
typedef struct {
    layout_box_t* root;
    uint32_t box_count;
    bool needs_layout;
    bool needs_paint;
    uint64_t layout_version;
    uint64_t paint_version;
} render_tree_t;

// Paint layer
typedef struct paint_layer {
    layout_box_t* box;
    rect_t bounds;
    bool is_composited;
    bool needs_repaint;
    struct paint_layer* parent;
    struct paint_layer** children;
    uint32_t child_count;
    
    // Compositing
    struct {
        uint32_t texture_id;
        float transform[16];
        float opacity;
        enum {
            BLEND_NORMAL,
            BLEND_MULTIPLY,
            BLEND_SCREEN,
            BLEND_OVERLAY
        } blend_mode;
    } compositing;
    
    // Stacking context
    int32_t z_index;
    bool creates_stacking_context;
    
    // Clipping
    bool has_clip;
    rect_t clip_rect;
    bool has_clip_path;
    void* clip_path;
} paint_layer_t;

// Display list commands
typedef enum {
    DISPLAY_DRAW_RECT,
    DISPLAY_DRAW_ROUNDED_RECT,
    DISPLAY_DRAW_TEXT,
    DISPLAY_DRAW_IMAGE,
    DISPLAY_DRAW_LINE,
    DISPLAY_DRAW_PATH,
    DISPLAY_FILL_RECT,
    DISPLAY_STROKE_RECT,
    DISPLAY_CLIP_RECT,
    DISPLAY_SAVE,
    DISPLAY_RESTORE,
    DISPLAY_TRANSLATE,
    DISPLAY_ROTATE,
    DISPLAY_SCALE,
    DISPLAY_SET_TRANSFORM,
    DISPLAY_SET_OPACITY,
    DISPLAY_SET_BLEND_MODE
} display_command_type_t;

// Display list item
typedef struct {
    display_command_type_t type;
    union {
        struct {
            rect_t rect;
            uint32_t color;
            float border_radius[4];
        } rect;
        
        struct {
            char* text;
            float x, y;
            char* font_family;
            float font_size;
            uint32_t color;
        } text;
        
        struct {
            void* image_data;
            rect_t src_rect;
            rect_t dst_rect;
        } image;
        
        struct {
            float x1, y1, x2, y2;
            uint32_t color;
            float width;
        } line;
        
        struct {
            void* path_data;
            uint32_t fill_color;
            uint32_t stroke_color;
            float stroke_width;
        } path;
        
        struct {
            float matrix[16];
        } transform;
        
        struct {
            float opacity;
        } opacity;
    } data;
} display_item_t;

// Display list
typedef struct {
    display_item_t** items;
    uint32_t item_count;
    uint32_t capacity;
    rect_t bounds;
} display_list_t;

// Render pipeline
typedef struct {
    // Layout engine
    struct {
        render_tree_t* (*build_render_tree)(struct dom_element* root);
        void (*compute_layout)(render_tree_t* tree, float viewport_width, float viewport_height);
        void (*reflow)(render_tree_t* tree, layout_box_t* dirty_box);
        bool (*hit_test)(render_tree_t* tree, float x, float y, layout_box_t** result);
    } layout;
    
    // Paint system
    struct {
        paint_layer_t* (*build_layer_tree)(render_tree_t* tree);
        display_list_t* (*paint)(paint_layer_t* layer);
        void (*repaint)(paint_layer_t* layer, rect_t* dirty_rect);
    } paint;
    
    // Compositing
    struct {
        void (*composite)(paint_layer_t** layers, uint32_t layer_count);
        void (*update_layer)(paint_layer_t* layer);
        uint32_t (*create_backing_store)(uint32_t width, uint32_t height);
        void (*destroy_backing_store)(uint32_t texture_id);
    } compositor;
    
    // Rasterization
    struct {
        void* (*create_context)(uint32_t width, uint32_t height);
        void (*execute_display_list)(void* context, display_list_t* list);
        void (*flush)(void* context);
        void (*present)(void* context);
    } raster;
    
    // GPU acceleration
    struct {
        bool enabled;
        void* gl_context;
        struct {
            uint32_t (*compile_shader)(const char* vertex, const char* fragment);
            uint32_t (*create_texture)(uint32_t width, uint32_t height);
            void (*bind_texture)(uint32_t texture_id);
            void (*draw_quad)(rect_t* rect, uint32_t texture_id);
        } gpu;
    } acceleration;
} render_pipeline_t;

// Layout algorithms
void layout_block(layout_box_t* box, float container_width);
void layout_inline(layout_box_t* box, float container_width);
void layout_flex(layout_box_t* box, float container_width);
void layout_grid(layout_box_t* box, float container_width);
void layout_table(layout_box_t* box, float container_width);
void layout_text(layout_box_t* box, float container_width);

// Box tree operations
layout_box_t* create_layout_box(layout_box_type_t type);
void destroy_layout_box(layout_box_t* box);
void append_child_box(layout_box_t* parent, layout_box_t* child);
void remove_child_box(layout_box_t* parent, layout_box_t* child);
layout_box_t* build_layout_tree(struct dom_element* element, struct css_computed_style* style);

// Painting operations
display_list_t* create_display_list(void);
void destroy_display_list(display_list_t* list);
void display_list_draw_rect(display_list_t* list, rect_t* rect, uint32_t color);
void display_list_draw_text(display_list_t* list, const char* text, float x, float y, const char* font, float size, uint32_t color);
void display_list_draw_image(display_list_t* list, void* image, rect_t* src, rect_t* dst);
void display_list_save(display_list_t* list);
void display_list_restore(display_list_t* list);
void display_list_transform(display_list_t* list, float matrix[16]);

// Layer operations
paint_layer_t* create_paint_layer(layout_box_t* box);
void destroy_paint_layer(paint_layer_t* layer);
void add_child_layer(paint_layer_t* parent, paint_layer_t* child);
void remove_child_layer(paint_layer_t* parent, paint_layer_t* child);
paint_layer_t** collect_layers_in_paint_order(paint_layer_t* root, uint32_t* count);

// Hit testing
layout_box_t* hit_test_box(layout_box_t* box, float x, float y);
layout_box_t* hit_test_layer(paint_layer_t* layer, float x, float y);

// Invalidation
void invalidate_layout(render_tree_t* tree, layout_box_t* box);
void invalidate_paint(render_tree_t* tree, layout_box_t* box, rect_t* dirty_rect);
void invalidate_layer(paint_layer_t* layer, rect_t* dirty_rect);

// Scrolling
typedef struct {
    layout_box_t* scrollable_box;
    float scroll_x;
    float scroll_y;
    float scroll_width;
    float scroll_height;
    float viewport_width;
    float viewport_height;
} scroll_state_t;

void scroll_to(scroll_state_t* state, float x, float y);
void scroll_by(scroll_state_t* state, float dx, float dy);
void smooth_scroll_to(scroll_state_t* state, float x, float y, uint32_t duration);

// Animation
typedef struct {
    layout_box_t* target;
    char* property;
    float from;
    float to;
    uint32_t duration;
    uint32_t elapsed;
    enum {
        EASING_LINEAR,
        EASING_EASE_IN,
        EASING_EASE_OUT,
        EASING_EASE_IN_OUT,
        EASING_CUBIC_BEZIER
    } easing;
    float bezier[4];
    bool is_running;
} animation_t;

animation_t* create_animation(layout_box_t* target, const char* property, float from, float to, uint32_t duration);
void start_animation(animation_t* animation);
void stop_animation(animation_t* animation);
void update_animation(animation_t* animation, uint32_t delta_time);
float evaluate_easing(animation_t* animation, float progress);

#endif