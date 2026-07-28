# رقيم · raqeem

**The easy way to use Cohere's open Arabic speech-recognition model from Python.**

`raqeem` wraps
[`CohereLabs/cohere-transcribe-arabic-07-2026`](https://huggingface.co/CohereLabs/cohere-transcribe-arabic-07-2026)
— the most accurate open-source Arabic ASR model (dialects + Arabic/English
code-switching), Apache-2.0.

It is a **compiled Rust extension**, not a pure-Python wrapper: no `torch`, no model
weights, no subprocess. Inference is delegated to an endpoint you choose — Cohere's
hosted API, or your own vLLM.

```bash
pip install raqeem
```

```python
import raqeem

# Cohere hosted API (reads $COHERE_API_KEY)
t = raqeem.transcribe("voice_note.ogg", lang="ar")
print(t.text)              # verbatim, for humans
print(t.text_normalized)   # Arabic-folded, for parsing
print(t.to_dict())

# your own vLLM — the Cohere key is never sent to a self-hosted endpoint
t = raqeem.transcribe(
    "clip.wav",
    provider="openai",
    endpoint="http://localhost:8000/v1/audio/transcriptions",
    model="CohereLabs/cohere-transcribe-arabic-07-2026",
)

# the Arabic normalizer on its own
raqeem.normalize_ar("الطماطم بـ ١٢٫٥ جنيه")   # 'الطماطم ب 12.5 جنيه'
```

`text_normalized` folds alef/hamza, taa-marbuta, strips tatweel and diacritics, removes the
invisible characters that come along with copied RTL text (zero-width joiners, bidi marks,
a stray BOM), and converts Arabic-Indic and Persian digits to ASCII — note `١٢٫٥` becomes
`12.5`, **one** number rather than two, which matters if anything downstream parses prices
or quantities. `normalize_ar` is idempotent and safe on mixed Arabic/ASCII, so you can run
it on both sides of a comparison.

Failures raise `raqeem.TranscriptionError`; a bad provider, or a missing key / endpoint /
model, raises `ValueError`. `transcribe` releases the GIL for the round-trip, so it does not
block other threads while the endpoint is working.

Full docs, the CLI, and the roadmap (subtitles, diarization, more bindings):
**https://github.com/SufficientDaikon/raqeem**

All model accuracy credit belongs to Cohere Labs — this package is the ergonomics.
Apache-2.0.
