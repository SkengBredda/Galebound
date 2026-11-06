#!/bin/sh
# Build the Android .apk (phones / tablets, Android 5.0+).
#
# Needs Docker (on Zorin / Ubuntu:  sudo apt install docker.io  then
# sudo usermod -aG docker $USER  and log out and back in).
# The first build downloads the Android build tools (a few GB) and takes a
# while; later builds are much quicker.
#
# Result: target/android-artifacts/release/apk/galebound.apk
# Copy it to your phone and open it (allow "install unknown apps").
set -e
cd "$(dirname "$0")"
IMAGE=galebound-android-builder
if ! docker image inspect $IMAGE >/dev/null 2>&1; then
    echo "Preparing the Android build tools (first time only)..."
    docker build -t $IMAGE -f android/Dockerfile android
fi
mkdir -p "$HOME/.cache/galebound-cargo"
docker run --rm \
    -v "$(pwd)":/root/src \
    -v "$HOME/.cache/galebound-cargo":/root/.cargo/registry \
    -w /root/src \
    $IMAGE \
    cargo quad-apk build --release
APK=$(ls target/android-artifacts/release/apk/*.apk 2>/dev/null | head -1)
if [ -n "$APK" ]; then
    cp "$APK" Galebound.apk
    echo
    echo "Done: $(pwd)/Galebound.apk"
else
    echo "The build finished but no .apk was found - see the messages above."
    exit 1
fi
