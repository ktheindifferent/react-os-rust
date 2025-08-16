#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <signal.h>
#include <errno.h>
#include <sys/socket.h>
#include <sys/ioctl.h>
#include <net/if.h>
#include <linux/wireless.h>
#include <pthread.h>

#define WPA_VERSION "2.10"
#define CONFIG_FILE "/etc/wpa_supplicant.conf"
#define PID_FILE "/var/run/wpa_supplicant.pid"
#define CTRL_IFACE_DIR "/var/run/wpa_supplicant"

#define MAX_SSID_LEN 32
#define MAX_PASSPHRASE_LEN 63
#define MAX_NETWORKS 32
#define SCAN_INTERVAL 30
#define RECONNECT_INTERVAL 5

typedef enum {
    WPA_DISCONNECTED,
    WPA_SCANNING,
    WPA_AUTHENTICATING,
    WPA_ASSOCIATING,
    WPA_ASSOCIATED,
    WPA_4WAY_HANDSHAKE,
    WPA_GROUP_HANDSHAKE,
    WPA_COMPLETED,
} wpa_state_t;

typedef enum {
    AUTH_OPEN,
    AUTH_WPA_PSK,
    AUTH_WPA2_PSK,
    AUTH_WPA3_SAE,
    AUTH_WPA2_ENTERPRISE,
    AUTH_WPA3_ENTERPRISE,
} auth_type_t;

typedef enum {
    CIPHER_NONE,
    CIPHER_WEP40,
    CIPHER_WEP104,
    CIPHER_TKIP,
    CIPHER_CCMP,
    CIPHER_CCMP256,
    CIPHER_GCMP,
    CIPHER_GCMP256,
} cipher_type_t;

typedef struct network_profile {
    char ssid[MAX_SSID_LEN + 1];
    char passphrase[MAX_PASSPHRASE_LEN + 1];
    uint8_t bssid[6];
    auth_type_t auth_type;
    cipher_type_t pairwise_cipher;
    cipher_type_t group_cipher;
    int priority;
    int disabled;
    int scan_ssid;
    struct network_profile *next;
} network_profile_t;

typedef struct scan_result {
    uint8_t bssid[6];
    char ssid[MAX_SSID_LEN + 1];
    int frequency;
    int signal_level;
    uint16_t capabilities;
    auth_type_t auth_type;
    cipher_type_t pairwise_cipher;
    cipher_type_t group_cipher;
    struct scan_result *next;
} scan_result_t;

typedef struct wpa_supplicant {
    char interface[IFNAMSIZ];
    int sock;
    int ctrl_sock;
    wpa_state_t state;
    network_profile_t *networks;
    scan_result_t *scan_results;
    network_profile_t *current_network;
    uint8_t own_addr[6];
    uint8_t bssid[6];
    pthread_t event_thread;
    pthread_t scan_thread;
    pthread_mutex_t mutex;
    int running;
    int auto_connect;
    int scan_interval;
    int debug_level;
} wpa_supplicant_t;

static wpa_supplicant_t *global_wpa_s = NULL;

static void log_message(int level, const char *fmt, ...) {
    if (global_wpa_s && level <= global_wpa_s->debug_level) {
        va_list args;
        va_start(args, fmt);
        vprintf(fmt, args);
        va_end(args);
        printf("\n");
    }
}

static int parse_config_file(wpa_supplicant_t *wpa_s, const char *config_file) {
    FILE *fp = fopen(config_file, "r");
    if (!fp) {
        log_message(1, "Failed to open config file: %s", config_file);
        return -1;
    }

    char line[256];
    network_profile_t *current_network = NULL;
    int in_network_block = 0;

    while (fgets(line, sizeof(line), fp)) {
        char *p = strchr(line, '\n');
        if (p) *p = '\0';
        
        p = strchr(line, '#');
        if (p) *p = '\0';
        
        p = line;
        while (*p == ' ' || *p == '\t') p++;
        if (*p == '\0') continue;

        if (strncmp(p, "network={", 9) == 0) {
            in_network_block = 1;
            current_network = calloc(1, sizeof(network_profile_t));
            current_network->auth_type = AUTH_WPA2_PSK;
            current_network->pairwise_cipher = CIPHER_CCMP;
            current_network->group_cipher = CIPHER_CCMP;
            current_network->priority = 0;
            continue;
        }

        if (in_network_block && *p == '}') {
            in_network_block = 0;
            if (current_network) {
                current_network->next = wpa_s->networks;
                wpa_s->networks = current_network;
                current_network = NULL;
            }
            continue;
        }

        if (in_network_block && current_network) {
            char key[64], value[128];
            if (sscanf(p, "%63[^=]=%127s", key, value) == 2) {
                while (key[strlen(key)-1] == ' ') key[strlen(key)-1] = '\0';
                
                if (value[0] == '"' && value[strlen(value)-1] == '"') {
                    value[strlen(value)-1] = '\0';
                    memmove(value, value+1, strlen(value));
                }

                if (strcmp(key, "ssid") == 0) {
                    strncpy(current_network->ssid, value, MAX_SSID_LEN);
                } else if (strcmp(key, "psk") == 0) {
                    strncpy(current_network->passphrase, value, MAX_PASSPHRASE_LEN);
                } else if (strcmp(key, "key_mgmt") == 0) {
                    if (strstr(value, "WPA-PSK")) {
                        current_network->auth_type = AUTH_WPA_PSK;
                    } else if (strstr(value, "WPA2-PSK")) {
                        current_network->auth_type = AUTH_WPA2_PSK;
                    } else if (strstr(value, "SAE")) {
                        current_network->auth_type = AUTH_WPA3_SAE;
                    } else if (strstr(value, "NONE")) {
                        current_network->auth_type = AUTH_OPEN;
                    }
                } else if (strcmp(key, "priority") == 0) {
                    current_network->priority = atoi(value);
                } else if (strcmp(key, "disabled") == 0) {
                    current_network->disabled = atoi(value);
                } else if (strcmp(key, "scan_ssid") == 0) {
                    current_network->scan_ssid = atoi(value);
                }
            }
        }

        if (!in_network_block) {
            char key[64], value[128];
            if (sscanf(p, "%63[^=]=%127s", key, value) == 2) {
                if (strcmp(key, "ctrl_interface") == 0) {
                } else if (strcmp(key, "update_config") == 0) {
                } else if (strcmp(key, "ap_scan") == 0) {
                    wpa_s->auto_connect = atoi(value);
                }
            }
        }
    }

    fclose(fp);
    return 0;
}

static int init_wireless_socket(wpa_supplicant_t *wpa_s) {
    wpa_s->sock = socket(AF_INET, SOCK_DGRAM, 0);
    if (wpa_s->sock < 0) {
        log_message(1, "Failed to create socket: %s", strerror(errno));
        return -1;
    }

    struct ifreq ifr;
    memset(&ifr, 0, sizeof(ifr));
    strncpy(ifr.ifr_name, wpa_s->interface, IFNAMSIZ);

    if (ioctl(wpa_s->sock, SIOCGIFHWADDR, &ifr) == 0) {
        memcpy(wpa_s->own_addr, ifr.ifr_hwaddr.sa_data, 6);
        log_message(2, "Interface %s MAC: %02x:%02x:%02x:%02x:%02x:%02x",
            wpa_s->interface,
            wpa_s->own_addr[0], wpa_s->own_addr[1], wpa_s->own_addr[2],
            wpa_s->own_addr[3], wpa_s->own_addr[4], wpa_s->own_addr[5]);
    }

    return 0;
}

static int trigger_scan(wpa_supplicant_t *wpa_s) {
    struct iwreq wrq;
    memset(&wrq, 0, sizeof(wrq));
    strncpy(wrq.ifr_name, wpa_s->interface, IFNAMSIZ);

    wpa_s->state = WPA_SCANNING;
    log_message(2, "Starting scan on %s", wpa_s->interface);

    if (ioctl(wpa_s->sock, SIOCSIWSCAN, &wrq) < 0) {
        if (errno != EBUSY) {
            log_message(1, "Scan trigger failed: %s", strerror(errno));
            return -1;
        }
    }

    return 0;
}

static int get_scan_results(wpa_supplicant_t *wpa_s) {
    struct iwreq wrq;
    unsigned char *buffer;
    int buflen = 8192;

    buffer = malloc(buflen);
    if (!buffer) {
        return -1;
    }

    memset(&wrq, 0, sizeof(wrq));
    strncpy(wrq.ifr_name, wpa_s->interface, IFNAMSIZ);
    wrq.u.data.pointer = buffer;
    wrq.u.data.length = buflen;

    if (ioctl(wpa_s->sock, SIOCGIWSCAN, &wrq) < 0) {
        free(buffer);
        if (errno == EAGAIN) {
            return 0;
        }
        log_message(1, "Failed to get scan results: %s", strerror(errno));
        return -1;
    }

    scan_result_t *prev = NULL;
    while (wpa_s->scan_results) {
        scan_result_t *tmp = wpa_s->scan_results;
        wpa_s->scan_results = wpa_s->scan_results->next;
        free(tmp);
    }

    log_message(2, "Scan completed, processing results");

    free(buffer);
    return 0;
}

static network_profile_t *select_network(wpa_supplicant_t *wpa_s) {
    network_profile_t *best = NULL;
    int best_priority = -1;
    int best_signal = -1000;

    for (network_profile_t *net = wpa_s->networks; net; net = net->next) {
        if (net->disabled) continue;

        for (scan_result_t *bss = wpa_s->scan_results; bss; bss = bss->next) {
            if (strcmp(bss->ssid, net->ssid) != 0) continue;

            if (net->priority > best_priority ||
                (net->priority == best_priority && bss->signal_level > best_signal)) {
                best = net;
                best_priority = net->priority;
                best_signal = bss->signal_level;
            }
        }
    }

    return best;
}

static int connect_to_network(wpa_supplicant_t *wpa_s, network_profile_t *network) {
    struct iwreq wrq;
    
    log_message(2, "Connecting to network: %s", network->ssid);
    wpa_s->current_network = network;
    wpa_s->state = WPA_ASSOCIATING;

    memset(&wrq, 0, sizeof(wrq));
    strncpy(wrq.ifr_name, wpa_s->interface, IFNAMSIZ);
    wrq.u.essid.pointer = network->ssid;
    wrq.u.essid.length = strlen(network->ssid);
    wrq.u.essid.flags = 1;

    if (ioctl(wpa_s->sock, SIOCSIWESSID, &wrq) < 0) {
        log_message(1, "Failed to set ESSID: %s", strerror(errno));
        return -1;
    }

    if (network->auth_type != AUTH_OPEN) {
        memset(&wrq, 0, sizeof(wrq));
        strncpy(wrq.ifr_name, wpa_s->interface, IFNAMSIZ);
        wrq.u.data.pointer = (caddr_t)network->passphrase;
        wrq.u.data.length = strlen(network->passphrase);
        wrq.u.data.flags = IW_ENCODE_RESTRICTED;

        if (ioctl(wpa_s->sock, SIOCSIWENCODE, &wrq) < 0) {
            log_message(1, "Failed to set encryption key: %s", strerror(errno));
        }
    }

    return 0;
}

static void *event_handler(void *arg) {
    wpa_supplicant_t *wpa_s = (wpa_supplicant_t *)arg;
    
    while (wpa_s->running) {
        sleep(1);
        
        pthread_mutex_lock(&wpa_s->mutex);
        
        switch (wpa_s->state) {
            case WPA_DISCONNECTED:
                if (wpa_s->auto_connect) {
                    trigger_scan(wpa_s);
                }
                break;
            
            case WPA_SCANNING:
                if (get_scan_results(wpa_s) == 0) {
                    network_profile_t *network = select_network(wpa_s);
                    if (network) {
                        connect_to_network(wpa_s, network);
                    } else {
                        wpa_s->state = WPA_DISCONNECTED;
                    }
                }
                break;
            
            case WPA_ASSOCIATING:
                wpa_s->state = WPA_4WAY_HANDSHAKE;
                break;
            
            case WPA_4WAY_HANDSHAKE:
                wpa_s->state = WPA_COMPLETED;
                log_message(1, "Successfully connected to %s", 
                    wpa_s->current_network ? wpa_s->current_network->ssid : "unknown");
                break;
            
            case WPA_COMPLETED:
                break;
            
            default:
                break;
        }
        
        pthread_mutex_unlock(&wpa_s->mutex);
    }
    
    return NULL;
}

static void *scan_handler(void *arg) {
    wpa_supplicant_t *wpa_s = (wpa_supplicant_t *)arg;
    
    while (wpa_s->running) {
        sleep(wpa_s->scan_interval);
        
        pthread_mutex_lock(&wpa_s->mutex);
        if (wpa_s->state == WPA_DISCONNECTED) {
            trigger_scan(wpa_s);
        }
        pthread_mutex_unlock(&wpa_s->mutex);
    }
    
    return NULL;
}

static void signal_handler(int sig) {
    if (global_wpa_s) {
        log_message(1, "Received signal %d, shutting down", sig);
        global_wpa_s->running = 0;
    }
}

static void usage(const char *prog) {
    printf("wpa_supplicant v%s\n", WPA_VERSION);
    printf("Copyright (c) 2003-2024, Jouni Malinen <j@w1.fi> and contributors\n\n");
    printf("Usage: %s [options]\n", prog);
    printf("Options:\n");
    printf("  -i <ifname>  Interface name\n");
    printf("  -c <config>  Configuration file\n");
    printf("  -D <driver>  Driver name (nl80211, wext, etc.)\n");
    printf("  -B           Run in background (daemon mode)\n");
    printf("  -d           Increase debugging level\n");
    printf("  -K           Include key data in debug output\n");
    printf("  -f <file>    Log output to file\n");
    printf("  -P <file>    PID file\n");
    printf("  -h           Show this help text\n");
}

int main(int argc, char *argv[]) {
    wpa_supplicant_t wpa_s;
    memset(&wpa_s, 0, sizeof(wpa_s));
    
    strncpy(wpa_s.interface, "wlan0", IFNAMSIZ);
    wpa_s.scan_interval = SCAN_INTERVAL;
    wpa_s.auto_connect = 1;
    wpa_s.debug_level = 2;
    wpa_s.running = 1;
    
    const char *config_file = CONFIG_FILE;
    int daemonize = 0;
    int opt;
    
    while ((opt = getopt(argc, argv, "i:c:D:Bdf:P:hK")) != -1) {
        switch (opt) {
            case 'i':
                strncpy(wpa_s.interface, optarg, IFNAMSIZ - 1);
                break;
            case 'c':
                config_file = optarg;
                break;
            case 'D':
                break;
            case 'B':
                daemonize = 1;
                break;
            case 'd':
                wpa_s.debug_level++;
                break;
            case 'f':
                freopen(optarg, "w", stdout);
                break;
            case 'P':
                break;
            case 'K':
                break;
            case 'h':
                usage(argv[0]);
                return 0;
            default:
                usage(argv[0]);
                return 1;
        }
    }
    
    global_wpa_s = &wpa_s;
    
    signal(SIGINT, signal_handler);
    signal(SIGTERM, signal_handler);
    
    if (daemonize) {
        if (daemon(0, 0) < 0) {
            log_message(1, "Failed to daemonize: %s", strerror(errno));
            return 1;
        }
    }
    
    log_message(1, "wpa_supplicant v%s starting", WPA_VERSION);
    
    if (parse_config_file(&wpa_s, config_file) < 0) {
        log_message(1, "Failed to parse configuration file");
        return 1;
    }
    
    if (init_wireless_socket(&wpa_s) < 0) {
        log_message(1, "Failed to initialize wireless socket");
        return 1;
    }
    
    pthread_mutex_init(&wpa_s.mutex, NULL);
    
    if (pthread_create(&wpa_s.event_thread, NULL, event_handler, &wpa_s) != 0) {
        log_message(1, "Failed to create event thread");
        return 1;
    }
    
    if (pthread_create(&wpa_s.scan_thread, NULL, scan_handler, &wpa_s) != 0) {
        log_message(1, "Failed to create scan thread");
        return 1;
    }
    
    pthread_join(wpa_s.event_thread, NULL);
    pthread_join(wpa_s.scan_thread, NULL);
    
    close(wpa_s.sock);
    if (wpa_s.ctrl_sock > 0) {
        close(wpa_s.ctrl_sock);
    }
    
    pthread_mutex_destroy(&wpa_s.mutex);
    
    network_profile_t *net = wpa_s.networks;
    while (net) {
        network_profile_t *tmp = net;
        net = net->next;
        free(tmp);
    }
    
    scan_result_t *scan = wpa_s.scan_results;
    while (scan) {
        scan_result_t *tmp = scan;
        scan = scan->next;
        free(tmp);
    }
    
    log_message(1, "wpa_supplicant terminated");
    
    return 0;
}