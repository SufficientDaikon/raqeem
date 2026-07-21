"""Offline tests for the raqeem binding. No network, no API key, no model.

These deliberately mirror the Rust unit tests in `raqeem-core/src/arabic.rs`: the
binding must expose the *same* normalization, not a second implementation that can
drift from it.
"""

import pytest

import raqeem


def test_normalize_ar_matches_the_rust_core():
    assert raqeem.normalize_ar("أ") == "ا"
    assert raqeem.normalize_ar("مصطفى") == "مصطفي"  # alef maqsura -> yaa
    assert raqeem.normalize_ar("سـلام") == "سلام"  # tatweel stripped
    assert raqeem.normalize_ar("١٢٣") == "123"  # arabic-indic digits
    assert raqeem.normalize_ar("۱۲۳") == "123"  # persian digits
    assert raqeem.normalize_ar("١٢٫٥") == "12.5"  # U+066B -> one number, not two
    assert raqeem.normalize_ar("الطماطم بـ ١٢٫٥ جنيه") == "الطماطم ب 12.5 جنيه"


def test_normalize_ar_is_idempotent():
    once = raqeem.normalize_ar("طماطة ١٢٫٥ جنيه")
    assert raqeem.normalize_ar(once) == once


def test_unknown_provider_raises_value_error():
    with pytest.raises(ValueError):
        raqeem.transcribe("clip.wav", provider="nope")


def test_openai_provider_requires_an_endpoint():
    with pytest.raises(ValueError):
        raqeem.transcribe("clip.wav", provider="openai")


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
            timeout=5,
        )


def test_missing_audio_file_raises_transcription_error():
    with pytest.raises(raqeem.TranscriptionError):
        raqeem.transcribe(
            "definitely-not-here.wav",
            provider="openai",
            endpoint="http://127.0.0.1:9/v1/audio/transcriptions",
            timeout=5,
        )


def test_module_exposes_version():
    assert isinstance(raqeem.__version__, str) and raqeem.__version__
