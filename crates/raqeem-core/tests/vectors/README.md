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
gap was in a range nobody had thought to test. Two divergences from the reference were
found by sweeping *every* plausible codepoint and comparing the two implementations
directly, not by choosing cases:

- the `_FORMAT_STRIP` set the port was missing
- U+001C–001F, which Python's `\s` collapses and `char::is_whitespace` does not

If you change either implementation, re-run that sweep rather than trusting this file.
It needs both scout and a built binding on the path, which is why it isn't a CI job:

```
python -c "
import sys; sys.path.insert(0, 'E:/work')
from scout.arabic import normalize_ar as py
import raqeem
bad = [hex(c) for r in [(0,0x2FF),(0x600,0x6FF),(0x2000,0x206F),(0xFE70,0xFEFF)]
       for c in range(r[0], r[1]+1)
       if not (0xD800 <= c < 0xE000)
       for probe in [chr(c), f'  a {chr(c)}  b  ', f'{chr(c)} طماطة {chr(c)}']
       if raqeem.normalize_ar(probe) != py(probe)]
print(sorted(set(bad)) or 'no divergence')
"
```
