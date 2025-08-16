#ifndef SECURITY_CSP_H
#define SECURITY_CSP_H

#include <stdint.h>
#include <stdbool.h>

// CSP directive types
typedef enum {
    CSP_DEFAULT_SRC,
    CSP_SCRIPT_SRC,
    CSP_STYLE_SRC,
    CSP_IMG_SRC,
    CSP_FONT_SRC,
    CSP_CONNECT_SRC,
    CSP_MEDIA_SRC,
    CSP_OBJECT_SRC,
    CSP_FRAME_SRC,
    CSP_FRAME_ANCESTORS,
    CSP_WORKER_SRC,
    CSP_MANIFEST_SRC,
    CSP_BASE_URI,
    CSP_FORM_ACTION,
    CSP_PLUGIN_TYPES,
    CSP_SANDBOX,
    CSP_UPGRADE_INSECURE_REQUESTS,
    CSP_BLOCK_ALL_MIXED_CONTENT,
    CSP_REQUIRE_SRI_FOR,
    CSP_REPORT_URI,
    CSP_REPORT_TO
} csp_directive_type_t;

// CSP source types
typedef enum {
    CSP_SOURCE_NONE,
    CSP_SOURCE_SELF,
    CSP_SOURCE_UNSAFE_INLINE,
    CSP_SOURCE_UNSAFE_EVAL,
    CSP_SOURCE_UNSAFE_HASHES,
    CSP_SOURCE_STRICT_DYNAMIC,
    CSP_SOURCE_REPORT_SAMPLE,
    CSP_SOURCE_SCHEME,
    CSP_SOURCE_HOST,
    CSP_SOURCE_NONCE,
    CSP_SOURCE_HASH
} csp_source_type_t;

// CSP hash algorithms
typedef enum {
    CSP_HASH_SHA256,
    CSP_HASH_SHA384,
    CSP_HASH_SHA512
} csp_hash_algorithm_t;

// CSP source
typedef struct {
    csp_source_type_t type;
    union {
        char* scheme;
        struct {
            char* host;
            uint16_t port;
            char* path;
        } host_source;
        char* nonce;
        struct {
            csp_hash_algorithm_t algorithm;
            char* value;
        } hash;
    } value;
} csp_source_t;

// CSP directive
typedef struct {
    csp_directive_type_t type;
    csp_source_t** sources;
    uint32_t source_count;
} csp_directive_t;

// CSP policy
typedef struct {
    csp_directive_t** directives;
    uint32_t directive_count;
    char* report_uri;
    char* report_to;
    bool report_only;
} csp_policy_t;

// CSP violation
typedef struct {
    char* document_uri;
    char* referrer;
    char* violated_directive;
    char* effective_directive;
    char* original_policy;
    char* blocked_uri;
    char* source_file;
    uint32_t line_number;
    uint32_t column_number;
    char* sample;
    char* disposition;
    uint16_t status_code;
} csp_violation_t;

// Policy parsing
csp_policy_t* csp_parse_policy(const char* policy_string);
void csp_policy_destroy(csp_policy_t* policy);
csp_directive_t* csp_find_directive(csp_policy_t* policy, csp_directive_type_t type);

// Policy enforcement
bool csp_allows_source(csp_policy_t* policy, csp_directive_type_t directive, const char* source_url);
bool csp_allows_inline_script(csp_policy_t* policy, const char* script_content, const char* nonce);
bool csp_allows_inline_style(csp_policy_t* policy, const char* style_content, const char* nonce);
bool csp_allows_eval(csp_policy_t* policy);
bool csp_allows_unsafe_inline(csp_policy_t* policy, csp_directive_type_t directive);

// Nonce and hash validation
char* csp_generate_nonce(void);
bool csp_validate_nonce(csp_policy_t* policy, csp_directive_type_t directive, const char* nonce);
char* csp_compute_hash(const char* content, csp_hash_algorithm_t algorithm);
bool csp_validate_hash(csp_policy_t* policy, csp_directive_type_t directive, const char* content);

// Reporting
void csp_report_violation(csp_violation_t* violation, const char* report_uri);
csp_violation_t* csp_create_violation(csp_policy_t* policy, csp_directive_type_t directive, const char* blocked_uri);
void csp_violation_destroy(csp_violation_t* violation);

// Sandbox directives
typedef enum {
    SANDBOX_ALLOW_FORMS = 1 << 0,
    SANDBOX_ALLOW_MODALS = 1 << 1,
    SANDBOX_ALLOW_ORIENTATION_LOCK = 1 << 2,
    SANDBOX_ALLOW_POINTER_LOCK = 1 << 3,
    SANDBOX_ALLOW_POPUPS = 1 << 4,
    SANDBOX_ALLOW_POPUPS_TO_ESCAPE = 1 << 5,
    SANDBOX_ALLOW_PRESENTATION = 1 << 6,
    SANDBOX_ALLOW_SAME_ORIGIN = 1 << 7,
    SANDBOX_ALLOW_SCRIPTS = 1 << 8,
    SANDBOX_ALLOW_TOP_NAVIGATION = 1 << 9,
    SANDBOX_ALLOW_TOP_NAVIGATION_BY_USER = 1 << 10,
    SANDBOX_ALLOW_DOWNLOADS = 1 << 11
} csp_sandbox_flag_t;

uint32_t csp_parse_sandbox_flags(const char* sandbox_value);
bool csp_sandbox_allows(uint32_t flags, csp_sandbox_flag_t flag);

// Same-origin policy
typedef struct {
    char* scheme;
    char* host;
    uint16_t port;
} origin_t;

origin_t* origin_parse(const char* url);
bool origin_same(origin_t* a, origin_t* b);
void origin_destroy(origin_t* origin);

// CORS (Cross-Origin Resource Sharing)
typedef struct {
    origin_t* origin;
    char* method;
    char** headers;
    uint32_t header_count;
    bool credentials;
} cors_request_t;

typedef struct {
    char** allowed_origins;
    uint32_t allowed_origin_count;
    char** allowed_methods;
    uint32_t allowed_method_count;
    char** allowed_headers;
    uint32_t allowed_header_count;
    char** exposed_headers;
    uint32_t exposed_header_count;
    uint32_t max_age;
    bool allow_credentials;
} cors_policy_t;

bool cors_check_request(cors_request_t* request, cors_policy_t* policy);
void cors_apply_headers(void* response, cors_policy_t* policy, origin_t* origin);

// Mixed content blocking
typedef enum {
    CONTENT_TYPE_BLOCKABLE,
    CONTENT_TYPE_OPTIONALLY_BLOCKABLE
} mixed_content_type_t;

bool mixed_content_should_block(const char* page_url, const char* resource_url, mixed_content_type_t type);
void mixed_content_upgrade_insecure(char** url);

// Subresource Integrity (SRI)
typedef struct {
    csp_hash_algorithm_t algorithm;
    char* hash;
} sri_hash_t;

typedef struct {
    sri_hash_t** hashes;
    uint32_t hash_count;
} sri_metadata_t;

sri_metadata_t* sri_parse_metadata(const char* integrity);
bool sri_verify(const void* data, uint32_t length, sri_metadata_t* metadata);
void sri_metadata_destroy(sri_metadata_t* metadata);

// Feature Policy / Permissions Policy
typedef enum {
    FEATURE_CAMERA,
    FEATURE_MICROPHONE,
    FEATURE_GEOLOCATION,
    FEATURE_NOTIFICATIONS,
    FEATURE_PUSH,
    FEATURE_SYNC_XHR,
    FEATURE_FULLSCREEN,
    FEATURE_PAYMENT,
    FEATURE_USB,
    FEATURE_BLUETOOTH,
    FEATURE_DISPLAY_CAPTURE,
    FEATURE_ACCELEROMETER,
    FEATURE_GYROSCOPE,
    FEATURE_MAGNETOMETER,
    FEATURE_MIDI,
    FEATURE_ENCRYPTED_MEDIA,
    FEATURE_AUTOPLAY,
    FEATURE_PICTURE_IN_PICTURE,
    FEATURE_XR_SPATIAL_TRACKING
} permission_feature_t;

typedef struct {
    permission_feature_t feature;
    char** allowed_origins;
    uint32_t origin_count;
    bool allow_self;
    bool allow_all;
} permission_directive_t;

typedef struct {
    permission_directive_t** directives;
    uint32_t directive_count;
} permissions_policy_t;

permissions_policy_t* permissions_parse_policy(const char* policy_string);
bool permissions_allows_feature(permissions_policy_t* policy, permission_feature_t feature, origin_t* origin);
void permissions_policy_destroy(permissions_policy_t* policy);

// Trusted Types
typedef struct {
    char** policy_names;
    uint32_t policy_count;
    bool allow_duplicates;
    char* default_policy;
    bool require_for_script;
} trusted_types_config_t;

typedef struct {
    char* name;
    char* (*create_html)(const char* input);
    char* (*create_script)(const char* input);
    char* (*create_script_url)(const char* input);
} trusted_types_policy_t;

trusted_types_policy_t* trusted_types_create_policy(const char* name, trusted_types_config_t* config);
bool trusted_types_allows_policy(trusted_types_config_t* config, const char* policy_name);
void trusted_types_policy_destroy(trusted_types_policy_t* policy);

// X-Frame-Options
typedef enum {
    FRAME_OPTIONS_DENY,
    FRAME_OPTIONS_SAMEORIGIN,
    FRAME_OPTIONS_ALLOW_FROM
} frame_options_t;

typedef struct {
    frame_options_t option;
    char* allowed_origin;
} frame_options_policy_t;

frame_options_policy_t* frame_options_parse(const char* header);
bool frame_options_allows_framing(frame_options_policy_t* policy, origin_t* parent_origin, origin_t* frame_origin);
void frame_options_policy_destroy(frame_options_policy_t* policy);

// Certificate validation
typedef struct {
    char* subject;
    char* issuer;
    uint64_t not_before;
    uint64_t not_after;
    char** san_list;
    uint32_t san_count;
    uint8_t* public_key;
    uint32_t public_key_size;
    uint8_t* signature;
    uint32_t signature_size;
} certificate_t;

typedef struct {
    certificate_t** chain;
    uint32_t chain_length;
    bool valid;
    char* error_message;
} certificate_validation_t;

certificate_validation_t* certificate_validate(certificate_t* cert, certificate_t** trusted_roots, uint32_t root_count);
bool certificate_matches_host(certificate_t* cert, const char* hostname);
void certificate_validation_destroy(certificate_validation_t* validation);

// HSTS (HTTP Strict Transport Security)
typedef struct {
    uint32_t max_age;
    bool include_subdomains;
    bool preload;
} hsts_policy_t;

typedef struct {
    char* host;
    hsts_policy_t* policy;
    uint64_t expiry;
} hsts_entry_t;

typedef struct {
    hsts_entry_t** entries;
    uint32_t entry_count;
} hsts_store_t;

hsts_policy_t* hsts_parse_header(const char* header);
void hsts_store_add(hsts_store_t* store, const char* host, hsts_policy_t* policy);
bool hsts_should_upgrade(hsts_store_t* store, const char* host);
void hsts_store_cleanup(hsts_store_t* store);
void hsts_store_destroy(hsts_store_t* store);

#endif