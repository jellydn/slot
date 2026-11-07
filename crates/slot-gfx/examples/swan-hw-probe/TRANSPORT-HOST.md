# Surfaceless texture-transport protocol proof

This is an **opt-in Linux host correctness diagnostic**, separate from Cargo's
normal builds. It does not load Slot or SwanStation. It does not implement
Slot's production transport, change the compositor or enable a renderer.
No game, BIOS, card, state or configuration files are opened. There is no native
window, swap, audio-device output, device access or package generation.

The owner approved this narrowed milestone after the surfaceless host exposed
no window/pbuffer EGL configuration. That limitation is not a stock TSP failure.
Do not infer window compatibility, strict actual ES2/ES3 mixing, host hardware
GPU cost, target speed or the original p95 acceptance thresholds from this test.

## Run

Requirements: Linux, C11 compiler, pthreads, EGL/GLES development headers and
libraries, Mesa with surfaceless support, `EGL_KHR_fence_sync` and test-only
`EGL_KHR_reusable_sync`. The latter supplies deterministic pending-sync controls;
it is not a requirement proposed for Slot's production transport.

From the repository root on a suitable Linux host:

```sh
EGL_PLATFORM=surfaceless LIBGL_ALWAYS_SOFTWARE=1 MESA_SHADER_CACHE_DISABLE=true \
  sh crates/slot-gfx/examples/swan-hw-probe/transport-host.sh
```

The script compiles with C11, O2 and warnings as errors, runs the quick and full
positive cases, then eight negative controls. It requires each control to exit 2
with its documented stop reason; a compiler error, unrelated assertion, signal,
timeout or unexpected success cannot pass. Temporary binaries/logs are private
to a new temporary directory and removed by the script's exit trap. Each process
has a 180-second alarm. Use only a host environment; this is not a device probe.

The local macOS investigation uses an existing private ARM64 Debian12 image
`slot-host-egl:20261107`, ID
`sha256:a852823c20fb01ad1a3916b97c7a3d7b74f03c80cd917d1b7c3566dfb43a66fb`.
It contains GCC12.2/Mesa22.3.6/GLVND1.6 and uses llvmpipe inside OrbStack. The image
is a local cache, not a published dependency or a stock firmware image.

```sh
docker run --rm --pull=never --network none --read-only --tmpfs /tmp:exec \
  --user 0 --env EGL_PLATFORM=surfaceless --env LIBGL_ALWAYS_SOFTWARE=1 \
  --env MESA_SHADER_CACHE_DISABLE=true \
  --mount "type=bind,source=$PWD/crates/slot-gfx/examples/swan-hw-probe,target=/proof,readonly" \
  --entrypoint /bin/sh slot-host-egl:20261107 /proof/transport-host.sh
```

Run through the repository's tmux session. No host desktop socket or device is
mounted. Dependencies are not installed by the test script.

## Rust adaptation (separate host-only gate)

`transport-host.rs` is a standard-library-only Rust adaptation, not an FFI
wrapper around the C proof. Cargo registers it as `transport-host-rust` behind
the existing `host-hw-probe` feature; normal builds do not include it. The
standalone runner avoids building or loading Slot, Swan or any core dependency:

```sh
sh crates/slot-gfx/examples/swan-hw-probe/transport-rust-host.sh
```

On the existing Mac, run this in the private derived ARM64 host image
`slot-host-rust-egl:20261107`, ID
`sha256:5591b4e77b2322fc639205578e7bbd9eed44a534638ed98955a7804d044804c6`:

```sh
docker run --rm --pull=never --network none --read-only --cap-drop ALL \
  --security-opt no-new-privileges --user 65534:65534 \
  --tmpfs /tmp:rw,exec,nosuid,nodev,size=256m,mode=1777 \
  --mount "type=bind,source=$PWD/crates/slot-gfx/examples/swan-hw-probe,target=/proof,readonly" \
  --entrypoint /bin/sh slot-host-rust-egl:20261107 /proof/transport-rust-host.sh
```

This private image adds Debian rustc1.63/LLVM14 to the original Mesa host. It is
not a firmware image or a published dependency. Standalone compilation with
edition2021, warnings as errors and O2 does not lower the workspace's Rust1.96
requirement. The script forces surfaceless llvmpipe and disables the shader
cache; compilation and logs stay in an automatically removed temporary root.
Direct Cargo invocation on a suitable Linux host still needs the workspace's
toolchain and normal dependencies:

```sh
EGL_PLATFORM=surfaceless LIBGL_ALWAYS_SOFTWARE=1 \
  cargo run -p slot-gfx --no-default-features --features host-hw-probe \
  --example transport-host-rust -- --quick
```

The Rust diagnostic follows the C image/fence contract below, with these explicit
ownership boundaries:

- One `Arc<Display>` owns EGL initialisation/termination and outlives both
  contexts, all fences and worker join. Static linkage keeps EGL loaded for the
  process. Each thread creates and owns its pbuffer/context and immutable local
  GL entry-point table. `Context` and `Target` are deliberately neither Send nor
  Sync; no unsafe blanket implementations or global `gl::load_with` are used.
- One stable target and exactly three publication targets cost the same fixed
  8MiB colour storage. Publication uses a GPU blit. The pool and completion table
  are fixed-size. One-entry synchronous command/reply channels allow at most one
  synthetic command in flight; they are test scheduling, not Slot pacing.
- A lease is returned only with an explicit flushed consumer fence. Dropping an
  unfinished session invalidates the generation, stops and joins its worker,
  skips GL deletion on the logically lost path and destroys the share group.
  It does not silently release the slot, restart a core or select Software.
- Producer errors invalidate before target destructors run, so a live consumer
  lease never becomes permission to GL-delete the worker's textures. Original
  worker error reasons survive channel disconnection and cannot masquerade as
  successful negative controls. Fences have single Rust owners and drop outside
  the CPU lock. Cleanup failures override the expected fault reason.
- Three partial-startup failures, a caught authored startup panic, early return
  and a caught authored main-thread panic with a live worker/held lease/two Ready
  slots check Rust cleanup. Expected panic-hook lines are printed; only the exact
  authored panic payload and zero remaining owned EGL handles pass. These do not
  test panic inside a driver or a real lost context.
- The same eight fault modes must reject at their intended boundaries, plus a
  ninth `rewind` control deliberately admits an old late completion. The retained
  test rejects that wrong implementation with `late-frame-must-not-rewind`.

Full positive mode retains the C scope:10000 initial numbered acquisitions,
100 reopens, whole-frame asymmetric RGBA checks, both origins, geometry changes,
congestion, delayed producer/consumer completion and10 logical-loss simulations.
No Slot menus, screenshots, thumbnails, real core callbacks, input, audio, discs,
cards, real driver recovery or performance are exercised. Rust handle counters
and teardown checks are not proof of driver-memory stability or hardware cost.

## Contract exercised

- Two pthreads keep separate contexts current on two 64×64 pbuffers in one share
  group. Consumer requests ES2, producer ES3; report actual versions. On the
  verified Mesa host both are GLES3.2, not strict actual ES2/ES3 mixing.
- Producer owns one stable 1024×512 RGBA8 FBO and three publication textures/FBOs.
  Colour storage is fixed at 8 MiB, excluding driver/context/test readback memory.
  Each publication GPU-blits the active rectangle; this is not zero-copy.
- Slot ownership is Free → Writing → Ready → Reading → Retiring → Free.
  Producer flushes its real EGL GPU fence before publishing. Consumer polls with
  zero timeout, rebinds the completed texture, and retains its lease until last
  use. Its flushed return fence gates reuse. GL/EGL work is outside the CPU lock.
- A full pool still renders the synthetic producer step but skips publication.
  Holding one lease permits two other publications, then 32 intentional drops.
  Concurrent reads must retain the held image. Newest completed acquisition must
  never go backwards when an older sync completes late.
- Numbered asymmetric images exercise whole-frame RGBA pixels, both origins,
  7×5/320×224/320×448/3×9 sizes, an active 1024×512 frame, geometry transitions,
  out-of-bounds rejection and preservation of non-default pack alignment.
  Capture performs readback for assertions, not for a performance measurement.
- Deterministic pending checks replace an **already completed** GPU fence with
  an unsignalled reusable EGL sync. They test producer-pending retention,
  consumer-pending non-reuse and late completion without relying on GPU timing.
  Real GPU fence completion is still used for all ordinary publications/returns.
- Full mode has 10,000 numbered acquisitions, then 100 context/share-group reopens.
  Every session closes with a held lease and two Ready publications. Normal close
  releases UI ownership before worker join and drains fences with bounded test
  ticks. Fence creation/destruction totals must balance in each positive session.
- Ten sessions simulate logical context loss: invalidate the generation, stop,
  skip GL cleanup on the invalidated path, destroy the EGL share group and start
  only a new explicitly driven test session. This is **not real driver loss**,
  core/card recovery, automatic Software relaunch or a runtime recovery claim.
- Eight negative modes reject missing producer/consumer fences, stale generation,
  wrong origin, never-signalled sync, oversized output, publication after loss
  and capture after loss. These fail closed in isolated test processes. Their
  deliberate failure uses process exit, not a tested production failure UI.

The harness uses a one-command condition-variable handshake to control scheduling;
it is not Slot's pacing, queue, audio or worker loop. Pending checks use bounded
poll ticks with 1 ms test sleeps, not a certified real-time 250 ms watchdog. Balanced
resource counters and successful EGL teardown do not prove driver memory usage.

## Next review gate

This proof must not become a production switch by changing an option. The Rust
adaptation is an isolated synthetic diagnostic only. Stop for review before an
authored Swan shared-context lifecycle gate or attaching a real core.
Slot menu/still/capture/thumbnail paths, actual
core callbacks/input/audio/disc/card persistence and real context-loss behaviour
remain unverified for this transport. Window-compositor compatibility, true
mixed-version support, hardware cost/backlog/p95 thresholds and any stock-device
operation remain separate gates requiring their own evidence and approval.

PCSX stays default. Swan Software, isolated cards and disabled states/quick
states/resume/rewind remain unchanged. No SD, release or Git push follows from
this diagnostic passing.
