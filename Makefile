# FlexiFlow
# See LICENSE file for copyright and license details.

PREFIX = /usr
SHAREPREFIX = /usr/share

BIN = target/release/FlexiFlow
BAR = target/release/flexiflow_bar

all: options ${BIN} ${BAR}

options:
	@echo FlexiFlow build options:
	@echo "PREFIX      = ${PREFIX}"
	@echo "SHAREPREFIX = ${SHAREPREFIX}"

# marked .PHONY below so this always runs `cargo build` and lets
# Cargo's own dependency tracking (which also covers include_str!'d
# files like bar.lua) decide what's actually stale, instead of Make
# only rebuilding when these filenames don't exist yet
${BIN} ${BAR} &:
	cargo build --release

clean:
	cargo clean

install: all
	mkdir -p ${DESTDIR}${PREFIX}/bin
	mkdir -p ${DESTDIR}${SHAREPREFIX}/xsessions
	install -m 755 ${BIN} ${DESTDIR}${PREFIX}/bin/FlexiFlow
	install -m 755 ${BAR} ${DESTDIR}${PREFIX}/bin/flexiflow_bar
	install -m 644 FlexiFlow.desktop ${DESTDIR}${SHAREPREFIX}/xsessions/

uninstall:
	# layoutmenu is no longer installed (FlexiFlow pipes straight into
	# xmenu now) but is still removed here to clean up older installs
	rm -f ${DESTDIR}${PREFIX}/bin/FlexiFlow\
		${DESTDIR}${PREFIX}/bin/flexiflow_bar\
		${DESTDIR}${PREFIX}/bin/layoutmenu\
		${DESTDIR}${SHAREPREFIX}/xsessions/FlexiFlow.desktop

.PHONY: all options clean install uninstall ${BIN} ${BAR}
