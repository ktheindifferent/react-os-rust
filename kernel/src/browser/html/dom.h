#ifndef DOM_H
#define DOM_H

#include <stdint.h>
#include <stdbool.h>

// DOM node types
typedef enum {
    NODE_ELEMENT = 1,
    NODE_ATTRIBUTE = 2,
    NODE_TEXT = 3,
    NODE_CDATA_SECTION = 4,
    NODE_PROCESSING_INSTRUCTION = 7,
    NODE_COMMENT = 8,
    NODE_DOCUMENT = 9,
    NODE_DOCUMENT_TYPE = 10,
    NODE_DOCUMENT_FRAGMENT = 11
} dom_node_type_t;

// Forward declarations
typedef struct dom_node dom_node_t;
typedef struct dom_element dom_element_t;
typedef struct dom_document dom_document_t;
typedef struct dom_attribute dom_attribute_t;
typedef struct dom_text dom_text_t;
typedef struct dom_comment dom_comment_t;
typedef struct dom_event dom_event_t;

// Base DOM node
struct dom_node {
    dom_node_type_t type;
    char* node_name;
    char* node_value;
    dom_document_t* owner_document;
    dom_node_t* parent_node;
    dom_node_t* first_child;
    dom_node_t* last_child;
    dom_node_t* previous_sibling;
    dom_node_t* next_sibling;
    uint32_t child_count;
    void* user_data;
};

// DOM element
struct dom_element {
    dom_node_t base;
    char* tag_name;
    char* id;
    char** class_list;
    uint32_t class_count;
    dom_attribute_t** attributes;
    uint32_t attribute_count;
    char* namespace_uri;
    char* prefix;
    
    // Style and layout
    void* computed_style;
    void* layout_box;
    
    // Shadow DOM
    dom_node_t* shadow_root;
    bool is_custom_element;
    
    // Event listeners
    struct {
        char* event_type;
        void (*handler)(dom_event_t*);
    }* event_listeners;
    uint32_t listener_count;
};

// DOM document
struct dom_document {
    dom_node_t base;
    char* document_uri;
    char* charset;
    char* content_type;
    dom_element_t* document_element;
    dom_element_t* head;
    dom_element_t* body;
    
    // Document state
    enum {
        READY_STATE_LOADING,
        READY_STATE_INTERACTIVE,
        READY_STATE_COMPLETE
    } ready_state;
    
    // Special collections
    struct {
        dom_element_t** forms;
        uint32_t form_count;
        dom_element_t** images;
        uint32_t image_count;
        dom_element_t** links;
        uint32_t link_count;
        dom_element_t** scripts;
        uint32_t script_count;
    } collections;
    
    // ID and name maps
    void* id_map;
    void* name_map;
    
    // Custom elements registry
    void* custom_elements;
    
    // Mutation observers
    void* mutation_observers;
};

// DOM attribute
struct dom_attribute {
    char* name;
    char* value;
    char* namespace_uri;
    char* prefix;
    dom_element_t* owner_element;
    bool specified;
};

// DOM text node
struct dom_text {
    dom_node_t base;
    char* data;
    uint32_t length;
    bool is_element_content_whitespace;
};

// DOM comment
struct dom_comment {
    dom_node_t base;
    char* data;
    uint32_t length;
};

// Node operations
dom_node_t* dom_node_clone(dom_node_t* node, bool deep);
dom_node_t* dom_node_append_child(dom_node_t* parent, dom_node_t* child);
dom_node_t* dom_node_insert_before(dom_node_t* parent, dom_node_t* child, dom_node_t* before);
dom_node_t* dom_node_remove_child(dom_node_t* parent, dom_node_t* child);
dom_node_t* dom_node_replace_child(dom_node_t* parent, dom_node_t* new_child, dom_node_t* old_child);
bool dom_node_contains(dom_node_t* node, dom_node_t* other);
char* dom_node_get_text_content(dom_node_t* node);
void dom_node_set_text_content(dom_node_t* node, const char* content);

// Document operations
dom_document_t* dom_document_create(void);
void dom_document_destroy(dom_document_t* document);
dom_element_t* dom_document_create_element(dom_document_t* document, const char* tag_name);
dom_element_t* dom_document_create_element_ns(dom_document_t* document, const char* namespace_uri, const char* qualified_name);
dom_text_t* dom_document_create_text_node(dom_document_t* document, const char* data);
dom_comment_t* dom_document_create_comment(dom_document_t* document, const char* data);
dom_attribute_t* dom_document_create_attribute(dom_document_t* document, const char* name);
dom_node_t* dom_document_import_node(dom_document_t* document, dom_node_t* node, bool deep);

// Element operations
dom_attribute_t* dom_element_get_attribute_node(dom_element_t* element, const char* name);
char* dom_element_get_attribute(dom_element_t* element, const char* name);
void dom_element_set_attribute(dom_element_t* element, const char* name, const char* value);
void dom_element_remove_attribute(dom_element_t* element, const char* name);
bool dom_element_has_attribute(dom_element_t* element, const char* name);
dom_element_t* dom_element_get_by_id(dom_document_t* document, const char* id);
dom_element_t** dom_element_get_by_tag_name(dom_element_t* element, const char* tag_name, uint32_t* count);
dom_element_t** dom_element_get_by_class_name(dom_element_t* element, const char* class_name, uint32_t* count);
bool dom_element_matches(dom_element_t* element, const char* selector);
dom_element_t* dom_element_query_selector(dom_element_t* element, const char* selector);
dom_element_t** dom_element_query_selector_all(dom_element_t* element, const char* selector, uint32_t* count);

// Shadow DOM
dom_node_t* dom_element_attach_shadow(dom_element_t* element, bool open);
dom_node_t* dom_element_get_shadow_root(dom_element_t* element);

// Custom elements
void dom_define_custom_element(dom_document_t* document, const char* name, void* constructor);
void dom_upgrade_element(dom_element_t* element, const char* name);

// Mutation observers
typedef struct {
    dom_node_t* target;
    enum {
        MUTATION_ATTRIBUTES = 1,
        MUTATION_CHARACTER_DATA = 2,
        MUTATION_CHILD_LIST = 4,
        MUTATION_SUBTREE = 8
    } type;
    char* attribute_name;
    char* attribute_namespace;
    char* old_value;
    dom_node_t** added_nodes;
    uint32_t added_count;
    dom_node_t** removed_nodes;
    uint32_t removed_count;
    dom_node_t* previous_sibling;
    dom_node_t* next_sibling;
} dom_mutation_record_t;

typedef void (*dom_mutation_callback_t)(dom_mutation_record_t** records, uint32_t count);

void* dom_create_mutation_observer(dom_mutation_callback_t callback);
void dom_observe_mutations(void* observer, dom_node_t* target, uint32_t options);
void dom_disconnect_observer(void* observer);
dom_mutation_record_t** dom_take_records(void* observer, uint32_t* count);

// Events
typedef enum {
    EVENT_PHASE_NONE = 0,
    EVENT_PHASE_CAPTURING = 1,
    EVENT_PHASE_AT_TARGET = 2,
    EVENT_PHASE_BUBBLING = 3
} dom_event_phase_t;

struct dom_event {
    char* type;
    dom_node_t* target;
    dom_node_t* current_target;
    dom_event_phase_t event_phase;
    bool bubbles;
    bool cancelable;
    bool default_prevented;
    bool composed;
    bool is_trusted;
    uint64_t timestamp;
    void* detail;
};

void dom_element_add_event_listener(dom_element_t* element, const char* type, void (*handler)(dom_event_t*), bool capture);
void dom_element_remove_event_listener(dom_element_t* element, const char* type, void (*handler)(dom_event_t*), bool capture);
void dom_element_dispatch_event(dom_element_t* element, dom_event_t* event);

// Tree walking
typedef struct {
    dom_node_t* root;
    uint32_t what_to_show;
    bool (*filter)(dom_node_t*);
    dom_node_t* current_node;
} dom_tree_walker_t;

dom_tree_walker_t* dom_create_tree_walker(dom_node_t* root, uint32_t what_to_show, bool (*filter)(dom_node_t*));
dom_node_t* dom_tree_walker_next_node(dom_tree_walker_t* walker);
dom_node_t* dom_tree_walker_previous_node(dom_tree_walker_t* walker);
dom_node_t* dom_tree_walker_parent_node(dom_tree_walker_t* walker);
dom_node_t* dom_tree_walker_first_child(dom_tree_walker_t* walker);
dom_node_t* dom_tree_walker_last_child(dom_tree_walker_t* walker);
void dom_tree_walker_destroy(dom_tree_walker_t* walker);

#endif