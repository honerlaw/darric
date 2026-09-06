# Replan: release-on-merge

## 2026-09-05 — the release build cannot use the build machine's own CPU, and never declared a minimum macOS

### Original plan

One native runner per architecture — `macos-15`/aarch64 and `macos-15-intel`/x64 — running a plain
`npm run tauri:build` with no build-configuration overrides. Native runners were chosen precisely
to avoid cross-compiling whisper.cpp's C/C++ half, and the proposal treated "native runner" as the
end of the build-configuration question.

### What changed

The first real CI run failed on **both** legs, for two unrelated reasons. That the two differ is
the point: "build it natively on the right hardware" turned out not to be sufficient on either
architecture, in two different ways.

**aarch64 (`macos-15-arm64`)** — ggml defaults to `GGML_NATIVE=ON` and compiles with
`-mcpu=native`:

```
-- ARM -mcpu not found, -mcpu=native will be used
-- Performing Test GGML_MACHINE_SUPPORTS_i8mm - Failed
ggml-cpu-quants.c:1818: error: always_inline function 'vmmlaq_s32' requires target feature
'i8mm', but would be inlined into function 'ggml_vec_dot_q4_0_q8_0' that is compiled without
support for 'i8mm'
```

`-mcpu=native` expands to a CPU whose headers define `__ARM_FEATURE_MATMUL_INT8`, while ggml's own
`GGML_MACHINE_SUPPORTS_i8mm` probe fails — so the intrinsic is reached in a translation unit
compiled without `+i8mm`. `check.yml` never hit this because it runs on `macos-latest`, which now
resolves to `macos-26-arm64`, not `macos-15`.

The build break is the visible half. The larger problem is that `-mcpu=native` is simply wrong for
an artifact handed to other people: it targets **the machine that built it**. A binary built on an
M2+ runner can emit i8mm instructions that `SIGILL` on an M1 Mac, and an x86 leg can emit AVX-512
that faults on older Intel Macs. Left alone, the aarch64 leg would have gone green the moment it
moved to a newer runner image — and started shipping a binary that crashes on part of the installed
base, with nothing in CI able to see it.

**x64 (`macos-15`, a genuine Intel image)** — a different failure entirely:

```
ggml-backend-reg.cpp:505: error: 'exists' is unavailable: introduced in macOS 10.15
ggml-backend-reg.cpp:508: error: 'directory_iterator' is unavailable: introduced in macOS 10.15
```

`std::filesystem` needs a macOS 10.15 deployment target. darric declared **no**
`minimumSystemVersion` anywhere, so Tauri applied its 10.13 default. arm64 hid this — its floor is
11.0 — and only the Intel leg was ever going to surface it.

That default was not merely too low for ggml. darric records through
`AudioHardwareCreateProcessTap`, a **macOS 14.4+** API it calls unconditionally, so the bundle has
been advertising support for a decade of macOS releases on which its central feature does not
exist. On those systems the app installs and fails at launch. The build error was the first thing
to make an already-wrong declaration visible.

### New plan

Keep both pinned native runners. Two additions, one per failure.

**1. A portable CPU baseline.** `src-tauri/cmake/portable-cpu.cmake` sets

```cmake
set(GGML_NATIVE OFF CACHE BOOL "Target a portable CPU baseline, not the build machine" FORCE)
```

and the build step passes `CMAKE_TOOLCHAIN_FILE` pointing at it.

A toolchain file rather than a `GGML_NATIVE=OFF` environment variable, because whisper-rs-sys
0.13.1's `build.rs` forwards into cmake only those variables whose names begin with `WHISPER_` or
`CMAKE_` (its "Allow passing any WHISPER or CMAKE compile flags" loop). `GGML_NATIVE` does not match
that allowlist and cannot be passed directly; `CMAKE_TOOLCHAIN_FILE` does, and a toolchain file is
read before ggml's `option()` call, so a `FORCE`d cache entry set there wins. The file deliberately
does not set `CMAKE_SYSTEM_NAME`, which would flip CMake into cross-compiling mode.

Applied to **both** legs, deliberately — a baseline x86-64 build is desirable for exactly the same
distribution reason.

The cost is some CPU-kernel tuning, which is close to free here: whisper inference runs on the
Metal GPU ([[2026-09-05-reference-whisper-inference-serialises-on-one-metal-gpu]]), so these CPU
quant kernels are not the hot path.

Verified locally rather than assumed. After a clean rebuild with the toolchain file set, the
generated `CMakeCache.txt` reads:

```
CMAKE_TOOLCHAIN_FILE:FILEPATH=.../src-tauri/cmake/portable-cpu.cmake
GGML_NATIVE:BOOL=OFF
GGML_METAL:BOOL=ON
```

— the passthrough works, the `FORCE`d set beat ggml's `option(... ON)`, and the Metal backend is
undisturbed. The `-mcpu=native` and `GGML_MACHINE_SUPPORTS_*` probe lines are gone from the build
output entirely, which is what removes the contradiction.

**2. A declared minimum macOS.** `bundle.macOS.minimumSystemVersion` is now `"14.4"` in
`tauri.conf.json` — the version darric has always actually required. This raises the deployment
target past `std::filesystem`'s 10.15 floor, fixing the Intel compile, and makes the bundle
metadata honest. Documented in the README's prerequisites and downloads.

### Success criteria

Unchanged, plus:

11. The aarch64 build does not use `-mcpu=native`: `GGML_NATIVE` is `OFF` in the generated CMake
    cache, and `GGML_METAL` remains `ON`.
12. `tauri.conf.json` declares `minimumSystemVersion: "14.4"`, and the README states it.
