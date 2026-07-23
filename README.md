<div align="center">

# رقيم · raqeem

### World-class Arabic speech-to-text — one line, no GPU, no model weights.

**English** · [العربية](README.ar.md)

[![crates.io](https://img.shields.io/crates/v/raqeem?logo=rust&label=crates.io&color=E43716)](https://crates.io/crates/raqeem)
[![PyPI](https://img.shields.io/pypi/v/raqeem?logo=pypi&logoColor=white&label=PyPI&color=3775A9)](https://pypi.org/project/raqeem/)
[![Python](https://img.shields.io/pypi/pyversions/raqeem?logo=python&logoColor=white)](https://pypi.org/project/raqeem/)
[![CI](https://github.com/SufficientDaikon/raqeem/actions/workflows/ci.yml/badge.svg)](https://github.com/SufficientDaikon/raqeem/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

</div>

`raqeem` (رقيم) is the easy way to use
[`CohereLabs/cohere-transcribe-arabic-07-2026`](https://huggingface.co/CohereLabs/cohere-transcribe-arabic-07-2026)
— the most accurate **open-source** Arabic speech-recognition model in the world, Apache-2.0.
Hand it an audio file, get Arabic text back: from Python, from the terminal, or from any
language at all.

It carries **no model weights**. Inference is delegated to an endpoint you choose — Cohere's
hosted API, or your own vLLM — so the whole thing is a single small binary (and a compiled
Python extension) with no `torch`, no CUDA, and nothing to download but the tool itself.

> Built out of respect for Cohere Labs' work: a genuinely open, Apache-2.0 Arabic ASR model
> deserves a first-class developer on-ramp. All accuracy credit is theirs — this repo is just
> the ergonomics around it.

## Quickstart

```bash
pip install raqeem
export COHERE_API_KEY=...          # a free key from dashboard.cohere.com/api-keys
```

```python
import raqeem

t = raqeem.transcribe("voice_note.ogg")   # Arabic by default
print(t.text)                             #  → الطماطم بـ ١٢٫٥ جنيه
print(t.text_normalized)                  #  → الطماطم ب 12.5 جنيه
```

Prefer the terminal? Same result, no Python:

```bash
raqeem voice_note.ogg
```

That's it. No GPU, no weights, no setup beyond a key.

## Why raqeem

- **The best open Arabic ASR there is.** Cohere's 2B Conformer model tops the Hugging Face
  Arabic ASR leaderboard — about **11 WER points** better than Whisper Large v3, and strong
  where Arabic is hardest: dialects (Egyptian, Gulf, Levantine, Maghrebi) and Arabic-English
  code-switching.
- **Featherweight by design.** The client loads no weights and runs no model — it POSTs your
  audio and folds the reply. A static binary, a `torch`-free wheel. Nothing heavy gets pulled
  in for a feature you didn't ask for.
- **You choose where inference runs.** Cohere's hosted API when you want zero infrastructure,
  or your own self-hosted vLLM when you want no rate limits and no data leaving your box —
  same interface either way.
- **Two forms of every transcript.** The model's verbatim text *and* an Arabic-normalized
  form — alef/hamza folded, taa-marbuta and tatweel and diacritics handled, Arabic-Indic
  digits turned to ASCII. `١٢٫٥` becomes `12.5` as **one** number. That is the difference
  between a transcript you can read and one a program can parse.
- **Callable from anything.** Rust core, native Python wheel, and a single binary that prints
  JSON — so Go, Node, a shell script, or a cron job all drive the exact same engine.
- **Honest about its limits.** The [status section](#status--what-we-actually-verified) below
  says plainly what has been run against a live model and what has not.
- **Built to be extended by AI agents.** Adding a new backend is one skill invocation — see
  [Contributing](#contributing-humans-and-ai-agents).

## Install

**Python** — a compiled extension, no `torch`, no subprocess:

```bash
pip install raqeem
```

**With cargo** — installs the `raqeem` binary:

```bash
cargo install raqeem
```

**Prebuilt binary** — grab one for Linux / macOS / Windows from
[Releases](https://github.com/SufficientDaikon/raqeem/releases). No runtime, no dependencies.

**From source:**

```bash
git clone https://github.com/SufficientDaikon/raqeem
cd raqeem && cargo build --release   # binary at target/release/raqeem
```

## Usage

**Cohere hosted API** (no GPU — just a key from [dashboard.cohere.com](https://dashboard.cohere.com/api-keys)):

```bash
export COHERE_API_KEY=...          # or pass --api-key
raqeem voice_note.ogg --lang ar
```

**Your own vLLM** (self-hosted, OpenAI-compatible — no API key, no rate limits):

```bash
# on your GPU box / VPS:
#   vllm serve CohereLabs/cohere-transcribe-arabic-07-2026 --trust-remote-code
raqeem clip.wav \
  --provider openai \
  --endpoint http://localhost:8000/v1/audio/transcriptions \
  --lang ar
```

No GPU? [`examples/serve_local.py`](examples/serve_local.py) runs the model on CPU behind
the same endpoint.

**JSON output** (verbatim + normalized + provenance — this is what programs consume):

```bash
raqeem voice_note.ogg --format json
```

```json
{
  "text": "الطماطم بـ ١٢٫٥ جنيه",
  "text_normalized": "الطماطم ب 12.5 جنيه",
  "provider": "cohere",
  "model": "cohere-transcribe-arabic-07-2026",
  "language": "ar"
}
```

`text` is the model verbatim (show this to a human); `text_normalized` is folded for
matching — note the tatweel stripped and `١٢٫٥` → `12.5` as **one** number, not two.

## From Python

`pip install raqeem` gives you a **native extension** built from this same Rust core —
no `torch`, no model weights, no subprocess:

```python
import raqeem

t = raqeem.transcribe("voice_note.ogg", lang="ar")   # reads $COHERE_API_KEY
print(t.text)              # verbatim, for humans
print(t.text_normalized)   # Arabic-folded, for parsing
print(t.to_dict())

# your own vLLM — the Cohere-scoped key is deliberately never sent to a self-hosted endpoint
raqeem.transcribe("clip.wav", provider="openai",
                  endpoint="http://localhost:8000/v1/audio/transcriptions")

raqeem.normalize_ar("الطماطم بـ ١٢٫٥ جنيه")   # 'الطماطم ب 12.5 جنيه'
```

Failures raise `raqeem.TranscriptionError`; a bad provider or a missing key/endpoint
raises `ValueError`. Type stubs ship with the wheel, so editors autocomplete. One `abi3`
wheel per OS/arch covers CPython 3.9+.

Any other language can drive the binary directly — `raqeem clip.ogg --format json` prints
the same JSON to stdout.

### Options worth knowing

| Flag | Default | Notes |
|---|---|---|
| `--model` | `cohere-transcribe-arabic-07-2026` | Cohere needs a **dated** id — undated aliases 404. |
| `--timeout` | `300` | Seconds, covers upload + inference + download. Raise for slow CPU inference. |
| `--api-key` | — | Falls back to `$RAQEEM_API_KEY`. For `--provider cohere` **only**, also `$COHERE_API_KEY` — that Cohere-scoped key is deliberately never sent to a self-hosted `--endpoint`. |

## Status — what we actually verified

- **Offline test suite green** (`cargo test`): Arabic normalization units, plus mocked-endpoint
  tests asserting the multipart shape, field order, bearer auth, and error handling.
- **A live Arabic transcription is verified** (2026-07-22), against Cohere's hosted API and
  through *both* shipped artifacts — the PyPI wheel and the released binary. Real Arabic
  speech in, correct Arabic text out: a 611 KB narration came back as an accurate paragraph
  in ~2s, with Arabic-Indic digits (`٢٠٠٣`) folding to ASCII in `text_normalized` as intended.
  An earlier attempt during development hung; it has not reproduced, and the cause was never
  established.
- Two real bugs were caught by live calls and fixed: Cohere requires the `model` and
  `language` form fields *before* the file part, and it 404s undated model ids.
- **Not verified: the self-hosted paths.** The `openai` provider (vLLM and friends) and
  [`examples/serve_local.py`](examples/serve_local.py) are implemented and covered by
  mocked-endpoint tests, but have never been run against a real self-hosted server. The
  request they send is the same shape Cohere accepts — treat it as sound but undemonstrated.
- The accuracy numbers below are **Cohere's**, from the model card. Nothing here benchmarks
  the model's dialect or code-switching performance.

## The model (all credit: Cohere Labs)

2B-parameter Conformer encoder + Transformer decoder, Apache-2.0. Best open-source Arabic
ASR on the Hugging Face Arabic ASR leaderboard.

| Model | Avg WER ↓ |
|---|---|
| **Cohere Transcribe Arabic** | **25.87** |
| OmniASR-7B-LLM | 28.32 |
| Whisper Large v3 | 36.86 |

Also: RTFx ≈ 525 (much faster than Whisper), preferred over Whisper in 95.8% of human
tests, covers MSA + Egyptian/Gulf/Levantine/Maghrebi + English + code-switching.

**What the model gives you:** text only. No timestamps, diarization, language detection, or
VAD. Those are `raqeem`'s roadmap, not the model's job.

## Supported audio

flac, mp3, mpeg, mpga, ogg, wav. Clips are sent as-is (no local decode needed).

## Roadmap

Each item is opt-in and must earn its keep — the core never grows a heavy dependency for a
feature you didn't ask for.

- Long-form audio: VAD chunking → **subtitles (SRT / VTT)** with real segment timestamps.
- Optional **speaker diarization** (separate endpoint/module, never a core dep).
- Optional punctuation / diacritics restoration.
- More native **bindings** (Node/Bun, WASM) from the one Rust core, the way Python already
  is — one implementation, no second copy of the logic to drift.

## Contributing (humans and AI agents)

This repo is built to be extended by AI. See [CONTRIBUTING.md](CONTRIBUTING.md) and
[`.claude/skills/`](.claude/skills/) — adding a new backend is a single skill:
*"add Deepgram support"* → the agent scaffolds the provider, endpoint, and test.

## License

Apache-2.0 — same as the model. See [LICENSE](LICENSE).
