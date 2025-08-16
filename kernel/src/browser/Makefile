# Browser Engine Makefile

CC = gcc
CFLAGS = -Wall -Wextra -O2 -fPIC -I. -I.. -I../.. 
LDFLAGS = -shared -lm -lpthread -lssl -lcrypto -lz

# Source directories
HTML_DIR = html
CSS_DIR = css
JS_DIR = js
RENDER_DIR = render
WEBAPI_DIR = webapi
SECURITY_DIR = security
NETWORK_DIR = network

# Object files
OBJS = engine.o \
       $(HTML_DIR)/parser.o \
       $(HTML_DIR)/dom.o \
       $(HTML_DIR)/tokenizer.o \
       $(CSS_DIR)/parser.o \
       $(CSS_DIR)/style.o \
       $(CSS_DIR)/selector.o \
       $(CSS_DIR)/cascade.o \
       $(JS_DIR)/engine.o \
       $(JS_DIR)/parser.o \
       $(JS_DIR)/runtime.o \
       $(JS_DIR)/gc.o \
       $(RENDER_DIR)/engine.o \
       $(RENDER_DIR)/layout.o \
       $(RENDER_DIR)/paint.o \
       $(RENDER_DIR)/compositor.o \
       $(WEBAPI_DIR)/fetch.o \
       $(WEBAPI_DIR)/websocket.o \
       $(WEBAPI_DIR)/canvas.o \
       $(WEBAPI_DIR)/webgl.o \
       $(WEBAPI_DIR)/storage.o \
       $(WEBAPI_DIR)/worker.o \
       $(SECURITY_DIR)/csp.o \
       $(SECURITY_DIR)/cors.o \
       $(SECURITY_DIR)/sandbox.o \
       $(SECURITY_DIR)/ssl.o \
       $(NETWORK_DIR)/http.o \
       $(NETWORK_DIR)/cache.o \
       $(NETWORK_DIR)/cookies.o

# Library name
TARGET = libbrowser.so

# Build rules
all: $(TARGET)

$(TARGET): $(OBJS)
	$(CC) $(LDFLAGS) -o $@ $^

%.o: %.c
	$(CC) $(CFLAGS) -c -o $@ $<

# HTML components
$(HTML_DIR)/parser.o: $(HTML_DIR)/parser.c $(HTML_DIR)/parser.h
	$(CC) $(CFLAGS) -c -o $@ $<

$(HTML_DIR)/dom.o: $(HTML_DIR)/dom.c $(HTML_DIR)/dom.h
	$(CC) $(CFLAGS) -c -o $@ $<

$(HTML_DIR)/tokenizer.o: $(HTML_DIR)/tokenizer.c $(HTML_DIR)/parser.h
	$(CC) $(CFLAGS) -c -o $@ $<

# CSS components
$(CSS_DIR)/parser.o: $(CSS_DIR)/parser.c $(CSS_DIR)/parser.h
	$(CC) $(CFLAGS) -c -o $@ $<

$(CSS_DIR)/style.o: $(CSS_DIR)/style.c $(CSS_DIR)/style.h
	$(CC) $(CFLAGS) -c -o $@ $<

$(CSS_DIR)/selector.o: $(CSS_DIR)/selector.c $(CSS_DIR)/parser.h
	$(CC) $(CFLAGS) -c -o $@ $<

$(CSS_DIR)/cascade.o: $(CSS_DIR)/cascade.c $(CSS_DIR)/style.h
	$(CC) $(CFLAGS) -c -o $@ $<

# JavaScript components
$(JS_DIR)/engine.o: $(JS_DIR)/engine.c $(JS_DIR)/engine.h
	$(CC) $(CFLAGS) -c -o $@ $<

$(JS_DIR)/parser.o: $(JS_DIR)/parser.c $(JS_DIR)/engine.h
	$(CC) $(CFLAGS) -c -o $@ $<

$(JS_DIR)/runtime.o: $(JS_DIR)/runtime.c $(JS_DIR)/engine.h
	$(CC) $(CFLAGS) -c -o $@ $<

$(JS_DIR)/gc.o: $(JS_DIR)/gc.c $(JS_DIR)/engine.h
	$(CC) $(CFLAGS) -c -o $@ $<

# Rendering components
$(RENDER_DIR)/engine.o: $(RENDER_DIR)/engine.c $(RENDER_DIR)/engine.h
	$(CC) $(CFLAGS) -c -o $@ $<

$(RENDER_DIR)/layout.o: $(RENDER_DIR)/layout.c $(RENDER_DIR)/engine.h
	$(CC) $(CFLAGS) -c -o $@ $<

$(RENDER_DIR)/paint.o: $(RENDER_DIR)/paint.c $(RENDER_DIR)/engine.h
	$(CC) $(CFLAGS) -c -o $@ $<

$(RENDER_DIR)/compositor.o: $(RENDER_DIR)/compositor.c $(RENDER_DIR)/engine.h
	$(CC) $(CFLAGS) -c -o $@ $<

# Web API components
$(WEBAPI_DIR)/fetch.o: $(WEBAPI_DIR)/fetch.c $(WEBAPI_DIR)/fetch.h
	$(CC) $(CFLAGS) -c -o $@ $<

$(WEBAPI_DIR)/websocket.o: $(WEBAPI_DIR)/websocket.c $(WEBAPI_DIR)/websocket.h
	$(CC) $(CFLAGS) -c -o $@ $<

$(WEBAPI_DIR)/canvas.o: $(WEBAPI_DIR)/canvas.c $(WEBAPI_DIR)/canvas.h
	$(CC) $(CFLAGS) -c -o $@ $<

$(WEBAPI_DIR)/webgl.o: $(WEBAPI_DIR)/webgl.c $(WEBAPI_DIR)/webgl.h
	$(CC) $(CFLAGS) -c -o $@ $<

$(WEBAPI_DIR)/storage.o: $(WEBAPI_DIR)/storage.c $(WEBAPI_DIR)/storage.h
	$(CC) $(CFLAGS) -c -o $@ $<

$(WEBAPI_DIR)/worker.o: $(WEBAPI_DIR)/worker.c $(WEBAPI_DIR)/worker.h
	$(CC) $(CFLAGS) -c -o $@ $<

# Security components
$(SECURITY_DIR)/csp.o: $(SECURITY_DIR)/csp.c $(SECURITY_DIR)/csp.h
	$(CC) $(CFLAGS) -c -o $@ $<

$(SECURITY_DIR)/cors.o: $(SECURITY_DIR)/cors.c $(SECURITY_DIR)/csp.h
	$(CC) $(CFLAGS) -c -o $@ $<

$(SECURITY_DIR)/sandbox.o: $(SECURITY_DIR)/sandbox.c $(SECURITY_DIR)/csp.h
	$(CC) $(CFLAGS) -c -o $@ $<

$(SECURITY_DIR)/ssl.o: $(SECURITY_DIR)/ssl.c $(SECURITY_DIR)/csp.h
	$(CC) $(CFLAGS) -c -o $@ $<

# Network components
$(NETWORK_DIR)/http.o: $(NETWORK_DIR)/http.c $(NETWORK_DIR)/http.h
	$(CC) $(CFLAGS) -c -o $@ $<

$(NETWORK_DIR)/cache.o: $(NETWORK_DIR)/cache.c $(NETWORK_DIR)/http.h
	$(CC) $(CFLAGS) -c -o $@ $<

$(NETWORK_DIR)/cookies.o: $(NETWORK_DIR)/cookies.c $(NETWORK_DIR)/http.h
	$(CC) $(CFLAGS) -c -o $@ $<

# Installation
install: $(TARGET)
	install -d /usr/local/lib
	install -m 755 $(TARGET) /usr/local/lib/
	install -d /usr/local/include/browser
	install -m 644 engine.h /usr/local/include/browser/
	install -m 644 $(HTML_DIR)/*.h /usr/local/include/browser/html/
	install -m 644 $(CSS_DIR)/*.h /usr/local/include/browser/css/
	install -m 644 $(JS_DIR)/*.h /usr/local/include/browser/js/
	install -m 644 $(RENDER_DIR)/*.h /usr/local/include/browser/render/
	install -m 644 $(WEBAPI_DIR)/*.h /usr/local/include/browser/webapi/
	install -m 644 $(SECURITY_DIR)/*.h /usr/local/include/browser/security/
	ldconfig

# Clean
clean:
	rm -f $(OBJS) $(TARGET)
	rm -f $(HTML_DIR)/*.o
	rm -f $(CSS_DIR)/*.o
	rm -f $(JS_DIR)/*.o
	rm -f $(RENDER_DIR)/*.o
	rm -f $(WEBAPI_DIR)/*.o
	rm -f $(SECURITY_DIR)/*.o
	rm -f $(NETWORK_DIR)/*.o

# Debug build
debug: CFLAGS += -g -DDEBUG
debug: $(TARGET)

# Test build
test: $(TARGET)
	$(CC) $(CFLAGS) -o test_browser test_browser.c -L. -lbrowser
	./test_browser

.PHONY: all clean install debug test