# Shared `normalize_ar` vectors

`normalize_ar.json` is read by **both** test suites:

- Rust — `crates/raqeem-core/tests/normalize_vectors.rs`
- Python — `crates/raqeem-python/tests/test_raqeem.py`

Each entry is `{input, expected, note}`. Add a case here and both languages must satisfy
it; there is nothing to remember to copy into a second file.

## Why this file exists

`normalize_ar` is one function maintained in two languages, with the source of truth in
scout (`scout/arabic.py`, git root `E:\work`). Before 0.3.0 the two suites listed their
cases independently, and the Rust port fell an entire pass behind the reference — scout
added `_FORMAT_STRIP` in commit `0ffcf36`, raqeem didn't follow, and invisible characters
survived normalization for as long as it took someone to go looking. Both suites stayed
green throughout, because neither had a case containing a zero-width joiner.

## Regenerating

`expected` values come from the reference, not from raqeem, so they cannot be wrong in the
same direction as a bug in the port:

```python
import sys; sys.path.insert(0, "E:/work")
from scout.arabic import normalize_ar
normalize_ar(your_input)
```

## Vectors are not sufficient on their own

Thirty-odd hand-picked cases could not have caught the drift they now guard against — the
gap was in a range nobody had thought to test. Both divergences found so far came from
sweeping *every* plausible codepoint and comparing the implementations directly, never from
choosing cases:

- the `_FORMAT_STRIP` set the port was missing
- U+001C–001F, which Python's `\s` collapses and `char::is_whitespace` does not

If you change either implementation, re-run the sweep rather than trusting this file. It
needs scout importable and a built binding, which is why it isn't a CI job.

The full BMP, plus the multi-character fuzz that the single-character sweep cannot cover
(interactions between passes, and the whitespace state machine):

```python
import random, sys
sys.path.insert(0, "E:/work")
from scout.arabic import normalize_ar as py
import raqeem

# Known-accepted: case mapping of cased non-Arabic letters, where the Rust toolchain's
# Unicode tables and the Python runtime's differ. None are in Arabic or ASCII.
ACCEPTED = {0x03A3, 0x1C89, 0xA7CB, 0xA7CC, 0xA7CE, 0xA7D2, 0xA7D4, 0xA7DA, 0xA7DC}

bad = set()
for cp in range(0x0000, 0x10000):
    if 0xD800 <= cp < 0xE000 or cp in ACCEPTED:
        continue
    c = chr(cp)
    for probe in (c, f"  a {c}  b  ", f"{c}{c} طماطة {c}", f"١٢٫٥{c}جنيه"):
        if raqeem.normalize_ar(probe) != py(probe):
            bad.add(cp)
print("single-char:", sorted(hex(c) for c in bad) or "no divergence")

alphabet = [chr(c) for c in range(0x0621, 0x0700)] + list("آأإٱىةؤئ \t\n") + \
           [chr(c) for c in range(0x200B, 0x2010)] + ["\uFEFF", "\u001C"] + list("abc0123.")
random.seed(1)
fuzz_bad = []
for _ in range(200_000):
    s = "".join(random.choice(alphabet) for _ in range(random.randint(0, 24)))
    if raqeem.normalize_ar(s) != py(s):
        fuzz_bad.append(s)
    r = raqeem.normalize_ar(s)
    if raqeem.normalize_ar(r) != r:
        fuzz_bad.append(("not idempotent", s))
print("multi-char fuzz:", fuzz_bad[:5] or "no divergence")
```
