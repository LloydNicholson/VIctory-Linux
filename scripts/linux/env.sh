# source scripts/linux/env.sh: cross-build environment for building the Windows victory.exe on Linux.
# Needs: a Rust toolchain with the x86_64-pc-windows-msvc target, clang-cl, lld-link, llvm-lib, and the MSVC CRT and
# Windows SDK unpacked by `xwin` (https://github.com/Jake-Shadle/xwin) into XWIN_DIR.
# Do not run crates/victory-hle/unix/build.sh in a shell with this sourced: victory_pages.so comes out broken.
X=${XWIN_DIR:-$HOME/.local/share/xwin-sdk}
SDK=${XWIN_SDK_VERSION:-10.0.26100}; MSVC=${XWIN_MSVC_VERSION:-14.44.17.14}
K="$X/Windows Kits/10/Lib/$SDK"; V="$X/VC/Tools/MSVC/$MSVC"; I="$X/Windows Kits/10/Include/$SDK"
[ -n "$VICTORY_RUST_HOME" ] && export RUSTUP_HOME=$VICTORY_RUST_HOME/rustup CARGO_HOME=$VICTORY_RUST_HOME/cargo PATH=$VICTORY_RUST_HOME/cargo/bin:$PATH
export CARGO_ENCODED_RUSTFLAGS="-Lnative=$V/lib/x64"$'\x1f'"-Lnative=$K/um/x64"$'\x1f'"-Lnative=$K/ucrt/x64"
export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=lld-link CC_x86_64_pc_windows_msvc=clang-cl CXX_x86_64_pc_windows_msvc=clang-cl AR_x86_64_pc_windows_msvc=llvm-lib
export INCLUDE="$V/include;$I/ucrt;$I/um;$I/shared" CARGO_BUILD_TARGET=x86_64-pc-windows-msvc
