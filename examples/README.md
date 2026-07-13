# Examples

No audio is committed here (binary blobs don't belong in git). Bring any short
Arabic clip — `flac`, `mp3`, `mpeg`, `mpga`, `ogg`, or `wav` — and run:

## Cohere hosted API

```bash
export COHERE_API_KEY=...
tafrigh your_clip.ogg --lang ar
tafrigh your_clip.ogg --lang ar --format json
```

## Self-hosted vLLM

```bash
# on the GPU box:
#   vllm serve CohereLabs/cohere-transcribe-arabic-07-2026 --trust-remote-code
tafrigh your_clip.wav \
  --provider openai \
  --endpoint http://localhost:8000/v1/audio/transcriptions \
  --lang ar --format json
```

## Run the model locally (dev, no cloud)

[`serve_local.py`](serve_local.py) is a small torch server that loads the model and
exposes the OpenAI-compatible endpoint `tafrigh --provider openai` talks to. It runs on
**CPU anywhere** and uses a GPU automatically if torch sees one (NVIDIA CUDA, or AMD ROCm
on Linux/WSL2 — native Windows ROCm only covers RDNA4 today, so AMD-on-Windows falls back
to CPU). See the file's docstring for setup. This is the heavy model-side half — it is
**not** part of the lightweight Rust client.

## Grab a test clip quickly

Cohere's demo Space accepts recordings if you just want to hear the model:
https://huggingface.co/spaces/CohereLabs/cohere-transcribe-arabic-07-2026

Or record a few seconds of Arabic to `clip.wav` with any recorder and point `tafrigh` at it.
