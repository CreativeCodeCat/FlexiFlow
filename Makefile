# FlexiFlow: the X11 window manager and its river (Wayland) build
# See LICENSE file for copyright and license details.

PREFIX = /usr
SHAREPREFIX = /usr/share

BIN = target/release/FlexiFlow
BAR = target/release/flexiflow_bar
RIVER = target/release/flexiflow-river\
	target/release/flexiflow-river-bar\
	target/release/flexiflow-river-status\
	target/release/flexiflow-river-hotkeys

all: options ${BIN} ${BAR} ${RIVER}

options:
	@echo FlexiFlow build options:
	@echo "PREFIX      = ${PREFIX}"
	@echo "SHAREPREFIX = ${SHAREPREFIX}"

# marked .PHONY below so this always runs `cargo build` and lets
# Cargo's own dependency tracking (which also covers include_str!'d
# files like bar.lua) decide what's actually stale, instead of Make
# only rebuilding when these filenames don't exist yet
${BIN} ${BAR} ${RIVER} &:
	cargo build --release --workspace

clean:
	cargo clean

install: all
	mkdir -p ${DESTDIR}${PREFIX}/bin
	mkdir -p ${DESTDIR}${SHAREPREFIX}/xsessions
	mkdir -p ${DESTDIR}${SHAREPREFIX}/wayland-sessions
	install -m 755 ${BIN} ${DESTDIR}${PREFIX}/bin/FlexiFlow
	install -m 755 ${BAR} ${DESTDIR}${PREFIX}/bin/flexiflow_bar
	install -m 755 ${RIVER} src/flexiflow-river/flexiflow-shot ${DESTDIR}${PREFIX}/bin/
	install -m 644 src/flexiflow-x11/FlexiFlow.desktop ${DESTDIR}${SHAREPREFIX}/xsessions/
	install -m 644 src/flexiflow-river/flexiflow-river.desktop ${DESTDIR}${SHAREPREFIX}/wayland-sessions/
	# river under the name FlexiFlow, so fetch tools name the WM that
	# (see `flexiflow-river --session`)
	mkdir -p ${DESTDIR}${PREFIX}/lib/flexiflow-river
	ln -sf ${PREFIX}/bin/river ${DESTDIR}${PREFIX}/lib/flexiflow-river/FlexiFlow

uninstall:
	# layoutmenu is no longer installed (FlexiFlow pipes straight into
	# xmenu now) but is still removed here to clean up older installs
	rm -f ${DESTDIR}${PREFIX}/bin/FlexiFlow\
		${DESTDIR}${PREFIX}/bin/flexiflow_bar\
		${DESTDIR}${PREFIX}/bin/layoutmenu\
		${DESTDIR}${PREFIX}/bin/flexiflow-river\
		${DESTDIR}${PREFIX}/bin/flexiflow-river-bar\
		${DESTDIR}${PREFIX}/bin/flexiflow-river-status\
		${DESTDIR}${PREFIX}/bin/flexiflow-river-hotkeys\
		${DESTDIR}${PREFIX}/bin/flexiflow-shot\
		${DESTDIR}${SHAREPREFIX}/xsessions/FlexiFlow.desktop\
		${DESTDIR}${SHAREPREFIX}/wayland-sessions/flexiflow-river.desktop\
		${DESTDIR}${PREFIX}/lib/flexiflow-river/FlexiFlow
	-rmdir ${DESTDIR}${PREFIX}/lib/flexiflow-river

.PHONY: all options clean install uninstall ${BIN} ${BAR} ${RIVER}
