---
name: add-endpoint-adapter
description: Add a new transcription backend to raqeem (e.g. "add Deepgram support", "add a Groq endpoint", "support another OpenAI-compatible server"). Use whenever a maintainer asks to make raqeem talk to a new transcription provider or self-hosted server. Covers the exact three edits — a Provider variant, an Endpoint constructor, and a mocked test — plus the invariants (delegated inference, offline tests, Arabic-first) so a new backend drops in without touching the request path.
---

# Add an endpoint adapter to raqeem

Every backend raqeem supports speaks the **same** OpenAI-compatible multipart
`/audio/transcriptions` shape (`file` + `model` + `language`, optional Bearer auth). So
"add a backend" is almost never a new request path — it's a new **preset**: where to POST,
how to authenticate, and the default model id. Three small edits, one test.

## Before you touch anything

Confirm the new backend actually returns `{"text": "..."}` (or note how it differs). If the
response shape differs, the only place to adjust is `extract_text` in
`crates/raqeem-core/src/lib.rs` — add a branch, do **not** invent a new response type.

Read `CONTRIBUTING.md` invariants: inference stays delegated (no ML deps in the core),
tests stay offline (mock the HTTP), Arabic-first stays intact (`text_normalized` always
produced — you get this for free by going through `Transcriber`).

## Step 1 — add the `Provider` variant

In `crates/raqeem-core/src/provider.rs` there are **four** places, not two. Miss one of the
last two and it still compiles:

```rust
pub enum Provider {
    Cohere,
    OpenAiCompatible,
    Deepgram, // <- new
}

// in as_str() — the provenance tag written onto every Transcript:
Provider::Deepgram => "deepgram",

// in ALL — so enumerations and the parity tests see it:
pub const ALL: &'static [Provider] = &[/* ... */, Provider::Deepgram];

// in ACCEPTED_NAMES — the spelling a *user* types, which is not always the tag
// ("openai-compatible" is a tag; users type "openai"):
pub const ACCEPTED_NAMES: &'static [&'static str] = &[/* ... */, "deepgram"];

// and in FromStr::from_str:
"deepgram" => Ok(Provider::Deepgram),
```

`every_variant_is_reachable_by_an_advertised_name` and `every_advertised_name_parses` are
what catch a half-done job here. Run them.

## Step 2 — add the `Endpoint` constructor

In `crates/raqeem-core/src/endpoint.rs`, add the URL/model constants and a constructor
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

## Step 3 — decide the credential rule

`resolve_api_key` in `crates/raqeem-core/src/credentials.rs` matches on `Provider`, so
adding a variant **will not compile** until you say which key sources the new backend may
draw from. That is deliberate: the rule that a vendor-scoped key never reaches someone
else's server is the one thing here that must not be decided by accident.

The default for a new hosted backend is `explicit.or(its_own_vendor_env)` — never another
vendor's. Add the case to `credentials.rs`'s tests alongside it; that module is the single
source of truth, and the CLI and Python binding both call into it rather than deciding
for themselves.

## Step 4 — wire the CLI (only if it's a first-class preset)

In `crates/raqeem/src/main.rs`, add a `ProviderArg` variant, map it in the
`From<ProviderArg> for Provider` impl, and handle it in `run()`.
If the backend is "just another OpenAI-compatible URL," you can skip this — users reach it
today with `--provider openai --endpoint <url> --model <id>`.

## Step 5 — a mocked test (required, offline)

Copy the pattern from `crates/raqeem-core/tests/endpoint_mock.rs`: start a `mockito`
server, assert the multipart body carries `file`/`model`/`language` and the auth header,
return a canned Arabic `{"text": ...}`, and check `text_normalized` folded correctly. No
network, no key. Use the `fake_clip` helper rather than writing to a fixed temp path.

## Done when

- `cargo test` green (offline), `cargo build --release` succeeds.
- All four spots in `provider.rs` updated: variant, `as_str`, `ALL`, `ACCEPTED_NAMES`
  (plus `FromStr`). The two parity tests there are what prove it.
- `resolve_api_key` has a case for the new provider, with a test asserting which key
  sources it may draw from — and, crucially, which it may not.
- The new backend has an `Endpoint` constructor and a mocked test.
- `extract_text` handles the response (branch added only if the shape differs).
- README's backend list / usage updated if it's a first-class CLI preset — that means
  `README.md`, `README.ar.md`, the per-crate READMEs, **and** `examples/`.
