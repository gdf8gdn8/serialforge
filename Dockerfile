FROM ubuntu:26.04

ENV DEBIAN_FRONTEND=noninteractive

# Install build dependencies, MinGW cross-compiler, GUI libraries, fuse, and packaging tools
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    curl \
    git \
    gnupg \
    pkg-config \
    file \
    wget \
    ca-certificates \
    gcc-mingw-w64-x86-64 \
    g++-mingw-w64-x86-64 \
    mingw-w64-tools \
    libudev-dev \
    libx11-dev \
    libxcursor-dev \
    libxrandr-dev \
    libxinerama-dev \
    libxi-dev \
    libgl1-mesa-dev \
    libxkbcommon-dev \
    libfuse2t64 \
    squashfs-tools \
    && rm -rf /var/lib/apt/lists/*

# Download appimagetool
RUN curl -sSL https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage -o /tmp/appimagetool.AppImage \
    && chmod +x /tmp/appimagetool.AppImage \
    && (cd /tmp && ./appimagetool.AppImage --appimage-extract) \
    && mv /tmp/squashfs-root /usr/local/bin/appimagetool-extracted \
    && ln -s /usr/local/bin/appimagetool-extracted/AppRun /usr/local/bin/appimagetool-bin \
    && rm /tmp/appimagetool.AppImage

# Install Rust Toolchain
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable \
    && /root/.cargo/bin/rustup target add x86_64-unknown-linux-gnu \
    && /root/.cargo/bin/rustup target add x86_64-pc-windows-gnu

ENV PATH="/root/.cargo/bin:${PATH}"

# Install cargo-deb tool
RUN cargo install cargo-deb --locked

# Configure Cargo linkers for MinGW
RUN mkdir -p /root/.cargo && cat << 'EOF' > /root/.cargo/config.toml
[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"
ar = "x86_64-w64-mingw32-ar"
EOF

WORKDIR /workspace

CMD ["/bin/bash"]