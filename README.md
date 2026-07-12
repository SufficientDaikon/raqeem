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

**English:** `tafrigh` is a lightweight client for
[`CohereLabs/cohere-transcribe-arabic-07-2026`](https://huggingface.co/CohereLabs/cohere-transcribe-arabic-07-2026)
— the world's most accurate open-source Arabic speech-recognition model (dialects +
Arabic/English code-switching), Apache-2.0. Give it an audio file, get Arabic text — from
the CLI or from any language by shelling out to the binary.

Inference is **always delegated** to an endpoint you choose (Cohere's hosted API, or your
own vLLM). `tafrigh` loads no weights, so it stays small and fast. It also folds the output
through Arabic normalization (alef/hamza, taa-marbuta, tatweel + diacritics, Arabic digits →
ASCII) so downstream parsers get a stable form.

> Built out of respect for Cohere Labs' work: a genuinely open, Apache-2.0 Arabic ASR model
> deserves a first-class developer on-ramp. All accuracy credit is theirs — this repo is
> just the ergonomics around it.

## Install / build

```bash
git clone https://github.com/SufficientDaikon/tafrigh
cd tafrigh
cargo build --release
# binary at target/release/tafrigh (single static executable, no runtime)
```

## Usage

**Cohere hosted API** (no GPU — just a key from https://dashboard.cohere.com):

```bash
export COHERE_API_KEY=...          # or pass --api-key
tafrigh voice_note.ogg --lang ar
```

**Your own vLLM** (self-hosted, OpenAI-compatible):

```bash
# on your GPU box / VPS:
#   vllm serve CohereLabs/cohere-transcribe-arabic-07-2026 --trust-remote-code
tafrigh clip.wav \
  --provider openai \
  --endpoint http://localhost:8000/v1/audio/transcriptions \
  --lang ar
```

**JSON output** (verbatim + normalized + provenance — this is what programs consume):

```bash
tafrigh voice_note.ogg --format json
```

```json
{
  "text": "الطماطم باتناشر جنيه",
  "text_normalized": "الطماطم باتناشر جنيه",
  "provider": "cohere",
  "model": "cohere-transcribe-arabic",
  "language": "ar"
}
```

Call it from anything — e.g. Python:

```python
import json, subprocess
out = subprocess.run(
    ["tafrigh", "note.ogg", "--provider", "cohere", "--lang", "ar", "--format", "json"],
    capture_output=True, text=True, check=True,
)
text = json.loads(out.stdout)["text_normalized"]
```

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

flac, mp3, mpeg, mpga, ogg, wav. Short clips are sent as-is (no local decode needed).

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
[`.claude/skills/`](.claude/skills/) — e.g. adding a new backend is a single skill:
*"add Deepgram support"* → the agent scaffolds the provider, endpoint, and test.

## License

Apache-2.0 — same as the model. See [LICENSE](LICENSE).
