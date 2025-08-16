#ifndef CSS_PARSER_H
#define CSS_PARSER_H

#include <stdint.h>
#include <stdbool.h>

// CSS token types
typedef enum {
    CSS_TOKEN_IDENT,
    CSS_TOKEN_FUNCTION,
    CSS_TOKEN_AT_KEYWORD,
    CSS_TOKEN_HASH,
    CSS_TOKEN_STRING,
    CSS_TOKEN_URL,
    CSS_TOKEN_NUMBER,
    CSS_TOKEN_PERCENTAGE,
    CSS_TOKEN_DIMENSION,
    CSS_TOKEN_WHITESPACE,
    CSS_TOKEN_CDO,
    CSS_TOKEN_CDC,
    CSS_TOKEN_COLON,
    CSS_TOKEN_SEMICOLON,
    CSS_TOKEN_COMMA,
    CSS_TOKEN_LEFT_BRACKET,
    CSS_TOKEN_RIGHT_BRACKET,
    CSS_TOKEN_LEFT_PAREN,
    CSS_TOKEN_RIGHT_PAREN,
    CSS_TOKEN_LEFT_BRACE,
    CSS_TOKEN_RIGHT_BRACE,
    CSS_TOKEN_DELIM,
    CSS_TOKEN_EOF
} css_token_type_t;

// CSS token
typedef struct {
    css_token_type_t type;
    union {
        char* string_value;
        double numeric_value;
        struct {
            double value;
            char* unit;
        } dimension;
        uint32_t hash;
    } value;
    char* raw;
} css_token_t;

// CSS tokenizer
typedef struct {
    const char* input;
    uint32_t position;
    uint32_t length;
    css_token_t* current_token;
    css_token_t* next_token;
} css_tokenizer_t;

// CSS selector types
typedef enum {
    SELECTOR_TYPE,
    SELECTOR_CLASS,
    SELECTOR_ID,
    SELECTOR_ATTRIBUTE,
    SELECTOR_PSEUDO_CLASS,
    SELECTOR_PSEUDO_ELEMENT,
    SELECTOR_UNIVERSAL,
    SELECTOR_DESCENDANT,
    SELECTOR_CHILD,
    SELECTOR_ADJACENT_SIBLING,
    SELECTOR_GENERAL_SIBLING
} css_selector_type_t;

// CSS selector
typedef struct css_selector {
    css_selector_type_t type;
    char* value;
    struct css_selector* next;
    struct css_selector* child;
    
    // Attribute selector
    struct {
        char* name;
        char* value;
        enum {
            ATTR_EQUALS,
            ATTR_INCLUDES,
            ATTR_DASH_MATCH,
            ATTR_PREFIX_MATCH,
            ATTR_SUFFIX_MATCH,
            ATTR_SUBSTRING_MATCH
        } match_type;
    } attribute;
    
    // Pseudo-class/element
    struct {
        char* name;
        char* argument;
    } pseudo;
    
    // Specificity
    uint32_t specificity;
} css_selector_t;

// CSS property
typedef struct {
    char* name;
    char* value;
    bool important;
    uint32_t source_line;
} css_property_t;

// CSS declaration
typedef struct {
    css_property_t** properties;
    uint32_t property_count;
} css_declaration_t;

// CSS rule types
typedef enum {
    RULE_STYLE,
    RULE_IMPORT,
    RULE_MEDIA,
    RULE_FONT_FACE,
    RULE_KEYFRAMES,
    RULE_KEYFRAME,
    RULE_NAMESPACE,
    RULE_SUPPORTS,
    RULE_DOCUMENT,
    RULE_PAGE,
    RULE_VIEWPORT
} css_rule_type_t;

// CSS rule
typedef struct css_rule {
    css_rule_type_t type;
    css_selector_t** selectors;
    uint32_t selector_count;
    css_declaration_t* declarations;
    
    // Media rule
    struct {
        char* media_query;
        struct css_rule** rules;
        uint32_t rule_count;
    } media;
    
    // Keyframes
    struct {
        char* name;
        struct {
            char* selector;
            css_declaration_t* declarations;
        }* keyframes;
        uint32_t keyframe_count;
    } animation;
    
    struct css_rule* next;
} css_rule_t;

// CSS stylesheet
typedef struct {
    css_rule_t** rules;
    uint32_t rule_count;
    char* href;
    char* type;
    char* media;
    bool disabled;
    struct dom_node* owner_node;
    struct css_stylesheet* parent;
} css_stylesheet_t;

// Parser API
css_stylesheet_t* css_parse_stylesheet(const char* input, uint32_t length);
css_rule_t* css_parse_rule(const char* input, uint32_t length);
css_selector_t* css_parse_selector(const char* input, uint32_t length);
css_declaration_t* css_parse_declaration(const char* input, uint32_t length);
void css_stylesheet_destroy(css_stylesheet_t* stylesheet);

// Tokenizer API
css_tokenizer_t* css_tokenizer_create(const char* input, uint32_t length);
void css_tokenizer_destroy(css_tokenizer_t* tokenizer);
css_token_t* css_tokenizer_next_token(css_tokenizer_t* tokenizer);
css_token_t* css_tokenizer_peek_token(css_tokenizer_t* tokenizer);
void css_token_destroy(css_token_t* token);

// Selector matching
bool css_selector_matches(css_selector_t* selector, struct dom_element* element);
uint32_t css_calculate_specificity(css_selector_t* selector);
int css_compare_specificity(uint32_t a, uint32_t b);

// Media queries
typedef struct {
    enum {
        MEDIA_ALL,
        MEDIA_SCREEN,
        MEDIA_PRINT,
        MEDIA_SPEECH
    } type;
    
    struct {
        char* feature;
        char* value;
        enum {
            MQ_MIN,
            MQ_MAX,
            MQ_EXACT
        } prefix;
    }* features;
    uint32_t feature_count;
    
    bool negated;
    bool only;
} css_media_query_t;

css_media_query_t* css_parse_media_query(const char* query);
bool css_media_query_matches(css_media_query_t* query, void* viewport);
void css_media_query_destroy(css_media_query_t* query);

// CSS values
typedef enum {
    VALUE_LENGTH,
    VALUE_PERCENTAGE,
    VALUE_COLOR,
    VALUE_STRING,
    VALUE_URL,
    VALUE_NUMBER,
    VALUE_KEYWORD,
    VALUE_FUNCTION,
    VALUE_LIST
} css_value_type_t;

typedef struct css_value {
    css_value_type_t type;
    union {
        struct {
            double value;
            enum {
                UNIT_PX, UNIT_EM, UNIT_REM, UNIT_VW, UNIT_VH,
                UNIT_PT, UNIT_PC, UNIT_IN, UNIT_CM, UNIT_MM,
                UNIT_EX, UNIT_CH, UNIT_VMIN, UNIT_VMAX
            } unit;
        } length;
        
        double percentage;
        
        struct {
            uint8_t r, g, b, a;
        } color;
        
        char* string;
        char* url;
        double number;
        char* keyword;
        
        struct {
            char* name;
            struct css_value** arguments;
            uint32_t argument_count;
        } function;
        
        struct {
            struct css_value** items;
            uint32_t item_count;
        } list;
    } value;
} css_value_t;

css_value_t* css_parse_value(const char* input);
void css_value_destroy(css_value_t* value);

// Error handling
typedef enum {
    CSS_ERROR_UNEXPECTED_TOKEN,
    CSS_ERROR_UNEXPECTED_EOF,
    CSS_ERROR_INVALID_SELECTOR,
    CSS_ERROR_INVALID_PROPERTY,
    CSS_ERROR_INVALID_VALUE,
    CSS_ERROR_INVALID_AT_RULE
} css_parse_error_t;

typedef void (*css_error_handler_t)(css_parse_error_t error, uint32_t line, uint32_t column, const char* message);
void css_parser_set_error_handler(css_error_handler_t handler);

#endif