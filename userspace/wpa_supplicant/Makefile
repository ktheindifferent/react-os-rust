CC = gcc
CFLAGS = -Wall -O2 -pthread
LDFLAGS = -pthread -lm

TARGETS = wpa_supplicant iwconfig iwlist

all: $(TARGETS)

wpa_supplicant: main.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS)

iwconfig: iwconfig.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS)

iwlist: iwlist.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS)

install: $(TARGETS)
	install -d $(DESTDIR)/usr/sbin
	install -d $(DESTDIR)/etc
	install -m 755 wpa_supplicant $(DESTDIR)/usr/sbin/
	install -m 755 iwconfig $(DESTDIR)/usr/sbin/
	install -m 755 iwlist $(DESTDIR)/usr/sbin/
	install -m 644 wpa_supplicant.conf $(DESTDIR)/etc/

clean:
	rm -f $(TARGETS) *.o

.PHONY: all install clean