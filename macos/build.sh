#!/bin/bash
set -e

# 切换到脚本所在目录
cd "$(dirname "$0")"

echo "1. 编译 Rust 核心动态库 (libbarrier_core_ffi.dylib)..."
cd ../app-core-ffi
cargo build
cd ../macos

echo "2. 创建 macOS App Bundle 结构..."
APP_DIR="Barrier.app"
CONTENTS_DIR="$APP_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"
FRAMEWORKS_DIR="$CONTENTS_DIR/Frameworks"

rm -rf "$APP_DIR"
mkdir -p "$MACOS_DIR"
mkdir -p "$RESOURCES_DIR"
mkdir -p "$FRAMEWORKS_DIR"

echo "3. 编译 Swift UI 代码..."
# 使用 swiftc 编译，指定头文件和链接库
swiftc -parse-as-library swift-ui-example/BarrierMenuApp.swift \
    -import-objc-header ../app-core-ffi/include/barrier_core_ffi.h \
    -L ../target/debug \
    -lbarrier_core_ffi \
    -o "$MACOS_DIR/Barrier"

echo "4. 拷贝动态库并修复 rpath..."
cp ../target/debug/libbarrier_core_ffi.dylib "$FRAMEWORKS_DIR/"

# 让可执行文件能在 Frameworks 目录下找到 dylib
install_name_tool -add_rpath "@executable_path/../Frameworks" "$MACOS_DIR/Barrier"

# 修改可执行文件中的 dylib 路径为相对路径
install_name_tool -change "/Users/chefeilun/code/barrier-rust/target/debug/deps/libbarrier_core_ffi.dylib" "@executable_path/../Frameworks/libbarrier_core_ffi.dylib" "$MACOS_DIR/Barrier"
install_name_tool -change "/Users/chefeilun/code/barrier-rust/target/release/deps/libbarrier_core_ffi.dylib" "@executable_path/../Frameworks/libbarrier_core_ffi.dylib" "$MACOS_DIR/Barrier"
install_name_tool -change "/Users/chefeilun/code/barrier-rust/target/debug/libbarrier_core_ffi.dylib" "@executable_path/../Frameworks/libbarrier_core_ffi.dylib" "$MACOS_DIR/Barrier"
install_name_tool -change "/Users/chefeilun/code/barrier-rust/target/release/libbarrier_core_ffi.dylib" "@executable_path/../Frameworks/libbarrier_core_ffi.dylib" "$MACOS_DIR/Barrier"

echo "5. 生成 Info.plist..."
cat > "$CONTENTS_DIR/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>Barrier</string>
    <key>CFBundleIdentifier</key>
    <string>org.barrier.mac</string>
    <key>CFBundleName</key>
    <string>Barrier</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleVersion</key>
    <string>1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSPrincipalClass</key>
    <string>NSApplication</string>
</dict>
</plist>
EOF

echo "APPL????" > "$CONTENTS_DIR/PkgInfo"

echo "6. 签名..."
codesign --force --deep --sign - "$APP_DIR"

echo "✅ 构建完成！你可以双击 macos/Barrier.app 或运行 'open macos/Barrier.app' 启动原生应用。"
