# CPU baseline for distributable builds of whisper.cpp's ggml.
#
# ggml defaults to GGML_NATIVE=ON, which compiles with `-mcpu=native`: the
# binary targets the CPU of whatever machine built it. That is right for a
# local build and wrong for a release artifact, in two separate ways.
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
#     runner emits i8mm instructions that fault with SIGILL on an M1 Mac, and
#     an x86 runner can emit AVX-512 that faults on older Intel Macs. That
#     failure reaches users at runtime, not us at build time.
#
# GGML_NATIVE=OFF drops back to the toolchain's default target for the
# architecture -- the baseline every Apple Silicon (or every x86-64) Mac
# supports. It costs some CPU kernel tuning, which is close to free here:
# darric runs whisper inference on the Metal GPU, so these CPU quant kernels
# are not the hot path.
#
# This is delivered as a toolchain file rather than an environment variable
# because whisper-rs-sys's build.rs forwards only variables whose names start
# with WHISPER_ or CMAKE_ (see its "Allow passing any WHISPER or CMAKE compile
# flags" loop). GGML_NATIVE does not match, so it cannot be passed directly;
# CMAKE_TOOLCHAIN_FILE does, and a toolchain file is read before ggml's
# option() calls, so a FORCEd cache entry here wins.
#
# Deliberately does NOT set CMAKE_SYSTEM_NAME -- doing so would flip CMake into
# cross-compiling mode. Each release leg builds natively on its own runner.

set(GGML_NATIVE OFF CACHE BOOL "Target a portable CPU baseline, not the build machine" FORCE)
