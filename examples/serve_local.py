"""Dev-only local model server for raqeem — NOT part of the lightweight client.

This is the heavy, model-side half (torch). It loads
`CohereLabs/cohere-transcribe-arabic-07-2026` via transformers and exposes the
OpenAI-compatible `POST /v1/audio/transcriptions` endpoint that
`raqeem --provider openai` talks to. Run this where the model lives; point the
tiny Rust client at it.

Runs on **CPU out of the box** (slow but works anywhere, needs ~8-10 GB RAM) and
uses a **GPU automatically** if torch sees one — NVIDIA (CUDA) or AMD (ROCm, which
today means Linux or WSL2; native Windows ROCm only really covers RDNA4). On plain
Windows with an AMD card, this runs on CPU.

Setup (in a fresh venv):
    pip install "transformers>=5.4.0" torch soundfile librosa sentencepiece \
        protobuf accelerate fastapi "uvicorn[standard]" python-multipart

Run:
    python serve_local.py                 # serves on http://localhost:8000

Then, from the repo:
    raqeem clip.wav --provider openai \
        --endpoint http://localhost:8000/v1/audio/transcriptions \
        --model CohereLabs/cohere-transcribe-arabic-07-2026 \
        --lang ar --format json

(`--model` is required with `--provider openai`. This server serves one model and ignores
the field, but raqeem will not invent a model id for a server it knows nothing about.)

Note: follows the model card's documented transformers API. It has not been run
end-to-end against the live weights from here, so if an API name shifted in your
transformers version it's a one-line fix (the two lines that matter are the
`processor(...)` call and `model.generate(...)`).
"""

from __future__ import annotations

import io

import librosa
import torch
import uvicorn
from fastapi import FastAPI, Form, UploadFile
from transformers import AutoProcessor, CohereAsrForConditionalGeneration

MODEL_ID = "CohereLabs/cohere-transcribe-arabic-07-2026"

print(f"cuda/ROCm visible to torch: {torch.cuda.is_available()}")
print(f"loading {MODEL_ID} (first run downloads ~4-5 GB)...")

# device_map="auto" places the model on a GPU if torch sees one, else CPU.
PROCESSOR = AutoProcessor.from_pretrained(MODEL_ID)
MODEL = CohereAsrForConditionalGeneration.from_pretrained(MODEL_ID, device_map="auto")
MODEL.eval()
print(f"ready on device: {MODEL.device}")

app = FastAPI()


@app.post("/v1/audio/transcriptions")
async def transcribe(
    file: UploadFile,
    model: str = Form(default=""),  # accepted for OpenAI-compat; we serve one model
    language: str = Form(default="ar"),
):
    raw = await file.read()
    # Decode to 16 kHz mono float32. librosa handles wav/flac/ogg via soundfile;
    # mp3 needs a recent libsndfile or ffmpeg on PATH.
    audio, _ = librosa.load(io.BytesIO(raw), sr=16000, mono=True)
    inputs = PROCESSOR(
        audio, sampling_rate=16000, return_tensors="pt", language=language
    ).to(MODEL.device)
    with torch.inference_mode():
        outputs = MODEL.generate(**inputs, max_new_tokens=256)
    return {"text": PROCESSOR.batch_decode(outputs, skip_special_tokens=True)[0]}


if __name__ == "__main__":
    uvicorn.run(app, host="0.0.0.0", port=8000)
