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
    ``$COHERE_API_KEY``. With ``provider="openai"`` you must pass ``endpoint=``,
    and the Cohere-scoped key is deliberately never sent there.

    Raises:
        ValueError: bad provider, or a missing key/endpoint.
        TranscriptionError: the transcription itself failed.
    """

def normalize_ar(text: str) -> str:
    """Fold Arabic to a matching-stable form. Idempotent; safe on mixed scripts."""
