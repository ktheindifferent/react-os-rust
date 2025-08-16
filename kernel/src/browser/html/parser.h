#ifndef HTML_PARSER_H
#define HTML_PARSER_H

#include <stdint.h>
#include <stdbool.h>

// HTML5 tokenizer states
typedef enum {
    STATE_DATA,
    STATE_TAG_OPEN,
    STATE_END_TAG_OPEN,
    STATE_TAG_NAME,
    STATE_BEFORE_ATTRIBUTE_NAME,
    STATE_ATTRIBUTE_NAME,
    STATE_AFTER_ATTRIBUTE_NAME,
    STATE_BEFORE_ATTRIBUTE_VALUE,
    STATE_ATTRIBUTE_VALUE_DOUBLE_QUOTED,
    STATE_ATTRIBUTE_VALUE_SINGLE_QUOTED,
    STATE_ATTRIBUTE_VALUE_UNQUOTED,
    STATE_AFTER_ATTRIBUTE_VALUE_QUOTED,
    STATE_SELF_CLOSING_START_TAG,
    STATE_COMMENT_START,
    STATE_COMMENT,
    STATE_COMMENT_END,
    STATE_DOCTYPE,
    STATE_SCRIPT_DATA,
    STATE_STYLE_DATA,
    STATE_CDATA_SECTION
} html_tokenizer_state_t;

// Token types
typedef enum {
    TOKEN_DOCTYPE,
    TOKEN_START_TAG,
    TOKEN_END_TAG,
    TOKEN_SELF_CLOSING_TAG,
    TOKEN_COMMENT,
    TOKEN_CHARACTER,
    TOKEN_EOF
} html_token_type_t;

// HTML token
typedef struct {
    html_token_type_t type;
    char* tag_name;
    struct {
        char* name;
        char* value;
    }* attributes;
    uint32_t attribute_count;
    char* data;
    bool self_closing;
} html_token_t;

// HTML tokenizer
typedef struct {
    const char* input;
    uint32_t position;
    uint32_t length;
    html_tokenizer_state_t state;
    html_token_t* current_token;
    char* buffer;
    uint32_t buffer_size;
} html_tokenizer_t;

// Tree construction modes
typedef enum {
    MODE_INITIAL,
    MODE_BEFORE_HTML,
    MODE_BEFORE_HEAD,
    MODE_IN_HEAD,
    MODE_AFTER_HEAD,
    MODE_IN_BODY,
    MODE_AFTER_BODY,
    MODE_AFTER_AFTER_BODY,
    MODE_IN_TABLE,
    MODE_IN_TABLE_BODY,
    MODE_IN_ROW,
    MODE_IN_CELL,
    MODE_IN_SELECT,
    MODE_IN_TEMPLATE,
    MODE_IN_FRAMESET,
    MODE_AFTER_FRAMESET
} html_insertion_mode_t;

// HTML parser
typedef struct {
    html_tokenizer_t* tokenizer;
    html_insertion_mode_t mode;
    struct dom_element** open_elements;
    uint32_t open_elements_count;
    struct dom_element** active_formatting;
    uint32_t active_formatting_count;
    struct dom_document* document;
    struct dom_element* head_element;
    struct dom_element* form_element;
    bool scripting_enabled;
    bool fragment_parsing;
} html_parser_t;

// Parser API
html_parser_t* html_parser_create(void);
void html_parser_destroy(html_parser_t* parser);
struct dom_document* html_parse(html_parser_t* parser, const char* input, uint32_t length);
struct dom_document* html_parse_fragment(html_parser_t* parser, const char* input, uint32_t length, struct dom_element* context);

// Tokenizer API
html_tokenizer_t* html_tokenizer_create(const char* input, uint32_t length);
void html_tokenizer_destroy(html_tokenizer_t* tokenizer);
html_token_t* html_tokenizer_next_token(html_tokenizer_t* tokenizer);
void html_token_destroy(html_token_t* token);

// Tree construction
void html_process_token(html_parser_t* parser, html_token_t* token);
void html_insert_element(html_parser_t* parser, html_token_t* token);
void html_insert_text(html_parser_t* parser, const char* text);
void html_insert_comment(html_parser_t* parser, const char* comment);
void html_close_element(html_parser_t* parser, const char* tag_name);

// Parsing algorithms
void html_adoption_agency_algorithm(html_parser_t* parser, html_token_t* token);
void html_reconstruct_formatting(html_parser_t* parser);
void html_clear_stack_to_table_context(html_parser_t* parser);
bool html_is_special_element(const char* tag_name);
bool html_is_formatting_element(const char* tag_name);
bool html_is_void_element(const char* tag_name);

// Error recovery
typedef enum {
    HTML_ERROR_UNEXPECTED_TOKEN,
    HTML_ERROR_UNEXPECTED_EOF,
    HTML_ERROR_MISSING_END_TAG,
    HTML_ERROR_NESTED_FORM,
    HTML_ERROR_INVALID_NESTING,
    HTML_ERROR_DUPLICATE_ATTRIBUTE,
    HTML_ERROR_INVALID_CHARACTER
} html_parse_error_t;

typedef void (*html_error_handler_t)(html_parse_error_t error, uint32_t line, uint32_t column, const char* message);
void html_parser_set_error_handler(html_parser_t* parser, html_error_handler_t handler);

#endif