# raqeem

The `raqeem` command (رقيم) — transcribe Arabic audio from the terminal, or from any
language, using
[`CohereLabs/cohere-transcribe-arabic-07-2026`](https://huggingface.co/CohereLabs/cohere-transcribe-arabic-07-2026),
the most accurate open-source Arabic speech-recognition model, Apache-2.0.

```bash
cargo install raqeem
```

A single static binary with no runtime — it loads no model weights. Inference is delegated
to an endpoint you choose:

```bash
export COHERE_API_KEY=...            # from dashboard.cohere.com/api-keys
raqeem voice_note.ogg --lang ar
```

Your own vLLM, no key and no rate limits:

```bash
raqeem clip.wav \
  --provider openai \
  --endpoint http://localhost:8000/v1/audio/transcriptions \
  --model CohereLabs/cohere-transcribe-arabic-07-2026 \
  --lang ar
```

`--format json` is what programs consume — verbatim text, an Arabic-normalized form, and
provenance:

```json
{
  "text": "الطماطم بـ ١٢٫٥ جنيه",
  "text_normalized": "الطماطم ب 12.5 جنيه",
  "provider": "cohere",
  "model": "cohere-transcribe-arabic-07-2026",
  "language": "ar"
}
```

That JSON contract is the language-agnostic API: shell out from Python, Node, Go, anything.
(Python has a native wheel instead — `pip install raqeem`.) It writes one line to stdout and
exits non-zero on failure with the reason on stderr, so `raqeem clip.wav | head -1` and the
usual pipeline habits behave.

The Cohere-scoped `$COHERE_API_KEY` is deliberately **never** sent to a self-hosted
`--endpoint`; use `--api-key` or `$RAQEEM_API_KEY` for those. Prefer the environment over
`--api-key`: an argument shows up in `ps` and in your shell history.

Prebuilt binaries, full docs, and the library:
[github.com/SufficientDaikon/raqeem](https://github.com/SufficientDaikon/raqeem).

Apache-2.0 — same as the model. All accuracy credit belongs to Cohere Labs.
