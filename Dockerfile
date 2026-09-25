FROM ubuntu:26.04

ENV DEBIAN_FRONTEND=noninteractive \
    PATH="/root/.cargo/bin:${PATH}"

# Prevent installation of documentation, man pages, and extra locales
RUN echo 'path-exclude=/usr/share/doc/*' > /etc/dpkg/dpkg.cfg.d/01_nodoc \
    && echo 'path-exclude=/usr/share/man/*' >> /etc/dpkg/dpkg.cfg.d/01_nodoc \
    && echo 'path-exclude=/usr/share/groff/*' >> /etc/dpkg/dpkg.cfg.d/01_nodoc \
    && echo 'path-exclude=/usr/share/info/*' >> /etc/dpkg/dpkg.cfg.d/01_nodoc \
    && echo 'path-exclude=/usr/share/lintian/*' >> /etc/dpkg/dpkg.cfg.d/01_nodoc \
    && echo 'path-exclude=/usr/share/linda/*' >> /etc/dpkg/dpkg.cfg.d/01_nodoc

# Configure APT to drop documentation suggestions
RUN echo 'APT::Install-Recommends "0";' > /etc/apt/apt.conf.d/01no-recommends \
    && echo 'APT::Install-Suggests "0";' >> /etc/apt/apt.conf.d/01no-recommends

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        curl \
        file \
        gcc-mingw-w64-x86-64 \
        g++-mingw-w64-x86-64 \
        git \
        libfuse2t64 \
        libgl1-mesa-dev \
        libudev-dev \
        libx11-dev \
        libxcursor-dev \
        libxi-dev \
        libxinerama-dev \
        libxkbcommon-dev \
        libxrandr-dev \
        mingw-w64-tools \
        pkg-config \
        squashfs-tools \
    && rm -rf /var/lib/apt/lists/*

RUN curl -fsSL \
        https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage \
        -o /tmp/appimagetool.AppImage \
    && chmod +x /tmp/appimagetool.AppImage \
    && cd /tmp \
    && ./appimagetool.AppImage --appimage-extract \
    && mv squashfs-root /usr/local/bin/appimagetool-extracted \
    && ln -s /usr/local/bin/appimagetool-extracted/AppRun /usr/local/bin/appimagetool \
    && rm /tmp/appimagetool.AppImage

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --default-toolchain stable --profile minimal \
    && rustup target add x86_64-pc-windows-gnu \
    && cargo install cargo-deb --locked

WORKDIR /workspace

CMD ["/bin/bash"]