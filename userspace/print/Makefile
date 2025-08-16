RUSTC = rustc
RUSTFLAGS = -C opt-level=2

TARGETS = lp lpstat lpadmin lpq lprm

all: $(TARGETS)

lp: lp.rs
	$(RUSTC) $(RUSTFLAGS) $< -o $@

lpstat: lpstat.rs
	$(RUSTC) $(RUSTFLAGS) $< -o $@

lpadmin: lpadmin.rs
	$(RUSTC) $(RUSTFLAGS) $< -o $@

lpq: lpq.rs
	$(RUSTC) $(RUSTFLAGS) $< -o $@

lprm: lprm.rs
	$(RUSTC) $(RUSTFLAGS) $< -o $@

clean:
	rm -f $(TARGETS)

install: all
	mkdir -p /usr/local/bin
	cp $(TARGETS) /usr/local/bin/

.PHONY: all clean install