#ifndef CSS_STYLE_H
#define CSS_STYLE_H

#include <stdint.h>
#include <stdbool.h>
#include "parser.h"

// Forward declarations
struct dom_element;

// CSS computed style
typedef struct {
    // Display and positioning
    enum {
        DISPLAY_NONE, DISPLAY_BLOCK, DISPLAY_INLINE, DISPLAY_INLINE_BLOCK,
        DISPLAY_FLEX, DISPLAY_INLINE_FLEX, DISPLAY_GRID, DISPLAY_INLINE_GRID,
        DISPLAY_TABLE, DISPLAY_TABLE_ROW, DISPLAY_TABLE_CELL, DISPLAY_LIST_ITEM
    } display;
    
    enum {
        POSITION_STATIC, POSITION_RELATIVE, POSITION_ABSOLUTE,
        POSITION_FIXED, POSITION_STICKY
    } position;
    
    enum {
        FLOAT_NONE, FLOAT_LEFT, FLOAT_RIGHT
    } float_type;
    
    enum {
        CLEAR_NONE, CLEAR_LEFT, CLEAR_RIGHT, CLEAR_BOTH
    } clear;
    
    // Box model
    struct {
        css_value_t* top;
        css_value_t* right;
        css_value_t* bottom;
        css_value_t* left;
    } margin, padding, border_width;
    
    css_value_t* width;
    css_value_t* height;
    css_value_t* min_width;
    css_value_t* min_height;
    css_value_t* max_width;
    css_value_t* max_height;
    
    enum {
        BOX_SIZING_CONTENT_BOX, BOX_SIZING_BORDER_BOX
    } box_sizing;
    
    // Position offsets
    css_value_t* top;
    css_value_t* right;
    css_value_t* bottom;
    css_value_t* left;
    
    // Typography
    char** font_family;
    uint32_t font_family_count;
    css_value_t* font_size;
    enum {
        FONT_WEIGHT_NORMAL = 400, FONT_WEIGHT_BOLD = 700
    } font_weight;
    enum {
        FONT_STYLE_NORMAL, FONT_STYLE_ITALIC, FONT_STYLE_OBLIQUE
    } font_style;
    css_value_t* line_height;
    enum {
        TEXT_ALIGN_LEFT, TEXT_ALIGN_RIGHT, TEXT_ALIGN_CENTER,
        TEXT_ALIGN_JUSTIFY, TEXT_ALIGN_START, TEXT_ALIGN_END
    } text_align;
    enum {
        TEXT_DECORATION_NONE, TEXT_DECORATION_UNDERLINE,
        TEXT_DECORATION_OVERLINE, TEXT_DECORATION_LINE_THROUGH
    } text_decoration;
    enum {
        TEXT_TRANSFORM_NONE, TEXT_TRANSFORM_CAPITALIZE,
        TEXT_TRANSFORM_UPPERCASE, TEXT_TRANSFORM_LOWERCASE
    } text_transform;
    css_value_t* letter_spacing;
    css_value_t* word_spacing;
    css_value_t* text_indent;
    
    // Colors and backgrounds
    css_value_t* color;
    css_value_t* background_color;
    char** background_image;
    uint32_t background_image_count;
    enum {
        BG_REPEAT_REPEAT, BG_REPEAT_NO_REPEAT,
        BG_REPEAT_REPEAT_X, BG_REPEAT_REPEAT_Y
    } background_repeat;
    enum {
        BG_ATTACHMENT_SCROLL, BG_ATTACHMENT_FIXED, BG_ATTACHMENT_LOCAL
    } background_attachment;
    struct {
        css_value_t* x;
        css_value_t* y;
    } background_position;
    enum {
        BG_SIZE_AUTO, BG_SIZE_COVER, BG_SIZE_CONTAIN
    } background_size;
    
    // Borders
    enum {
        BORDER_STYLE_NONE, BORDER_STYLE_SOLID, BORDER_STYLE_DASHED,
        BORDER_STYLE_DOTTED, BORDER_STYLE_DOUBLE, BORDER_STYLE_GROOVE,
        BORDER_STYLE_RIDGE, BORDER_STYLE_INSET, BORDER_STYLE_OUTSET
    } border_style[4];
    css_value_t* border_color[4];
    css_value_t* border_radius[4];
    
    // Flexbox
    enum {
        FLEX_DIRECTION_ROW, FLEX_DIRECTION_ROW_REVERSE,
        FLEX_DIRECTION_COLUMN, FLEX_DIRECTION_COLUMN_REVERSE
    } flex_direction;
    enum {
        FLEX_WRAP_NOWRAP, FLEX_WRAP_WRAP, FLEX_WRAP_WRAP_REVERSE
    } flex_wrap;
    enum {
        JUSTIFY_FLEX_START, JUSTIFY_FLEX_END, JUSTIFY_CENTER,
        JUSTIFY_SPACE_BETWEEN, JUSTIFY_SPACE_AROUND, JUSTIFY_SPACE_EVENLY
    } justify_content;
    enum {
        ALIGN_FLEX_START, ALIGN_FLEX_END, ALIGN_CENTER,
        ALIGN_BASELINE, ALIGN_STRETCH
    } align_items, align_self;
    css_value_t* flex_grow;
    css_value_t* flex_shrink;
    css_value_t* flex_basis;
    css_value_t* order;
    css_value_t* gap;
    
    // Grid
    char** grid_template_columns;
    uint32_t grid_column_count;
    char** grid_template_rows;
    uint32_t grid_row_count;
    char** grid_template_areas;
    uint32_t grid_area_count;
    css_value_t* grid_gap;
    struct {
        uint32_t start;
        uint32_t end;
    } grid_column, grid_row;
    
    // Visibility and overflow
    enum {
        VISIBILITY_VISIBLE, VISIBILITY_HIDDEN, VISIBILITY_COLLAPSE
    } visibility;
    enum {
        OVERFLOW_VISIBLE, OVERFLOW_HIDDEN, OVERFLOW_SCROLL,
        OVERFLOW_AUTO, OVERFLOW_CLIP
    } overflow_x, overflow_y;
    css_value_t* opacity;
    
    // Transforms
    char** transform;
    uint32_t transform_count;
    struct {
        css_value_t* x;
        css_value_t* y;
        css_value_t* z;
    } transform_origin;
    enum {
        TRANSFORM_STYLE_FLAT, TRANSFORM_STYLE_PRESERVE_3D
    } transform_style;
    css_value_t* perspective;
    
    // Transitions and animations
    struct {
        char* property;
        css_value_t* duration;
        char* timing_function;
        css_value_t* delay;
    }* transitions;
    uint32_t transition_count;
    
    struct {
        char* name;
        css_value_t* duration;
        char* timing_function;
        css_value_t* delay;
        uint32_t iteration_count;
        enum {
            ANIM_DIRECTION_NORMAL, ANIM_DIRECTION_REVERSE,
            ANIM_DIRECTION_ALTERNATE, ANIM_DIRECTION_ALTERNATE_REVERSE
        } direction;
        enum {
            ANIM_FILL_NONE, ANIM_FILL_FORWARDS,
            ANIM_FILL_BACKWARDS, ANIM_FILL_BOTH
        } fill_mode;
        enum {
            ANIM_STATE_RUNNING, ANIM_STATE_PAUSED
        } play_state;
    }* animations;
    uint32_t animation_count;
    
    // Miscellaneous
    css_value_t* z_index;
    enum {
        CURSOR_AUTO, CURSOR_DEFAULT, CURSOR_POINTER, CURSOR_MOVE,
        CURSOR_TEXT, CURSOR_WAIT, CURSOR_HELP, CURSOR_CROSSHAIR,
        CURSOR_NOT_ALLOWED, CURSOR_PROGRESS
    } cursor;
    enum {
        POINTER_EVENTS_AUTO, POINTER_EVENTS_NONE
    } pointer_events;
    enum {
        USER_SELECT_AUTO, USER_SELECT_NONE, USER_SELECT_TEXT, USER_SELECT_ALL
    } user_select;
    
    // CSS custom properties
    struct {
        char* name;
        css_value_t* value;
    }* custom_properties;
    uint32_t custom_property_count;
} css_computed_style_t;

// Style computation
css_computed_style_t* css_compute_style(struct dom_element* element, css_stylesheet_t** stylesheets, uint32_t stylesheet_count);
void css_computed_style_destroy(css_computed_style_t* style);
css_value_t* css_get_computed_value(css_computed_style_t* style, const char* property);
void css_set_inline_style(struct dom_element* element, const char* property, const char* value);

// Cascade and inheritance
typedef struct {
    css_rule_t* rule;
    css_selector_t* selector;
    css_property_t* property;
    uint32_t specificity;
    uint32_t order;
    enum {
        ORIGIN_USER_AGENT,
        ORIGIN_USER,
        ORIGIN_AUTHOR,
        ORIGIN_ANIMATION,
        ORIGIN_TRANSITION
    } origin;
} css_cascade_entry_t;

css_cascade_entry_t** css_collect_declarations(struct dom_element* element, css_stylesheet_t** stylesheets, uint32_t stylesheet_count, uint32_t* count);
void css_sort_declarations(css_cascade_entry_t** entries, uint32_t count);
css_value_t* css_cascade_property(const char* property, css_cascade_entry_t** entries, uint32_t count);
css_value_t* css_inherit_property(const char* property, css_computed_style_t* parent_style);
bool css_is_inherited_property(const char* property);

// CSS animations
typedef struct {
    char* name;
    struct {
        double offset;
        css_declaration_t* declarations;
    }* keyframes;
    uint32_t keyframe_count;
} css_animation_t;

css_animation_t* css_find_animation(const char* name, css_stylesheet_t** stylesheets, uint32_t stylesheet_count);
css_computed_style_t* css_interpolate_animation(css_animation_t* animation, double progress, css_computed_style_t* base_style);
void css_animation_destroy(css_animation_t* animation);

// Style invalidation
typedef struct {
    struct dom_element** elements;
    uint32_t element_count;
    bool needs_layout;
    bool needs_paint;
} css_invalidation_t;

css_invalidation_t* css_invalidate_style(struct dom_element* element, const char* property);
void css_invalidation_destroy(css_invalidation_t* invalidation);

// Style matching cache
typedef struct {
    void* selector_cache;
    void* computed_style_cache;
    uint64_t hit_count;
    uint64_t miss_count;
} css_style_cache_t;

css_style_cache_t* css_style_cache_create(void);
void css_style_cache_destroy(css_style_cache_t* cache);
void css_style_cache_clear(css_style_cache_t* cache);
css_computed_style_t* css_style_cache_get(css_style_cache_t* cache, struct dom_element* element);
void css_style_cache_put(css_style_cache_t* cache, struct dom_element* element, css_computed_style_t* style);

#endif