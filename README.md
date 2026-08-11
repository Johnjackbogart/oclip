# openclip

An open-source clipper plugin for Ableton Live and other DAWs, in the spirit of GClip. Three
controls, nothing else: **Gain**, **Clip Amount**, and **Softness**.

- **Gain** — input drive, in dB.
- **Clip Amount** — the threshold/ceiling clipping engages at, in dB. Lower = harder clipping.
- **Softness** — blends between a hard brick-wall clip (0%) and a smooth `tanh`-based saturation
  (100%) at the same threshold.

Built in Rust on [`nice-plug`](https://codeberg.org/RustAudio/nice-plug) for VST3 and CLAP.

## Download

Prebuilt VST3 + CLAP bundles for macOS (universal), Windows, and Linux:
**[johnjackbogart.github.io/oclip](https://johnjackbogart.github.io/oclip/)** — or grab them
directly from the [latest release](https://github.com/Johnjackbogart/oclip/releases/latest).

## Build

```sh
cargo build
cargo test
cargo xtask bundle openclip --release
```

This produces `target/bundled/openclip.vst3` and `target/bundled/openclip.clap`. To load it in a DAW,
copy the `.vst3` bundle into your local VST3 plugin directory (on macOS:
`~/Library/Audio/Plug-Ins/VST3/`) and rescan plugins.

## Status

Early — v1 DSP and a minimal custom GUI. AU (Logic/GarageBand) support and oversampling are not
implemented yet; see `CLAUDE.md`/`AGENTS.md` for the full rationale on what's deferred and why.

This was vibe coded. I've read the code, it makes sense. But I'm learning Rust and am not an expert. The plug in works

## License

MIT — see `LICENSE`.
