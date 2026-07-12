---
name: add-endpoint-adapter
description: Add a new transcription backend to tafrigh (e.g. "add Deepgram support", "add a Groq endpoint", "support another OpenAI-compatible server"). Use whenever a maintainer asks to make tafrigh talk to a new transcription provider or self-hosted server. Covers the exact three edits — a Provider variant, an Endpoint constructor, and a mocked test — plus the invariants (delegated inference, offline tests, Arabic-first) so a new backend drops in without touching the request path.
---

# Add an endpoint adapter to tafrigh

Every backend tafrigh supports speaks the **same** OpenAI-compatible multipart
`/audio/transcriptions` shape (`file` + `model` + `language`, optional Bearer auth). So
"add a backend" is almost never a new request path — it's a new **preset**: where to POST,
how to authenticate, and the default model id. Three small edits, one test.

## Before you touch anything

Confirm the new backend actually returns `{"text": "..."}` (or note how it differs). If the
response shape differs, the only place to adjust is `extract_text` in
`crates/tafrigh-core/src/lib.rs` — add a branch, do **not** invent a new response type.

Read `CONTRIBUTING.md` invariants: inference stays delegated (no ML deps in the core),
tests stay offline (mock the HTTP), Arabic-first stays intact (`text_normalized` always
produced — you get this for free by going through `Transcriber`).

## Step 1 — add the `Provider` variant

In `crates/tafrigh-core/src/provider.rs`, add a variant and its `as_str` tag:

```rust
pub enum Provider {
    Cohere,
    OpenAiCompatible,
    Deepgram, // <- new
}

// in as_str():
Provider::Deepgram => "deepgram",
```

## Step 2 — add the `Endpoint` constructor

In `crates/tafrigh-core/src/endpoint.rs`, add the URL/model constants and a constructor
mirroring `cohere`:

```rust
pub const DEEPGRAM_URL: &str = "https://api.deepgram.com/v1/audio/transcriptions";
pub const DEFAULT_DEEPGRAM_MODEL: &str = "...";

impl Endpoint {
    pub fn deepgram(api_key: impl Into<String>, model: Option<String>) -> Self {
        Endpoint {
            url: DEEPGRAM_URL.to_string(),
            api_key: Some(api_key.into()),
            model: model.unwrap_or_else(|| DEFAULT_DEEPGRAM_MODEL.to_string()),
            provider: Provider::Deepgram,
        }
    }
}
```

Re-export the new consts from `lib.rs` if the CLI needs them.

## Step 3 — wire the CLI (only if it's a first-class preset)

In `crates/tafrigh-cli/src/main.rs`, add a `ProviderArg` variant and map it in `run()`.
If the backend is "just another OpenAI-compatible URL," you can skip this — users reach it
today with `--provider openai --endpoint <url>`.

## Step 4 — a mocked test (required, offline)

Copy the pattern from `crates/tafrigh-core/tests/endpoint_mock.rs`: start a `mockito`
server, assert the multipart body carries `file`/`model`/`language` and the auth header,
return a canned Arabic `{"text": ...}`, and check `text_normalized` folded correctly. No
network, no key.

## Done when

- `cargo test` green (offline), `cargo build --release` succeeds.
- The new backend has a `Provider` variant, an `Endpoint` constructor, and a mocked test.
- `extract_text` handles the response (branch added only if the shape differs).
- README's backend list / usage updated if it's a first-class CLI preset.
