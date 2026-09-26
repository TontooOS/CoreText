# Ffi

C ABI (`src/ffi.rs`, `Headers/coretext.h`): version and measure for
the SDK dynamic binding.

| Function | Return | Meaning |
|---|---|---|
| `coretext_version` | `const char *` | Borrowed version string; do not free |
| `coretext_measure` | `int` | `1` on success, `0` on null input |
| `coretext_string_free` | `void` | Frees a heap string returned by CoreText |

```c
int coretext_measure(const char *text, float size, float scale, float *out_w, float *out_h);
```

- Measures `text` at `size` logical px (regular weight) on a
  `scale` display; writes logical width/height to `out_w`/`out_h`.
- The SDK wraps this as `sdk::CoreText::measure` when the full
  crate is not linked (`features = ["CoreText"]` links the full
  crate instead).

```rust
sdk::preinclude!();
let (w, h) = CoreText::measure("Hello", 17.0, 2.0)?;
```

## Cross References

- [Typeset.md](Typeset.md) – full Rust measure API
- [MAIN.md](MAIN.md) – library overview
