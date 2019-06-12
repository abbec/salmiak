#!/usr/bin/env sh

# Change these when we want other versions
BINUTILS_EXPECTED_VERSION="0.1.6"
XBUILD_EXPECTED_VERSION="0.5.11"

PACKAGES=""
VERSIONS=$(cargo install --list)

# check for binutils
BINUTILS_VERSION=$(echo "$VERSIONS" | awk -F'[v :]' '/cargo-binutils v/{print $3}')
if [ ! "$BINUTILS_VERSION" = "$BINUTILS_EXPECTED_VERSION" ]; then
	PACKAGES="$PACKAGES cargo-binutils"
	echo "\n♻️  reinstalling binutils since we want $BINUTILS_EXPECTED_VERSION, but $BINUTILS_VERSION \
is installed"
else
	echo "\n✅ skipping binutils since we want $BINUTILS_EXPECTED_VERSION, and $BINUTILS_VERSION \
is installed"
fi;

# check for xbuild
XBUILD_VERSION=$(echo "$VERSIONS" | awk -F'[v :]' '/cargo-xbuild v/{print $3}')
if [ ! "$XBUILD_VERSION" = "$XBUILD_EXPECTED_VERSION" ]; then
	PACKAGES="$PACKAGES cargo-xbuild"
	echo "\n♻️  reinstalling xbuild since we want $XBUILD_EXPECTED_VERSION, but $XBUILD_VERSION \
is installed"
else
	echo "\n✅ skipping xbuild since we want $XBUILD_EXPECTED_VERSION, and $XBUILD_VERSION \
is installed"
fi;

echo ""

if [ ! -z "$PACKAGES" ]; then
	echo "executing 'cargo install --force$PACKAGES'"
	cargo install --force$PACKAGES
fi;
