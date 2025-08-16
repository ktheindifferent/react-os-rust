#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <errno.h>
#include <sys/socket.h>
#include <sys/ioctl.h>
#include <net/if.h>
#include <linux/wireless.h>
#include <math.h>

#define VERSION "30"

static int sock = -1;

static void print_usage(void) {
    printf("Usage: iwconfig [interface]\n");
    printf("       iwconfig interface [essid X] [mode X] [freq N] [channel N]\n");
    printf("                          [sens X] [nick N] [rate X] [rts X] [frag X]\n");
    printf("                          [txpower X] [retry X] [key X] [power X]\n");
    printf("                          [commit]\n");
}

static void format_frequency(double freq, char *buf, size_t buflen) {
    if (freq < 1e3) {
        snprintf(buf, buflen, "%g Hz", freq);
    } else if (freq < 1e6) {
        snprintf(buf, buflen, "%g kHz", freq / 1e3);
    } else if (freq < 1e9) {
        snprintf(buf, buflen, "%g MHz", freq / 1e6);
    } else {
        snprintf(buf, buflen, "%g GHz", freq / 1e9);
    }
}

static void format_bitrate(double rate, char *buf, size_t buflen) {
    if (rate < 1e3) {
        snprintf(buf, buflen, "%g bit/s", rate);
    } else if (rate < 1e6) {
        snprintf(buf, buflen, "%g kb/s", rate / 1e3);
    } else if (rate < 1e9) {
        snprintf(buf, buflen, "%g Mb/s", rate / 1e6);
    } else {
        snprintf(buf, buflen, "%g Gb/s", rate / 1e9);
    }
}

static int dbm_to_quality(int dbm) {
    if (dbm >= -50) return 100;
    if (dbm <= -100) return 0;
    return 2 * (dbm + 100);
}

static void print_interface_info(const char *ifname) {
    struct iwreq wrq;
    char essid[IW_ESSID_MAX_SIZE + 1] = {0};
    char freq_buf[32], rate_buf[32];
    
    memset(&wrq, 0, sizeof(wrq));
    strncpy(wrq.ifr_name, ifname, IFNAMSIZ);
    
    printf("%-10s", ifname);
    
    wrq.u.essid.pointer = essid;
    wrq.u.essid.length = IW_ESSID_MAX_SIZE;
    wrq.u.essid.flags = 0;
    if (ioctl(sock, SIOCGIWESSID, &wrq) >= 0) {
        if (wrq.u.essid.flags) {
            essid[wrq.u.essid.length] = '\0';
            printf("  ESSID:\"%s\"", essid);
        } else {
            printf("  ESSID:off/any");
        }
    }
    
    if (ioctl(sock, SIOCGIWMODE, &wrq) >= 0) {
        switch (wrq.u.mode) {
            case IW_MODE_AUTO:
                printf("  Mode:Auto");
                break;
            case IW_MODE_ADHOC:
                printf("  Mode:Ad-Hoc");
                break;
            case IW_MODE_INFRA:
                printf("  Mode:Managed");
                break;
            case IW_MODE_MASTER:
                printf("  Mode:Master");
                break;
            case IW_MODE_REPEAT:
                printf("  Mode:Repeater");
                break;
            case IW_MODE_SECOND:
                printf("  Mode:Secondary");
                break;
            case IW_MODE_MONITOR:
                printf("  Mode:Monitor");
                break;
            default:
                printf("  Mode:Unknown");
        }
    }
    
    printf("\n          ");
    
    if (ioctl(sock, SIOCGIWFREQ, &wrq) >= 0) {
        double freq = wrq.u.freq.m * pow(10, wrq.u.freq.e);
        format_frequency(freq, freq_buf, sizeof(freq_buf));
        printf("  Frequency:%s", freq_buf);
        
        if (freq >= 2.4e9 && freq <= 2.5e9) {
            int channel = (int)((freq - 2.407e9) / 5e6);
            if (channel >= 1 && channel <= 14) {
                printf(" (Channel %d)", channel);
            }
        } else if (freq >= 5e9 && freq <= 6e9) {
            int channel = (int)((freq - 5e9) / 5e6);
            printf(" (Channel %d)", channel);
        }
    }
    
    if (ioctl(sock, SIOCGIWAP, &wrq) >= 0) {
        unsigned char *ap = (unsigned char *)wrq.u.ap_addr.sa_data;
        if (ap[0] || ap[1] || ap[2] || ap[3] || ap[4] || ap[5]) {
            printf("  Access Point: %02X:%02X:%02X:%02X:%02X:%02X",
                ap[0], ap[1], ap[2], ap[3], ap[4], ap[5]);
        } else {
            printf("  Access Point: Not-Associated");
        }
    }
    
    printf("\n          ");
    
    if (ioctl(sock, SIOCGIWRATE, &wrq) >= 0) {
        format_bitrate(wrq.u.bitrate.value, rate_buf, sizeof(rate_buf));
        printf("  Bit Rate:%s", rate_buf);
    }
    
    if (ioctl(sock, SIOCGIWTXPOW, &wrq) >= 0) {
        if (wrq.u.txpower.disabled) {
            printf("  Tx-Power:off");
        } else {
            int dbm = wrq.u.txpower.value;
            if (wrq.u.txpower.flags & IW_TXPOW_MWATT) {
                dbm = (int)(10.0 * log10((double)wrq.u.txpower.value));
            }
            printf("  Tx-Power:%d dBm", dbm);
        }
    }
    
    if (ioctl(sock, SIOCGIWRETRY, &wrq) >= 0) {
        if (wrq.u.retry.disabled) {
            printf("  Retry:off");
        } else {
            printf("  Retry limit:%d", wrq.u.retry.value);
        }
    }
    
    printf("\n          ");
    
    if (ioctl(sock, SIOCGIWRTS, &wrq) >= 0) {
        if (wrq.u.rts.disabled) {
            printf("  RTS thr:off");
        } else {
            printf("  RTS thr:%d B", wrq.u.rts.value);
        }
    }
    
    if (ioctl(sock, SIOCGIWFRAG, &wrq) >= 0) {
        if (wrq.u.frag.disabled) {
            printf("  Fragment thr:off");
        } else {
            printf("  Fragment thr:%d B", wrq.u.frag.value);
        }
    }
    
    printf("\n          ");
    
    char key[IW_ENCODING_TOKEN_MAX + 1];
    wrq.u.data.pointer = key;
    wrq.u.data.length = IW_ENCODING_TOKEN_MAX;
    wrq.u.data.flags = 0;
    if (ioctl(sock, SIOCGIWENCODE, &wrq) >= 0) {
        if (wrq.u.data.flags & IW_ENCODE_DISABLED) {
            printf("  Encryption key:off");
        } else {
            printf("  Encryption key:****");
            if (wrq.u.data.flags & IW_ENCODE_RESTRICTED) {
                printf("   Security mode:restricted");
            } else if (wrq.u.data.flags & IW_ENCODE_OPEN) {
                printf("   Security mode:open");
            }
        }
    }
    
    printf("\n          ");
    
    if (ioctl(sock, SIOCGIWPOWER, &wrq) >= 0) {
        if (wrq.u.power.disabled) {
            printf("  Power Management:off");
        } else {
            printf("  Power Management:on");
            if (wrq.u.power.flags & IW_POWER_TYPE) {
                if (wrq.u.power.flags & IW_POWER_MIN) {
                    printf(" min");
                } else if (wrq.u.power.flags & IW_POWER_MAX) {
                    printf(" max");
                }
            }
            if (wrq.u.power.flags & IW_POWER_PERIOD) {
                printf(" period:%dus", wrq.u.power.value);
            } else if (wrq.u.power.flags & IW_POWER_TIMEOUT) {
                printf(" timeout:%dus", wrq.u.power.value);
            }
        }
    }
    
    printf("\n          ");
    
    struct iw_statistics stats;
    wrq.u.data.pointer = &stats;
    wrq.u.data.length = sizeof(stats);
    wrq.u.data.flags = 1;
    if (ioctl(sock, SIOCGIWSTATS, &wrq) >= 0) {
        int quality = dbm_to_quality(stats.qual.level - 256);
        printf("  Link Quality=%d/100", quality);
        printf("  Signal level=%d dBm", stats.qual.level - 256);
        if (stats.qual.noise) {
            printf("  Noise level=%d dBm", stats.qual.noise - 256);
        }
    }
    
    printf("\n          ");
    
    struct ifreq ifr;
    memset(&ifr, 0, sizeof(ifr));
    strncpy(ifr.ifr_name, ifname, IFNAMSIZ);
    if (ioctl(sock, SIOCGIFFLAGS, &ifr) >= 0) {
        if (ifr.ifr_flags & IFF_UP) {
            printf("  Interface UP");
        } else {
            printf("  Interface DOWN");
        }
        if (ifr.ifr_flags & IFF_RUNNING) {
            printf(" RUNNING");
        }
    }
    
    printf("\n\n");
}

static int set_essid(const char *ifname, const char *essid) {
    struct iwreq wrq;
    
    memset(&wrq, 0, sizeof(wrq));
    strncpy(wrq.ifr_name, ifname, IFNAMSIZ);
    
    if (strcmp(essid, "off") == 0 || strcmp(essid, "any") == 0) {
        wrq.u.essid.flags = 0;
        wrq.u.essid.pointer = NULL;
        wrq.u.essid.length = 0;
    } else {
        wrq.u.essid.pointer = (caddr_t)essid;
        wrq.u.essid.length = strlen(essid);
        wrq.u.essid.flags = 1;
    }
    
    if (ioctl(sock, SIOCSIWESSID, &wrq) < 0) {
        fprintf(stderr, "Error setting ESSID: %s\n", strerror(errno));
        return -1;
    }
    
    return 0;
}

static int set_mode(const char *ifname, const char *mode) {
    struct iwreq wrq;
    
    memset(&wrq, 0, sizeof(wrq));
    strncpy(wrq.ifr_name, ifname, IFNAMSIZ);
    
    if (strcasecmp(mode, "managed") == 0 || strcasecmp(mode, "station") == 0) {
        wrq.u.mode = IW_MODE_INFRA;
    } else if (strcasecmp(mode, "ad-hoc") == 0 || strcasecmp(mode, "adhoc") == 0) {
        wrq.u.mode = IW_MODE_ADHOC;
    } else if (strcasecmp(mode, "master") == 0 || strcasecmp(mode, "ap") == 0) {
        wrq.u.mode = IW_MODE_MASTER;
    } else if (strcasecmp(mode, "monitor") == 0) {
        wrq.u.mode = IW_MODE_MONITOR;
    } else if (strcasecmp(mode, "repeater") == 0) {
        wrq.u.mode = IW_MODE_REPEAT;
    } else if (strcasecmp(mode, "auto") == 0) {
        wrq.u.mode = IW_MODE_AUTO;
    } else {
        fprintf(stderr, "Error: Invalid mode '%s'\n", mode);
        return -1;
    }
    
    if (ioctl(sock, SIOCSIWMODE, &wrq) < 0) {
        fprintf(stderr, "Error setting mode: %s\n", strerror(errno));
        return -1;
    }
    
    return 0;
}

static int set_channel(const char *ifname, int channel) {
    struct iwreq wrq;
    
    memset(&wrq, 0, sizeof(wrq));
    strncpy(wrq.ifr_name, ifname, IFNAMSIZ);
    
    double freq;
    if (channel <= 14) {
        freq = 2.407e9 + channel * 5e6;
    } else {
        freq = 5e9 + channel * 5e6;
    }
    
    wrq.u.freq.m = (int)freq;
    wrq.u.freq.e = 0;
    wrq.u.freq.flags = IW_FREQ_FIXED;
    
    if (ioctl(sock, SIOCSIWFREQ, &wrq) < 0) {
        fprintf(stderr, "Error setting channel: %s\n", strerror(errno));
        return -1;
    }
    
    return 0;
}

static int set_txpower(const char *ifname, const char *power) {
    struct iwreq wrq;
    
    memset(&wrq, 0, sizeof(wrq));
    strncpy(wrq.ifr_name, ifname, IFNAMSIZ);
    
    if (strcmp(power, "off") == 0) {
        wrq.u.txpower.disabled = 1;
    } else if (strcmp(power, "auto") == 0) {
        wrq.u.txpower.disabled = 0;
        wrq.u.txpower.fixed = 0;
    } else {
        char *endptr;
        int dbm = strtol(power, &endptr, 10);
        
        if (*endptr != '\0') {
            fprintf(stderr, "Error: Invalid txpower value '%s'\n", power);
            return -1;
        }
        
        wrq.u.txpower.disabled = 0;
        wrq.u.txpower.fixed = 1;
        wrq.u.txpower.value = dbm;
        wrq.u.txpower.flags = IW_TXPOW_DBM;
    }
    
    if (ioctl(sock, SIOCSIWTXPOW, &wrq) < 0) {
        fprintf(stderr, "Error setting txpower: %s\n", strerror(errno));
        return -1;
    }
    
    return 0;
}

static int set_key(const char *ifname, const char *key) {
    struct iwreq wrq;
    
    memset(&wrq, 0, sizeof(wrq));
    strncpy(wrq.ifr_name, ifname, IFNAMSIZ);
    
    if (strcmp(key, "off") == 0) {
        wrq.u.data.flags = IW_ENCODE_DISABLED;
        wrq.u.data.pointer = NULL;
        wrq.u.data.length = 0;
    } else {
        wrq.u.data.pointer = (caddr_t)key;
        wrq.u.data.length = strlen(key);
        wrq.u.data.flags = IW_ENCODE_RESTRICTED;
    }
    
    if (ioctl(sock, SIOCSIWENCODE, &wrq) < 0) {
        fprintf(stderr, "Error setting encryption key: %s\n", strerror(errno));
        return -1;
    }
    
    return 0;
}

int main(int argc, char *argv[]) {
    sock = socket(AF_INET, SOCK_DGRAM, 0);
    if (sock < 0) {
        fprintf(stderr, "Error: Cannot open socket: %s\n", strerror(errno));
        return 1;
    }
    
    if (argc == 1) {
        struct if_nameindex *if_ni, *i;
        
        if_ni = if_nameindex();
        if (if_ni == NULL) {
            fprintf(stderr, "Error: Cannot get interface list\n");
            close(sock);
            return 1;
        }
        
        for (i = if_ni; i->if_index && i->if_name; i++) {
            if (strncmp(i->if_name, "wlan", 4) == 0 ||
                strncmp(i->if_name, "ath", 3) == 0 ||
                strncmp(i->if_name, "wifi", 4) == 0 ||
                strncmp(i->if_name, "wl", 2) == 0) {
                print_interface_info(i->if_name);
            }
        }
        
        if_freenameindex(if_ni);
    } else if (argc == 2) {
        print_interface_info(argv[1]);
    } else {
        const char *ifname = argv[1];
        int i = 2;
        
        while (i < argc) {
            if (strcmp(argv[i], "essid") == 0) {
                if (i + 1 >= argc) {
                    fprintf(stderr, "Error: essid requires an argument\n");
                    close(sock);
                    return 1;
                }
                if (set_essid(ifname, argv[i + 1]) < 0) {
                    close(sock);
                    return 1;
                }
                i += 2;
            } else if (strcmp(argv[i], "mode") == 0) {
                if (i + 1 >= argc) {
                    fprintf(stderr, "Error: mode requires an argument\n");
                    close(sock);
                    return 1;
                }
                if (set_mode(ifname, argv[i + 1]) < 0) {
                    close(sock);
                    return 1;
                }
                i += 2;
            } else if (strcmp(argv[i], "channel") == 0) {
                if (i + 1 >= argc) {
                    fprintf(stderr, "Error: channel requires an argument\n");
                    close(sock);
                    return 1;
                }
                if (set_channel(ifname, atoi(argv[i + 1])) < 0) {
                    close(sock);
                    return 1;
                }
                i += 2;
            } else if (strcmp(argv[i], "txpower") == 0) {
                if (i + 1 >= argc) {
                    fprintf(stderr, "Error: txpower requires an argument\n");
                    close(sock);
                    return 1;
                }
                if (set_txpower(ifname, argv[i + 1]) < 0) {
                    close(sock);
                    return 1;
                }
                i += 2;
            } else if (strcmp(argv[i], "key") == 0) {
                if (i + 1 >= argc) {
                    fprintf(stderr, "Error: key requires an argument\n");
                    close(sock);
                    return 1;
                }
                if (set_key(ifname, argv[i + 1]) < 0) {
                    close(sock);
                    return 1;
                }
                i += 2;
            } else if (strcmp(argv[i], "commit") == 0) {
                i++;
            } else {
                fprintf(stderr, "Error: Unknown parameter '%s'\n", argv[i]);
                print_usage();
                close(sock);
                return 1;
            }
        }
    }
    
    close(sock);
    return 0;
}