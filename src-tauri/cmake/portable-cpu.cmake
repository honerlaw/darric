# CPU baseline for whisper.cpp's ggml: target a fixed floor, not the machine
# that happens to be compiling.
#
# ggml defaults to GGML_NATIVE=ON, which compiles with `-mcpu=native`. That is
# right for a local build and wrong for a binary handed to someone else, in two
# separate ways.
#
#   * It breaks the build outright on some runners. On GitHub's macos-15 arm64
#     image, `-mcpu=native` expands to a CPU whose headers define
#     __ARM_FEATURE_MATMUL_INT8 while ggml's own GGML_MACHINE_SUPPORTS_i8mm
#     probe fails -- so ggml-cpu-quants.c reaches `vmmlaq_s32` in a translation
#     unit compiled without +i8mm, and clang rejects it:
#       error: always_inline function 'vmmlaq_s32' requires target feature
#       'i8mm', but would be inlined into function 'ggml_vec_dot_q4_0_q8_0'
#       that is compiled without support for 'i8mm'
#
#   * When it does build, it produces a binary tuned to the builder. An M2+
#     runner emits i8mm instructions that fault with SIGILL on an M1 Mac. That
#     failure reaches users at runtime, not us at build time.
#
# WHAT THIS ACTUALLY GUARANTEES, per architecture. Do not read GGML_NATIVE=OFF
# as "runs on every Mac" -- it is not symmetric, and the x86 half surprises you:
#
#   * arm64: a real baseline. ggml's non-native branch adds -march only from
#     GGML_CPU_ARM_ARCH, which is unset, so the build takes the toolchain
#     default for apple-darwin and the i8mm path is compiled out. This is the
#     half that fixes the failure above.
#
#   * x86_64: NOT a baseline. ggml computes
#       INS_ENB = NOT (GGML_NATIVE OR NOT GGML_NATIVE_DEFAULT)
#     and GGML_NATIVE_DEFAULT is ON whenever we are not cross-compiling -- which
#     is always here, since each release leg builds natively. So turning
#     GGML_NATIVE off makes INS_ENB *ON*, and GGML_AVX2/GGML_FMA/GGML_F16C
#     default to it. The x64 binary therefore requires Haswell-class (2013+)
#     hardware. Confirmed in the generated CMakeCache.txt: GGML_NATIVE:BOOL=OFF
#     alongside GGML_AVX2:BOOL=ON and GGML_FMA:BOOL=ON.
#
# That x86 floor is safe here only because tauri.conf.json sets
# minimumSystemVersion 14.4: every Intel Mac that can run Sonoma is a 2017 model
# or newer, hence Skylake+ and AVX2-capable. The two settings are load-bearing
# together. If minimumSystemVersion is ever lowered, this stops holding and
# GGML_AVX2/GGML_FMA/GGML_F16C must be forced OFF here as well.
#
# WHY A TOOLCHAIN FILE. whisper-rs-sys's build.rs forwards into cmake only those
# environment variables whose names start with WHISPER_ or CMAKE_ (its "Allow
# passing any WHISPER or CMAKE compile flags" loop). GGML_NATIVE does not match
# and cannot be passed directly. CMAKE_TOOLCHAIN_FILE does match, and a
# toolchain file is read at the first project() call -- before whisper.cpp's and
# ggml's own option(GGML_NATIVE ...) run -- so the FORCEd cache entry below
# wins, because option() never overwrites an existing cache entry.
#
# The obvious-looking shortcut does not work: whisper.cpp has a WHISPER_NATIVE
# alias, which would pass the allowlist with no toolchain file at all, but its
# shim fires only `if (WHISPER_NATIVE)` and only ever forces GGML_NATIVE *ON*.
# WHISPER_NATIVE=OFF is silently ignored. Do not "simplify" to it.
#
# Two deliberate omissions:
#   * No CMAKE_SYSTEM_NAME -- setting it would flip CMake into cross-compiling
#     mode, and each release leg builds natively on its own runner.
#   * Defining CMAKE_TOOLCHAIN_FILE suppresses the cmake crate's own
#     -DCMAKE_C_COMPILER/-DCMAKE_CXX_COMPILER pinning (it emits those only when
#     no toolchain file is set), so CMake runs its own compiler search. That is
#     fine on a stock macOS toolchain, but if CC/CXX is ever customized here,
#     pin the compilers in this file.

set(GGML_NATIVE OFF CACHE BOOL "Target a fixed CPU floor, not the build machine" FORCE)
