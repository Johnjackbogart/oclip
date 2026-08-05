# oclip

An open-source clipper plugin for Ableton Live and other DAWs, in the spirit of GClip. Three
controls, nothing else: **Gain**, **Clip Amount**, and **Softness**.

- **Gain** — input drive, in dB.
- **Clip Amount** — the threshold/ceiling clipping engages at, in dB. Lower = harder clipping.
- **Softness** — blends between a hard brick-wall clip (0%) and a smooth `tanh`-based saturation
  (100%) at the same threshold.

Built in Rust on [`nice-plug`](https://codeberg.org/RustAudio/nice-plug) for VST3 and CLAP.

## Build

```sh
cargo build
cargo test
cargo xtask bundle oclip --release
```

This produces `target/bundled/oclip.vst3` and `target/bundled/oclip.clap`. To load it in a DAW,
copy the `.vst3` bundle into your local VST3 plugin directory (on macOS:
`~/Library/Audio/Plug-Ins/VST3/`) and rescan plugins.

## Status

Early — v1 DSP and a minimal custom GUI. AU (Logic/GarageBand) support and oversampling are not
implemented yet; see `CLAUDE.md`/`AGENTS.md` for the full rationale on what's deferred and why.

## License

MIT — see `LICENSE`.
