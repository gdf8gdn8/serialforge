# Copyright 1999-2026 Gentoo Authors
# Distributed under the terms of the MIT License

EAPI=8

CRATES="
"

inherit cargo desktop

DESCRIPTION="Platform-independent serial terminal written in Rust with egui and Rhai scripting"
HOMEPAGE="https://github.com/gdf8gdn8/serialforge"
SRC_URI="https://github.com/gdf8gdn8/serialforge/archive/v${PV}.tar.gz -> ${P}.tar.gz
	${CARGO_CRATE_URIS}"

LICENSE="MIT"
SLOT="0"
KEYWORDS="~amd64"

RDEPEND="
	virtual/libudev
	x11-libs/libX11
	x11-libs/libXcursor
	x11-libs/libXrandr
	x11-libs/libXi
	dev-libs/libxkbcommon
	media-libs/mesa
"
DEPEND="${RDEPEND}"
BDEPEND="
	virtual/pkgconfig
"

src_install() {
	cargo_src_install
	domenu scripts/serialforge.desktop
}