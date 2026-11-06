# Android build tools for Galebound: macroquad's official cargo-quad-apk
# image (Android SDK + NDK + cargo-quad-apk), with an up-to-date Rust.
FROM notfl3/cargo-apk
RUN rustup update stable && rustup default stable \
 && rustup target add aarch64-linux-android armv7-linux-androideabi
