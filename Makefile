# ABOUTME: Top-level build orchestration for the dot-viewer project.
# ABOUTME: Coordinates Rust library build, Swift binding generation, and Xcode build.

.PHONY: all build-core generate-bindings build-app clean

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

clean:
	cd dot-core && cargo clean
	rm -rf DotViewer/DotViewer/Generated
