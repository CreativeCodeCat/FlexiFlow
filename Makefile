# flexiflow-rs - a fork of rwm (itself a port of dwm) carrying the
# flexiflow window manager config.
# See LICENSE file for copyright and license details.

PREFIX = /usr
SHAREPREFIX = /usr/share

BIN = target/release/flexiflow-rs

all: options ${BIN}

options:
	@echo flexiflow-rs build options:
	@echo "PREFIX      = ${PREFIX}"
	@echo "SHAREPREFIX = ${SHAREPREFIX}"

${BIN}:
	cargo build --release

clean:
	cargo clean

install: all
	mkdir -p ${DESTDIR}${PREFIX}/bin
	mkdir -p ${DESTDIR}${SHAREPREFIX}/xsessions
	install -m 755 ${BIN} ${DESTDIR}${PREFIX}/bin/flexiflow-rs
	install -m 755 scripts/flexiflow_bar ${DESTDIR}${PREFIX}/bin/flexiflow_bar
	install -m 644 flexiflow-rs.desktop ${DESTDIR}${SHAREPREFIX}/xsessions/

uninstall:
	rm -f ${DESTDIR}${PREFIX}/bin/flexiflow-rs\
		${DESTDIR}${PREFIX}/bin/flexiflow_bar\
		${DESTDIR}${SHAREPREFIX}/xsessions/flexiflow-rs.desktop

.PHONY: all options clean install uninstall
