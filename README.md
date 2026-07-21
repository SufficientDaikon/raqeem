<div dir="rtl">

# تفريغ · tafrigh

**أسهل طريقة لاستخدام نموذج Cohere المفتوح للتعرّف على الكلام العربي.**

`tafrigh` (تفريغ) غلافٌ خفيف حول
[`CohereLabs/cohere-transcribe-arabic-07-2026`](https://huggingface.co/CohereLabs/cohere-transcribe-arabic-07-2026)
— أدقّ نموذج مفتوح المصدر في العالم للتعرّف على الكلام العربي (لهجات + عربي/إنجليزي مختلط)،
تحت رخصة Apache 2.0. يأخذ ملفًّا صوتيًّا ويُرجِع نصًّا عربيًّا — من سطر الأوامر أو من أي
لغة برمجة عبر استدعاء الملف التنفيذي.

الاستدلال (inference) دائمًا عبر نقطة نهاية خارجية — إمّا واجهة Cohere المستضافة، أو خادم
vLLM تشغّله بنفسك. الأداة لا تُحمّل أوزان النموذج، لذلك تبقى خفيفة وسريعة.

</div>

---

[![CI](https://github.com/SufficientDaikon/tafrigh/actions/workflows/ci.yml/badge.svg)](https://github.com/SufficientDaikon/tafrigh/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

**English:** `tafrigh` is a lightweight client for
[`CohereLabs/cohere-transcribe-arabic-07-2026`](https://huggingface.co/CohereLabs/cohere-transcribe-arabic-07-2026)
— the world's most accurate open-source Arabic speech-recognition model (dialects +
Arabic/English code-switching), Apache-2.0. Give it an audio file, get Arabic text — from
the CLI or from any language by shelling out to the binary.

Inference is **always delegated** to an endpoint you choose (Cohere's hosted API, or your
own vLLM). `tafrigh` loads no weights, so it stays small and fast — a single static binary
with no runtime. It also folds the output through Arabic normalization (alef/hamza,
taa-marbuta, tatweel + diacritics, Arabic digits → ASCII) so downstream parsers get a
stable form.

> Built out of respect for Cohere Labs' work: a genuinely open, Apache-2.0 Arabic ASR model
> deserves a first-class developer on-ramp. All accuracy credit is theirs — this repo is
> just the ergonomics around it.

## Install

**Prebuilt binary** — grab one for Linux / macOS / Windows from
[Releases](https://github.com/SufficientDaikon/tafrigh/releases). No runtime, no
dependencies.

**With cargo:**

```bash
cargo install --git https://github.com/SufficientDaikon/tafrigh tafrigh-cli
```

**From source:**

```bash
git clone https://github.com/SufficientDaikon/tafrigh
cd tafrigh && cargo build --release   # binary at target/release/tafrigh
```

## Usage

**Cohere hosted API** (no GPU — just a key from [dashboard.cohere.com](https://dashboard.cohere.com/api-keys)):

```bash
export COHERE_API_KEY=...          # or pass --api-key
tafrigh voice_note.ogg --lang ar
```

**Your own vLLM** (self-hosted, OpenAI-compatible — no API key, no rate limits):

```bash
# on your GPU box / VPS:
#   vllm serve CohereLabs/cohere-transcribe-arabic-07-2026 --trust-remote-code
tafrigh clip.wav \
  --provider openai \
  --endpoint http://localhost:8000/v1/audio/transcriptions \
  --lang ar
```

No GPU? [`examples/serve_local.py`](examples/serve_local.py) runs the model on CPU behind
the same endpoint.

**JSON output** (verbatim + normalized + provenance — this is what programs consume):

```bash
tafrigh voice_note.ogg --format json
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

Call it from anything — e.g. Python:

```python
import json, subprocess
out = subprocess.run(
    ["tafrigh", "note.ogg", "--lang", "ar", "--format", "json"],
    capture_output=True, text=True, encoding="utf-8", check=True,
)
text = json.loads(out.stdout)["text_normalized"]
```

### Options worth knowing

| Flag | Default | Notes |
|---|---|---|
| `--model` | `cohere-transcribe-arabic-07-2026` | Cohere needs a **dated** id — undated aliases 404. |
| `--timeout` | `300` | Seconds, covers upload + inference + download. Raise for slow CPU inference. |
| `--api-key` | — | Falls back to `$TAFRIGH_API_KEY`. For `--provider cohere` **only**, also `$COHERE_API_KEY` — that Cohere-scoped key is deliberately never sent to a self-hosted `--endpoint`. |

## Status — what's verified, honestly

- **Offline test suite green** (`cargo test`): Arabic normalization units, plus mocked-endpoint
  tests asserting the multipart shape, field order, bearer auth, and error handling.
- **The live round-trip is proven** end-to-end against Cohere's real API — audio in, HTTP 200,
  transcript out. Two real bugs were caught that way and fixed: Cohere requires the `model`
  and `language` form fields *before* the file part, and it 404s undated model ids.
- **Not yet verified: a live *Arabic* transcription.** On the trial key available during
  development, the Arabic model never responds (the request hangs rather than returning 403),
  while the *same* request on a dated English model returns instantly. The bytes we send are
  identical either way, so this looks like account/model access rather than a client bug — but
  it is unproven, and you should know that before trusting it in production. Self-hosting via
  vLLM sidesteps it entirely.

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
VAD. Those are `tafrigh`'s roadmap, not the model's job.

## Supported audio

flac, mp3, mpeg, mpga, ogg, wav. Clips are sent as-is (no local decode needed).

## Roadmap

Each item is opt-in and must earn its keep — the core never grows a heavy dependency for a
feature you didn't ask for.

- Long-form audio: VAD chunking → **subtitles (SRT / VTT)** with real segment timestamps.
- Optional **speaker diarization** (separate endpoint/module, never a core dep).
- Optional punctuation / diacritics restoration.
- Native **bindings** (Python, Node/Bun, WASM) from the one Rust core — the first binding
  is the flagship demo of the AI-contributor toolkit.

## Contributing (humans and AI agents)

This repo is built to be extended by AI. See [CONTRIBUTING.md](CONTRIBUTING.md) and
[`.claude/skills/`](.claude/skills/) — adding a new backend is a single skill:
*"add Deepgram support"* → the agent scaffolds the provider, endpoint, and test.

## License

Apache-2.0 — same as the model. See [LICENSE](LICENSE).
