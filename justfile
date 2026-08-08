# Justfile for serialforge cross-compilation & packaging

image_tag := "serialforge-builder:26.04"

# Default action: Build all release artifacts
default: build-all

# Build the Ubuntu 26.04 Docker builder image
docker-build:
    docker build -t {{image_tag}} .

# Cross-compile native Linux binary
build-linux: docker-build
    docker run --rm -v $(pwd):/workspace {{image_tag}} cargo build --release --target x86_64-unknown-linux-gnu

# Cross-compile Windows binary
build-windows: docker-build
    docker run --rm -v $(pwd):/workspace {{image_tag}} cargo build --release --target x86_64-pc-windows-gnu

# Package Linux binary into an AppImage
build-appimage: build-linux
    docker run --rm -v $(pwd):/workspace {{image_tag}} /bin/bash ./scripts/build-appimage.sh

# Build Debian package (.deb) using cargo-deb
build-deb: build-linux
    docker run --rm -v $(pwd):/workspace {{image_tag}} cargo deb --target x86_64-unknown-linux-gnu --no-build

# Build all targets: Linux binary, Windows binary, AppImage, and Debian package
build-all: build-linux build-windows build-appimage build-deb
    @echo "=== All build targets generated successfully! ==="
    @echo "Linux Binary:    target/x86_64-unknown-linux-gnu/release/serialforge"
    @echo "Windows Binary:  target/x86_64-pc-windows-gnu/release/serialforge.exe"
    @echo "Linux AppImage:  serialforge-x86_64.AppImage"
    @echo "Debian Package:  target/x86_64-unknown-linux-gnu/debian/*.deb"

# Clean build directory
clean:
    cargo clean
    rm -rf AppDir *.AppImage