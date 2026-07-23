#!/bin/bash
# iOSシミュレータ用の .app バンドルを組み立ててインストール・起動するスクリプト
# 使い方: ./ios/build-sim.sh
set -euo pipefail

cd "$(dirname "$0")/.."

TARGET=aarch64-apple-ios-sim
APP_NAME=BebyRogueLike
BUNDLE_ID=dev.na2kera.beby-rogue-like
APP_DIR="target/ios-sim/$APP_NAME.app"
SIM_DEVICE="${SIM_DEVICE:-iPhone 16}"

# 1. ビルド（dynamic_linking はiOSで使えないので --no-default-features）
cargo build --target "$TARGET" --no-default-features

# 2. .app バンドルを組み立てる
rm -rf "$APP_DIR"
mkdir -p "$APP_DIR"
cp "target/$TARGET/debug/beby-rogue-like" "$APP_DIR/"
cp ios/Info.plist "$APP_DIR/"
cp -R assets "$APP_DIR/assets"

# 3. シミュレータを起動してインストール・実行
xcrun simctl bootstatus "$SIM_DEVICE" -b
open -a Simulator
xcrun simctl install "$SIM_DEVICE" "$APP_DIR"
xcrun simctl launch "$SIM_DEVICE" "$BUNDLE_ID"
