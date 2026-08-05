# oclip

An open-source clipper plugin (VST3 + CLAP) in the spirit of GClip. Rust, built on
[`nice-plug`](https://codeberg.org/RustAudio/nice-plug) (a community-led fork/successor of
nih-plug — not the same thing, don't confuse the two APIs when searching for docs/examples).

## What this is

Three parameters, no more, unless the user explicitly asks for a new one:

- **Gain** (`src/lib.rs`, param id `"gain"`) — input drive in dB, applied before the clipper.
- **Clip Amount** (id `"clip_amount"`) — threshold/ceiling in dB; lower = clips harder.
- **Softness** (id `"softness"`) — 0–100%, blends between a hard brick-wall clip and a
  `tanh`-based soft saturation at the same threshold.

The waveshaping math lives in `src/dsp/clipper.rs`, deliberately isolated from the plugin/param
glue in `src/lib.rs` so it stays unit-testable and benchmarkable without a host. If you change the
algorithm, change it there first and update/extend its tests — don't reimplement clipping math
inline in `process()`.

The GUI (`src/editor/`) is a custom egui interface: three rotary knobs bound to the params above,
via a hand-rolled `Knob` widget in `src/editor/knob.rs` (nice-plug-egui only ships slider widgets,
not knobs). **Currently disabled** — `Oclip::editor()` in `src/lib.rs` returns `None` instead of
calling `editor::create()`, so hosts fall back to the generic parameter panel. See "Known issues"
below before re-enabling it. Level metering was scoped out of v1 — see "Not in v1" below.

**Adding parameters or features is a scope decision, not an implementation detail.** If a task
seems to call for a new knob, mode, or behavior beyond the three params above, check with the user
before adding it rather than expanding scope silently.

## Priorities, in order

1. **DSP correctness.** The clipper should do exactly what its three parameters say it does, and
   the unit tests in `src/dsp/clipper.rs` should reflect real invariants (not just "whatever the
   code currently does" — see the git history for an example of a test that encoded a wrong
   assumption and had to be fixed).
2. **Performance.** No allocation or locking in `process()` or anything it calls. `Oclip::process`
   already follows the pattern to keep: pull one smoothed dB value per sample-*frame* (not per
   channel) from `param.smoothed.next()`, convert dB→linear once per frame with
   `util::db_to_gain`, then apply that single linear value across all channels in the frame. Don't
   call `db_to_gain`/`smoothed.next()` more often than once per frame per parameter, and don't
   reintroduce per-channel conversion work.
3. **GUI polish.** Comes last. The knob widget is intentionally minimal.

## Not in v1 (considered, deliberately deferred)

- **AU (Audio Unit) support.** Only benefits Logic Pro/GarageBand; nice-plug has no native AU
  export, would require a hand-written Objective-C/C++ wrapper or a third-party VST3→AU shim. Not
  needed for Ableton.
- **Oversampling.** Hard/soft clipping generates harmonics that can alias above Nyquist. v1 ships
  without oversampling for simplicity and speed; 2x oversampling (polyphase halfband filtering) is
  a plausible fast-follow if aliasing turns out to be audible in practice.
- **Output/makeup gain, dry/wet mix, level meters.** Reasonable additions, but out of scope until
  asked for.

## Known issues

**GUI crashes Ableton on macOS (currently disabled, see above).** First real-world test crashed
Ableton Live 12.4.3 on macOS 26.5.2 immediately on opening the editor: `EXC_BAD_ACCESS` /
`SIGSEGV`, "Thread stack size exceeded due to excessive recursion" — one function called itself
~104,795 times until the thread's stack overflowed. It happened on the main thread inside AppKit's
cursor-rect/hit-test routing (`-[NSApplication sendEvent:]` → `routeCursorRect` →
`-[NSView hitTest:]`), not in DSP/audio code.

Leading hypothesis (not confirmed — the crash report had no symbols for `oclip`, see below): the
custom `hitTest:` override in `baseview-0.2.2/src/platform/macos/view.rs` (used to work around
baseview's "first click dead zone" bug, #129/#202/#169) is meant to call the *real* `NSView`'s
`hitTest:` via `msg_send![super(this.view, superclass), hitTest: point]`. If that superclass
dispatch ever resolves back to itself instead of true `NSView` — plausible on a newer macOS than
this crate generation was tested against — you'd get exactly this signature: one frame recursing
with zero alternation. This is consistent with nice-plug's own "limited macOS testing" disclaimer
below. Not something to fix by editing `baseview`'s vendored-in code; if confirmed, the real fix is
tracking an upstream baseview/egui-baseview update or reporting it.

**To re-enable the GUI**: first rebuild (symbols are currently kept in release specifically for
this, see `Cargo.toml`) and reproduce the crash to get a symbolicated stack trace confirming or
ruling out the hypothesis above, before touching `editor()` in `src/lib.rs` again.

## macOS caveat

nice-plug's own documentation notes only "limited testing" on macOS (Linux/Windows are the primary
targets) — the GUI crash above is a concrete instance of that, not just a theoretical risk. VST3
bundle loading, CLAP loading, and the base DSP have not shown macOS-specific problems; the GUI
stack (baseview/egui-baseview) is the part that has.

## Build, test, bundle

```sh
cargo build              # library only, does not produce a loadable plugin bundle
cargo test                # runs src/dsp/clipper.rs unit tests
cargo xtask bundle oclip --release   # produces target/bundled/oclip.vst3 and oclip.clap
```

To manually test in a DAW, copy `target/bundled/oclip.vst3` into the local VST3 plugin directory
(`~/Library/Audio/Plug-Ins/VST3/` on macOS) and rescan plugins in the DAW.

## License

MIT. Matches nice-plug's own permissive (ISC) license — no GPL entanglement from either VST3 or
CLAP, since nice-plug doesn't link Steinberg's actual SDK source.
