"""Offline tests for the raqeem binding. No network, no API key, no model.

The normalization cases are **not** written out here. They live in
`crates/raqeem-core/tests/vectors/normalize_ar.json`, generated from scout's
`normalize_ar`, and the Rust suite reads the same file. Listing them separately in each
language is what let this binding's normalization drift a whole pass behind the reference
without either suite failing; a shared file has nothing to forget to copy.
"""

import json
import pathlib

import pytest

import raqeem

VECTORS_PATH = (
    pathlib.Path(__file__).resolve().parents[2]
    / "raqeem-core"
    / "tests"
    / "vectors"
    / "normalize_ar.json"
)


def load_vectors():
    if not VECTORS_PATH.exists():
        pytest.fail(
            f"shared normalization vectors not found at {VECTORS_PATH}. "
            "Run pytest from a repo checkout — these tests exist to catch drift "
            "between the Rust core and this binding, and silently skipping them "
            "would defeat the point."
        )
    return json.loads(VECTORS_PATH.read_text(encoding="utf-8"))


VECTORS = load_vectors()


@pytest.mark.parametrize(
    "vector", VECTORS, ids=[v["note"] for v in VECTORS]
)
def test_normalize_ar_matches_the_shared_vectors(vector):
    assert raqeem.normalize_ar(vector["input"]) == vector["expected"]


@pytest.mark.parametrize(
    "vector", VECTORS, ids=[v["note"] for v in VECTORS]
)
def test_normalize_ar_is_idempotent(vector):
    assert raqeem.normalize_ar(vector["expected"]) == vector["expected"]


def test_unknown_provider_raises_value_error():
    with pytest.raises(ValueError):
        raqeem.transcribe("clip.wav", provider="nope")


def test_openai_provider_requires_an_endpoint():
    with pytest.raises(ValueError):
        raqeem.transcribe("clip.wav", provider="openai")


def test_a_blank_api_key_is_not_a_key(monkeypatch):
    # An empty env var is an everyday state — an unresolved CI secret, `export KEY=` in a
    # script. It used to pass the key check and send `Authorization: Bearer `, producing an
    # opaque 401 from the server instead of this clear local error.
    monkeypatch.setenv("COHERE_API_KEY", "")
    monkeypatch.setenv("RAQEEM_API_KEY", "   ")
    with pytest.raises(ValueError):
        raqeem.transcribe("clip.wav")
    with pytest.raises(ValueError):
        raqeem.transcribe("clip.wav", api_key="")


def test_a_malformed_endpoint_is_rejected_by_name(monkeypatch):
    monkeypatch.delenv("COHERE_API_KEY", raising=False)
    monkeypatch.delenv("RAQEEM_API_KEY", raising=False)
    for bad in ("", "not-a-url", "ftp://host/v1"):
        with pytest.raises(ValueError, match="endpoint"):
            raqeem.transcribe("clip.wav", provider="openai", endpoint=bad, model="m")


def test_openai_provider_requires_an_explicit_model():
    # No silent fall-back to Cohere's dated model id: a self-hosted server has its own
    # ids, and sending Cohere's just fails at the server with a confusing error.
    with pytest.raises(ValueError, match="model"):
        raqeem.transcribe(
            "clip.wav",
            provider="openai",
            endpoint="http://127.0.0.1:9/v1/audio/transcriptions",
        )


def test_missing_cohere_key_raises_value_error(monkeypatch):
    monkeypatch.delenv("COHERE_API_KEY", raising=False)
    monkeypatch.delenv("RAQEEM_API_KEY", raising=False)
    with pytest.raises(ValueError):
        raqeem.transcribe("clip.wav")


def test_cohere_key_is_not_sent_to_a_self_hosted_endpoint(monkeypatch):
    # A Cohere-scoped key must not satisfy provider='openai'; that path still needs an
    # explicit endpoint, and the key never travels there. Mirrors the CLI's
    # `openai_never_uses_cohere_env` test.
    monkeypatch.setenv("COHERE_API_KEY", "cohere-only-key")
    monkeypatch.delenv("RAQEEM_API_KEY", raising=False)
    with pytest.raises(ValueError):
        raqeem.transcribe("clip.wav", provider="openai")


def test_unreachable_endpoint_raises_transcription_error(tmp_path):
    # A closed local port: proves a transport failure crosses the FFI boundary as a
    # Python exception rather than an abort/panic.
    clip = tmp_path / "clip.wav"
    clip.write_bytes(b"RIFF....WAVEfake")
    with pytest.raises(raqeem.TranscriptionError):
        raqeem.transcribe(
            str(clip),
            provider="openai",
            endpoint="http://127.0.0.1:9/v1/audio/transcriptions",
            model="whisper-1",
            timeout=5,
        )


def test_missing_audio_file_raises_transcription_error():
    with pytest.raises(raqeem.TranscriptionError):
        raqeem.transcribe(
            "definitely-not-here.wav",
            provider="openai",
            endpoint="http://127.0.0.1:9/v1/audio/transcriptions",
            model="whisper-1",
            timeout=5,
        )


def test_module_exposes_version():
    assert isinstance(raqeem.__version__, str) and raqeem.__version__
