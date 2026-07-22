# raqeem-core

The library behind [`raqeem`](https://github.com/SufficientDaikon/raqeem) (رقيم) — a
lightweight client for
[`CohereLabs/cohere-transcribe-arabic-07-2026`](https://huggingface.co/CohereLabs/cohere-transcribe-arabic-07-2026),
the most accurate open-source Arabic speech-recognition model (dialects + Arabic/English
code-switching), Apache-2.0.

**Inference is always delegated** to an endpoint you choose — Cohere's hosted API or your
own vLLM. This crate loads no model weights: it POSTs the audio to an OpenAI-compatible
`/audio/transcriptions` endpoint and folds the result. That is what keeps it small, and
what lets the same core drive the CLI, the Python wheel, and any future binding.

```rust
use raqeem_core::{Endpoint, Transcriber};
use std::path::Path;

let endpoint = Endpoint::cohere(std::env::var("COHERE_API_KEY")?, None);
let transcript = Transcriber::new(endpoint)
    .language("ar")                           // the default; shown for clarity
    .transcribe(Path::new("voice_note.ogg"))?;

println!("{}", transcript.text);              // verbatim, for humans
println!("{}", transcript.text_normalized);   // Arabic-folded, for parsing
```

Self-hosted instead — same `Transcriber`, different endpoint:

```rust
let endpoint = Endpoint::openai_compatible(
    "http://localhost:8000/v1/audio/transcriptions",
    "cohere-transcribe-arabic-07-2026",
    None,                                     // no key needed
);
```

`text_normalized` runs the output through `normalize_ar` — alef/hamza folding,
taa-marbuta → haa, tatweel and diacritics stripped, Arabic-Indic and Persian digits → ASCII
(`١٢٫٥` becomes `12.5`, one number) — so downstream matching gets a stable form.

Two things Cohere's API requires, both handled here: the `model` and `language` form fields
must precede the file part, and the model id must be **dated** (undated aliases 404).

Full docs, the CLI, and the Python bindings:
[github.com/SufficientDaikon/raqeem](https://github.com/SufficientDaikon/raqeem).

Apache-2.0 — same as the model. All accuracy credit belongs to Cohere Labs.
