# ABOUTME: Top-level build orchestration for the dot-viewer project.
# ABOUTME: Coordinates Rust library build, Swift binding generation, Xcode build, and web app build.

.PHONY: all build-core generate-bindings build-app clean build-wasm web-install web-dev web-build web-test

all: build-app

build-core:
	cd dot-core && cargo build --release

generate-bindings: build-core
	bash scripts/generate-bindings.sh

build-app: generate-bindings
	xcodebuild -project DotViewer/DotViewer.xcodeproj \
		-scheme DotViewer \
		-configuration Release \
		build

build-wasm:
	cd dot-core-wasm && wasm-pack build --target web --release

web-install: build-wasm
	cd web && npm install

web-dev: web-install
	cd web && npm run dev

web-build: web-install
	cd web && npm run build

web-test: web-build
	cd web && npx playwright test

clean:
	cd dot-core && cargo clean
	rm -rf DotViewer/DotViewer/Generated
	rm -rf dot-core-wasm/pkg
	rm -rf web/node_modules web/.svelte-kit web/build
