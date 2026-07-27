# Changelog

All notable changes to this project are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] — 2026-07-28

A hardening pass over the whole client. The headline is a normalization bug that made
`text_normalized` silently wrong for text carrying invisible characters — if you match on
that field, re-run anything you have cached.

### Fixed

- **`normalize_ar` was missing an entire pass, so invisible characters survived it.**
  Zero-width space and joiners, the bidi marks, embeddings, overrides and isolates, and
  the BOM (`U+200B–200F`, `U+202A–202E`, `U+2066–2069`, `U+FEFF`) all passed through
  untouched. None of them render, so two strings that look identical compared unequal —
  exactly the failure `text_normalized` exists to prevent, and RTL text pasted out of a
  chat app or scraped off a page carries these routinely.

  raqeem's `normalize_ar` is a port of scout's, and the port was faithful when written;
  the reference added this pass and the port didn't follow. The two test suites had
  listed their cases separately by hand, which is why neither noticed. They now read
  **shared vectors** generated from the reference
  (`crates/raqeem-core/tests/vectors/normalize_ar.json`), so a case added once is a case
  both languages must satisfy.

- **`Transcriber::with_timeout` turned a recoverable error into a panic.** It fell back to
  `Client::new()` when the builder failed — a constructor that panics on the very same
  failure. Had it ever succeeded it would have installed reqwest's default 30s timeout,
  silently truncating the slow inference the parameter exists to allow.

- **`--endpoint` was accepted and ignored under `--provider cohere`**, so a request meant
  for your own server went to Cohere's hosted API, with your key. It is now rejected.

- **A broken pipe panicked.** `raqeem clip.wav | head -1` died with `failed printing to
  stdout`. It now exits cleanly, as a CLI in a pipeline should.

- **The release workflow could publish a release with a platform's binary missing.** Each
  job listed both a `.tar.gz` and a `.zip` with `fail_on_unmatched_files` disabled to
  tolerate the one it never built — which also tolerated the one it should have.

### Changed

- **Breaking: `Transcriber::new` and `Transcriber::with_timeout` return `Result`.** They
  can no longer paper over a client that failed to build. Add `?`.
- **Breaking: `--provider openai` now requires `--model`** (and `model=` in Python).
  It used to default to Cohere's dated model id, which a self-hosted server has no reason
  to recognise; the result was a confusing failure at the server rather than a clear one
  here.
- **Breaking: `Error` gained a `Client` variant and is now `#[non_exhaustive]`.** Matches
  on it need a `_` arm. The upside is that the next variant won't be a breaking change.
- The audio file is **streamed** to the endpoint instead of being read into memory first.
  Peak memory no longer scales with the length of the recording. Note that a disk error
  *during* the upload now surfaces as a transport error rather than `Error::ReadFile`.
- Error messages no longer paste an entire response body into the terminal — bodies are
  excerpted at 512 bytes — and no longer print the endpoint URL twice, which mattered
  because a URL can carry credentials in its userinfo.
- Credential resolution moved into `raqeem-core` (`resolve_api_key`). The CLI and the
  Python binding had each implemented the "a Cohere key never reaches a self-hosted
  endpoint" rule separately, kept in step by hand and by a comment promising they matched.
- `--timeout` rejects `0` and anything above 24 hours instead of accepting it.
- Release builds are LTO'd, single-codegen-unit, and stripped.

### Added

- `Provider` implements `FromStr` and `Display`, so backend names parse in one place.
- An MSRV (`rust-version = "1.82"`), so an old toolchain gets a clear message.
- `cargo audit` runs in CI, and `cargo test`/`clippy` now run `--locked`.

## [0.2.4] — 2026-07-28

Security release. If you have ever run `raqeem --help` with `$RAQEEM_API_KEY` set and shared
the output — a CI log, an issue, a screen share, a recording — **rotate that key.** Upgrading
does not un-leak a key that is already published somewhere.

### Security

- **`--help` printed the API key in plaintext.** clap renders the *value* of a set
  environment variable into help output unless `hide_env_values` is set, and it wasn't. With
  `$RAQEEM_API_KEY` exported, `raqeem --help` emitted
  `[env: RAQEEM_API_KEY=<your key>]`. It now shows the variable name only. The key was never
  transmitted anywhere it shouldn't have been — the exposure is entirely in help output that
  users copy into public places.

  Only `cargo install raqeem` picks this up by upgrading. Anyone running a prebuilt binary
  from a GitHub Release or the PyPI wheel stays exposed until they pull the new artifact.

- `--api-key` on the command line is visible to other processes (`ps`,
  `/proc/<pid>/cmdline`) and lands in shell history. The flag stays, but its help text now
  says so and points at `$RAQEEM_API_KEY` instead.

### Added

- Python 3.14 declared in the classifiers. The `cp39-abi3` wheel already loads on it and
  `requires-python` has no upper bound, so 0.2.3's list stopping at 3.13 made the badge read
  as if the current Python line were unsupported.

## [0.2.3] — 2026-07-27

Metadata only. No library, CLI, or binding behaviour changed from 0.2.2 — this release exists
to get corrected package metadata onto PyPI, since a published version's metadata is immutable.

### Fixed

- The PyPI package declared no `Programming Language :: Python :: 3.x` classifiers, so the
  README's Python badge rendered a red **missing** — shields.io reads those classifiers and
  ignores `requires-python`, which was set correctly the whole time. Classifiers for 3.9
  through 3.13 added; the badge can only pick them up from a release that uploads new
  metadata, since PyPI won't let a published version's metadata be edited.

## [0.2.2] — 2026-07-25

### Added

- **The crates are published to crates.io** — `cargo install raqeem` and
  `cargo add raqeem-core` now work without the `--git` flag. Nothing about the library or
  CLI changed; this release exists so that all three registries (crates.io, PyPI, GitHub
  Releases) carry the same version.
- **An Arabic README** ([README.ar.md](README.ar.md)), a full translation rather than a
  summary — install, usage, the Python API, the honest status section, and the roadmap.
  Both READMEs link to each other. An Arabic-first tool whose documentation was English-only
  was an odd thing to ship.
- Per-crate READMEs for `raqeem-core` (library-focused) and `raqeem` (CLI-focused), so
  each crates.io page reads for the audience that lands on it.
- A WER comparison chart in both READMEs ([assets/](assets/)) — hand-authored SVG, light and
  dark variants, RTL for the Arabic page. The bare three-row table said nothing about what WER
  measures or which direction is good; the chart says both. Same numbers, still Cohere Labs'.

### Changed

- **The CLI crate is named `raqeem`, not `raqeem-cli`** — so the install command matches the
  binary and matches `pip install raqeem`, and the bare name is held by this project rather
  than left open. The directory moved to `crates/raqeem/` to match. Nothing user-facing
  changed: the binary was already `raqeem`, and `raqeem-cli` was never published under that
  name, so no one can be depending on it.

### Fixed

- The CLI crate depended on `raqeem-core` **by path only**, which cannot be published — the
  dependency now carries a version alongside the path.
- **Four wrong claims on the README landing page**, all of them there since the README rewrite
  and all now corrected in both languages:
  - `pip install raqeem` was presented as though it also gave you the `raqeem` command. It
    doesn't — the wheel is an extension module with no console script. The CLI comes from
    `cargo install` or a release binary, and the READMEs now say so.
  - The `--api-key` row claimed API keys are *never* forwarded to self-hosted endpoints. Only
    the Cohere-scoped `$COHERE_API_KEY` is withheld; `--api-key` and `$RAQEEM_API_KEY` are
    sent wherever you point the tool. The behaviour was always right — the docs weren't.
  - `examples/serve_local.py` was listed as "unit tested via mocks". It has no tests at all
    and has never been run end to end.
  - `--provider openai` records `"provider": "openai-compatible"` in JSON output, which the
    options table never mentioned.
- Two typos in the Arabic README that changed meaning: the verification heading read
  الاختيارات (choices) instead of الاختبارات (tests), and المحكلي for المحلي. The Arabic page
  had also silently dropped the "undated aliases 404" caveat the English one carried.

### Verified

- **A live Arabic transcription now works** (2026-07-22) — the gap flagged under 0.1.0's
  *Known limitations*. Real Arabic speech went through both shipped artifacts (the PyPI wheel
  and the released binary) to Cohere's hosted API and returned correct Arabic text. The
  earlier hang did not reproduce, and its cause was never established — so the
  account-access explanation recorded under 0.1.0 is a guess that no longer holds. Docs
  only; no code changed.

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

[0.3.0]: https://github.com/SufficientDaikon/raqeem/releases/tag/v0.3.0
[0.2.4]: https://github.com/SufficientDaikon/raqeem/releases/tag/v0.2.4
[0.2.3]: https://github.com/SufficientDaikon/raqeem/releases/tag/v0.2.3
[0.2.2]: https://github.com/SufficientDaikon/raqeem/releases/tag/v0.2.2
[0.2.1]: https://github.com/SufficientDaikon/raqeem/releases/tag/v0.2.1
[0.2.0]: https://github.com/SufficientDaikon/raqeem/releases/tag/v0.2.0
[0.1.0]: https://github.com/SufficientDaikon/raqeem/releases/tag/v0.1.0
