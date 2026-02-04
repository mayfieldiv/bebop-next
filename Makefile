BUILD_DIR := build
CMAKE_DIR := $(BUILD_DIR)/cmake
DIST_DIR := dist
JOBS ?= $(shell nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)

VERSION := $(shell cat VERSION)
VERSION_BASE := $(firstword $(subst -, ,$(VERSION)))
VERSION_MAJOR := $(word 1,$(subst ., ,$(VERSION_BASE)))
VERSION_MINOR := $(word 2,$(subst ., ,$(VERSION_BASE)))
VERSION_PATCH := $(word 3,$(subst ., ,$(VERSION_BASE)))
VERSION_SUFFIX := $(word 2,$(subst -, ,$(VERSION)))

OS := $(shell uname -s | tr '[:upper:]' '[:lower:]')
ARCH := $(shell uname -m)

.PHONY: all debug release test clean publish dist vscode archive

all: release

debug:
	@cmake -B $(CMAKE_DIR) -DCMAKE_BUILD_TYPE=Debug \
		-DBEBOP_VERSION_MAJOR=$(VERSION_MAJOR) \
		-DBEBOP_VERSION_MINOR=$(VERSION_MINOR) \
		-DBEBOP_VERSION_PATCH=$(VERSION_PATCH) \
		-DBEBOP_VERSION_SUFFIX=$(VERSION_SUFFIX)
	@cmake --build $(CMAKE_DIR) -j$(JOBS)

release:
	@cmake -B $(CMAKE_DIR) -DCMAKE_BUILD_TYPE=Release \
		-DBEBOP_VERSION_MAJOR=$(VERSION_MAJOR) \
		-DBEBOP_VERSION_MINOR=$(VERSION_MINOR) \
		-DBEBOP_VERSION_PATCH=$(VERSION_PATCH) \
		-DBEBOP_VERSION_SUFFIX=$(VERSION_SUFFIX)
	@cmake --build $(CMAKE_DIR) -j$(JOBS)

test: debug
	@ctest --test-dir $(CMAKE_DIR) --output-on-failure

clean:
	rm -rf $(BUILD_DIR) $(DIST_DIR) bebop-*.tar.gz

publish: release dist archive
	@echo "Published bebop-$(VERSION)-$(OS)-$(ARCH).tar.gz"

dist:
	@rm -rf $(DIST_DIR)
	@mkdir -p $(DIST_DIR)/bin $(DIST_DIR)/lib $(DIST_DIR)/include $(DIST_DIR)/share/bebop
	@cp $(BUILD_DIR)/bin/bebopc $(DIST_DIR)/bin/
	@cp $(BUILD_DIR)/bin/bebopc-gen-* $(DIST_DIR)/bin/ 2>/dev/null || true
	@cp $(BUILD_DIR)/lib/libbebop.a $(DIST_DIR)/lib/
	@cp $(BUILD_DIR)/include/*.h $(DIST_DIR)/include/
	@cp $(BUILD_DIR)/share/bebop/*.bop $(DIST_DIR)/share/bebop/
	@if ls plugins/vscode/*.vsix 1>/dev/null 2>&1; then \
		mkdir -p $(DIST_DIR)/ide/vscode && \
		cp plugins/vscode/*.vsix $(DIST_DIR)/ide/vscode/; \
	fi

vscode:
	cd plugins/vscode && npm install && npm run compile && npm run package

archive:
	@tar -czf bebop-$(VERSION)-$(OS)-$(ARCH).tar.gz -C $(DIST_DIR) .
