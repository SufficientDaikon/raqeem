# Contributing to tafrigh

This repo is built to be extended by **AI agents** as much as by humans. If you're an AI
agent asked to "add X to tafrigh," start here, then use the matching skill in
[`.claude/skills/`](.claude/skills/).

## The invariants (do not break these)

1. **Arabic-first.** Arabic is the primary language everywhere — labels, docs, defaults
   (`--lang ar`, `cohere-transcribe-arabic`). Every transcript carries a `text_normalized`
   folded through `normalize_ar` (`crates/tafrigh-core/src/arabic.rs`). Never fuzzy-match or
   parse raw Arabic without normalizing. **Do not edit `arabic.rs` without a matching test** —
   it is a faithful port of scout's verified `normalize_ar`.
2. **Inference is delegated. Always.** The core loads no model weights and pulls no ML
   framework. It POSTs audio to an endpoint and reads back text. Anything that would drag
   `torch`/`onnx`/ffmpeg into `tafrigh-core`'s dependencies is wrong — that belongs in an
   **optional** module or a separate endpoint.
3. **The core stays light.** No dependency joins `tafrigh-core` unless it earns its keep for
   the core transcribe path. Optional capabilities (diarization, subtitles) live behind their
   own crates/features, opt-in.
4. **Tests run offline.** No test may hit the network or need an API key. Mock the HTTP
   endpoint (`mockito`, see `crates/tafrigh-core/tests/endpoint_mock.rs`). `cargo test` must
   stay green with no secrets.

## Where the extension points are

| You want to… | Touch | Skill |
|---|---|---|
| Add a backend (Deepgram, Groq, another vLLM shape) | `provider.rs` (variant) + `endpoint.rs` (constructor) + a mocked test | [`add-endpoint-adapter`](.claude/skills/add-endpoint-adapter/) |
| Add an output format (SRT, VTT) | `output.rs` (`OutputFormat` variant + `render`) + CLI `--format` | *(roadmap skill)* |
| Add a language binding (Python, Node, WASM) | new workspace crate wrapping `tafrigh-core` | *(roadmap skill)* |

## Architecture (one-minute tour)

```
crates/tafrigh-core/   the library — all logic
  arabic.rs      normalize_ar (Arabic folding; ported from scout, keep in parity)
  provider.rs    Provider enum — backend presets/labels
  endpoint.rs    Endpoint — the one adapter: url + auth + model
  lib.rs         Transcriber::transcribe — read file → multipart POST → parse → normalize
  output.rs      Transcript → text / json
crates/tafrigh-cli/    the `tafrigh` binary — clap args → core (the universal calling surface)
```

Every backend speaks the same OpenAI-compatible multipart `/audio/transcriptions` shape
(`file` + `model` + `language`), so a new backend is almost always just a new `Endpoint`
constructor — not a new request path.

## Checklist before you're done

- [ ] `cargo test` green (offline).
- [ ] `cargo build --release` succeeds.
- [ ] New behavior has at least one test (unit for pure logic, mocked-endpoint for I/O).
- [ ] Arabic-first invariant intact; `text_normalized` still produced.
- [ ] No new heavy dependency in `tafrigh-core`.
