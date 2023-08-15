#!/bin/bash

# Build helper for rusty-spout
# usage: build.sh [COMMAND]

set -e

BAD_ARG_MSG="Possible options are: debug, release, package, clean"
# Intermediate file needed since cargo locks all build files during builds
# so we have to copy the dll separately
DLL_PATH_FILE="_spout_dll_path"
PACKAGE_DIR="rusty-spout"
SPOUT_FILE_NAME="SpoutLibrary.dll"
LIB_FILE_NAME="rusty_spout.dll"

if [[ "$#" -ne 1 ]]; then
    echo "$BAD_ARG_MSG"
    exit 1
fi

# Ensure we are in the correct directory
cd "$(dirname "$0")"

opt="$1"
target_dir="./target/$opt"
shift

echo "Using option $opt"

case "$opt" in
    debug)
        echo "Running: cargo build"
        cargo build
        ;;
    release)
        echo "Running: cargo build --release"
        cargo build --release
        ;;
    package)
        # Redefine since ./target/package is not a valid directory
        target_dir="./target/release"
        echo "Running: cargo build --release"
        cargo build --release
        ;;
    clean)
        echo "Cleaning!"

        echo "Running: cargo clean"
        cargo clean

        if [[ -e "$DLL_PATH_FILE" ]]; then
            echo "Removing $DLL_PATH_FILE"
            rm "$DLL_PATH_FILE"
        fi

        if [[ -d "$PACKAGE_DIR" ]]; then
            echo "Removing $PACKAGE_DIR"
            rm -rf "$PACKAGE_DIR"
        fi

        echo "Clean complete!"

        exit 0
        ;;
    *)
        echo "$BAD_ARG_MSG"
        exit 1
        ;;
esac

echo "Copying $SPOUT_FILE_NAME to $target_dir"
cp "$(head -n 1 "$DLL_PATH_FILE")" "$target_dir"

if [[ "$opt" == "package" ]]; then
    echo "Packaging into $PACKAGE_DIR"

    if [[ -d "$PACKAGE_DIR" ]]; then
        echo "Removing existing $PACKAGE_DIR directory"
        rm -rf "$PACKAGE_DIR"
    fi

    echo "Creating $PACKAGE_DIR directory"
    mkdir "$PACKAGE_DIR"

    echo "Copying $SPOUT_FILE_NAME"
    cp "$target_dir/$SPOUT_FILE_NAME" "$PACKAGE_DIR"
    echo "Copying $LIB_FILE_NAME"
    cp "$target_dir/$LIB_FILE_NAME" "$PACKAGE_DIR"

    echo "Packaging complete"
fi
