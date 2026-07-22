# Changelog

All notable changes to this project are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] — 2026-07-22

The first release published to PyPI — `pip install raqeem`. No library or CLI behaviour
changed from 0.2.0; this is 0.2.0 plus a wheel pipeline that builds on every target.

### Fixed

- The **aarch64 Linux wheel** failed to build: the manylinux2014 cross-gcc doesn't define
  `__ARM_ARCH` while assembling `ring`'s pregenerated ARM assembly (`ring` arrives via
  `rustls`), so `chacha-armv8-linux64.S` hard-errored out of `asm_base.h`. Now defined
  explicitly, for that target only.
- Dropped `sccache` from the wheel build — wrapping the cross-compiler interfered with the
  same assembly step.

## [0.2.0] — 2026-07-21

### Changed — the project is renamed `tafrigh` → `raqeem` (رقيم)

`tafrigh` was already taken on PyPI by
[ieasybooks/tafrigh](https://github.com/ieasybooks/tafrigh), an established Arabic
transcription tool in the same niche — so `pip install tafrigh` was impossible and the name
competed with an incumbent. Renamed while the project had no users rather than later.

**Breaking:** the binary is now `raqeem`, the crates are `raqeem-core` / `raqeem-cli`, and
the CLI env var `TAFRIGH_API_KEY` is now `RAQEEM_API_KEY`. `COHERE_API_KEY` is unchanged.
The v0.1.0 release keeps its `tafrigh-*` assets; GitHub redirects the old repository URL.

### Added — Python bindings

- `pip install raqeem` → `import raqeem`. A PyO3 native extension over the *same* Rust core,
  so there is no second implementation to drift: `transcribe(path, lang=…, provider=…,
  endpoint=…, api_key=…, model=…, timeout=…)` returns a `Transcript` with `.text`,
  `.text_normalized`, `.to_dict()`, and `normalize_ar()` is exposed directly.
- Built `abi3-py39`, so one wheel per OS/arch serves CPython 3.9+. Wheels for Linux
  (x86_64/aarch64, manylinux2014), macOS (x86_64/arm64) and Windows, published to PyPI via
  Trusted Publishing.
- The GIL is released for the blocking round-trip, so a transcription doesn't freeze the
  caller's interpreter. Type stubs ship with the wheel.
- The Cohere-scoped `$COHERE_API_KEY` fallback applies to `provider="cohere"` only, matching
  the CLI — a self-hosted endpoint never receives it. Covered by tests in both languages.

## [0.1.0] — 2026-07-21

Released under the project's original name, `tafrigh`. A lightweight client for
[`CohereLabs/cohere-transcribe-arabic-07-2026`](https://huggingface.co/CohereLabs/cohere-transcribe-arabic-07-2026)
that delegates all inference to an endpoint and never loads model weights.

### Added

- `raqeem-core` — `Transcriber` (multipart POST → parse → normalize), a single `Endpoint`
  adapter covering both Cohere's hosted API and any self-hosted OpenAI-compatible server
  (they differ only in URL, auth, and model id), and `normalize_ar`, a faithful Rust port
  of the reference Arabic folding (alef/hamza, taa-marbuta, tatweel + diacritics,
  Arabic-Indic & Persian digits → ASCII, U+066B → `.`).
- `raqeem` CLI — the universal calling surface; any language can drive it by shelling out
  and reading JSON from stdout.
- `--timeout` (default 300s) covering upload + inference + download.
- `examples/serve_local.py` — an optional CPU/GPU torch server exposing the same endpoint
  locally, for people without a Cohere key.
- Contributor tooling aimed at AI agents: `CONTRIBUTING.md` plus a working
  `.claude/skills/add-endpoint-adapter` skill that scaffolds a new backend end to end.

### Fixed (both caught by live testing against the real API)

- Cohere rejects requests unless the `model` and `language` form fields appear **before**
  the file part in the multipart body. Field order is now asserted by a test.
- Cohere 404s undated model aliases; the default is now the dated
  `cohere-transcribe-arabic-07-2026`.

### Security

- The Cohere-scoped `$COHERE_API_KEY` fallback applies to `--provider cohere` only, so the
  key can never be sent to a self-hosted `--endpoint`. Covered by tests.

### Known limitations

- A live *Arabic* transcription is unverified: on the trial key used in development the
  Arabic model never responds (hangs instead of returning 403) while the same request
  against a dated English model returns instantly. The request bytes are identical, so this
  appears to be account/model access rather than a client defect — but it is unproven.
- No timestamps, diarization, VAD, or long-form chunking yet (see the roadmap in the README).

[0.2.1]: https://github.com/SufficientDaikon/raqeem/releases/tag/v0.2.1
[0.2.0]: https://github.com/SufficientDaikon/raqeem/releases/tag/v0.2.0
[0.1.0]: https://github.com/SufficientDaikon/raqeem/releases/tag/v0.1.0
