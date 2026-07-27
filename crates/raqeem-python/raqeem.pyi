"""Type stubs for raqeem — Cohere's open Arabic ASR, from Python."""

from typing import Any, Literal

__version__: str

class TranscriptionError(Exception):
    """Transcription failed: unreadable audio, transport error, an HTTP error from
    the endpoint, or an unparseable response."""

class Transcript:
    """A completed transcription."""

    @property
    def text(self) -> str:
        """The model's verbatim output — show this to a human."""

    @property
    def text_normalized(self) -> str:
        """Arabic-folded form — parse/match against this."""

    @property
    def provider(self) -> str: ...
    @property
    def model(self) -> str: ...
    @property
    def language(self) -> str: ...
    def to_dict(self) -> dict[str, Any]:
        """The transcript as a plain dict (same shape as the CLI's --format json)."""

def transcribe(
    path: str,
    *,
    lang: str = "ar",
    provider: Literal["cohere", "openai"] = "cohere",
    api_key: str | None = None,
    endpoint: str | None = None,
    model: str | None = None,
    timeout: int | None = None,
) -> Transcript:
    """Transcribe an audio file.

    With ``provider="cohere"`` the key falls back to ``$RAQEEM_API_KEY`` then
    ``$COHERE_API_KEY``, and ``model`` defaults to Cohere's current dated Arabic model.

    With ``provider="openai"`` both ``endpoint=`` and ``model=`` are required — your
    server has its own model ids, and defaulting to Cohere's only produced a confusing
    failure at the server. The Cohere-scoped ``$COHERE_API_KEY`` is deliberately never
    sent to a self-hosted endpoint; ``$RAQEEM_API_KEY`` is, since it is raqeem's own.

    Raises:
        ValueError: bad provider, or a missing key/endpoint/model.
        TranscriptionError: the transcription itself failed.
    """

def normalize_ar(text: str) -> str:
    """Fold Arabic to a matching-stable form. Idempotent; safe on mixed scripts."""
