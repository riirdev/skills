# Differential Harness, Property Testing & Fault Injection

## 1. JSONL Truth-Set Format (`cases.jsonl`)

Each line in `cases.jsonl` is one self-contained deterministic test case compatible with `riir-skills diff-run`:

```json
{"id": "neg_modulo", "args": ["eval", "-7 % 3"], "stdin": ""}
{"id": "utf8_boundary", "args": ["slice", "1", "3"], "stdin": "a🦀b\n"}
{"id": "bad_magic_header", "args": ["parse", "-"], "stdin": "\u0000\u0001bad"}
```

Supported fields:
- `id` (string): Stable identifier for diff reporting.
- `args` (array of strings): Arguments passed cleanly to the executable.
- `stdin` (string): Exact standard input bytes.
- `env` (object, optional): Explicit per-case environment variables.

## 2. Deterministic Normalization Rules

Only normalize fields that are inherently non-deterministic across runs (and document every normalization rule explicitly):
- Process IDs, temporary directory paths, or timestamps -> scrub via explicit regex/placeholder before byte comparison.
- Never normalize away **exit codes**, **error type prefixes**, **floating-point sign (`-0.0` vs `0.0`)**, or **trailing newline differences** unless the specification explicitly declares them irrelevant.

## 3. Property-Based Testing (`proptest`)

Pair differential execution with invariant property tests on parsers, codecs, and state machines:
- **Roundtrip**: `decode(encode(x)) == Ok(x)` for arbitrary valid structures.
- **No-Panic Fuzz Invariant**: `decode(arbitrary_bytes)` never panics — returns `Ok` or `Err`.
- **Differential Oracle**: Inside a property test, invoke the reference implementation (or pure reference model) on generated inputs and assert identical structured output.

## 4. Coverage-Guided Fuzzing (`cargo-fuzz`)

Target every untrusted boundary:
- File/wire format parsers and deserializers
- String/charset/regex decoders
- FFI input validators

Keep minimized crash inputs in `tests/fixtures/` as permanent regression cases.

## 5. Fault Injection Matrix

Verify failure paths explicitly on both implementations:
- Truncated and bit-flipped inputs at header, length-prefix, and payload boundaries
- `EINTR` / transient IO interruption and retry bounds
- Resource exhaustion (memory limit, file descriptor limit, disk-full write error)
- Signal delivery (`SIGINT`, `SIGTERM`, `SIGPIPE`) during active processing
- Resource-bounded execution with explicit timeouts so deadlocks fail fast instead of stalling CI
