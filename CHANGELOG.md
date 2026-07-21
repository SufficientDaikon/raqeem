# Changelog

All notable changes to this project are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-07-21

First release. A lightweight client for
[`CohereLabs/cohere-transcribe-arabic-07-2026`](https://huggingface.co/CohereLabs/cohere-transcribe-arabic-07-2026)
that delegates all inference to an endpoint and never loads model weights.

### Added

- `tafrigh-core` — `Transcriber` (multipart POST → parse → normalize), a single `Endpoint`
  adapter covering both Cohere's hosted API and any self-hosted OpenAI-compatible server
  (they differ only in URL, auth, and model id), and `normalize_ar`, a faithful Rust port
  of the reference Arabic folding (alef/hamza, taa-marbuta, tatweel + diacritics,
  Arabic-Indic & Persian digits → ASCII, U+066B → `.`).
- `tafrigh` CLI — the universal calling surface; any language can drive it by shelling out
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

[0.1.0]: https://github.com/SufficientDaikon/tafrigh/releases/tag/v0.1.0
