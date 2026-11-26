# The 12 Dimensions of High-Fidelity Parity

Use this checklist for mechanical or ABI-exact ports (Route A) and strict boundary audits.

| Dim | Name | Definition | Verification Gate |
| --- | --- | --- | --- |
| **D1** | File & Tree | Deterministic 1:1 mapping between source files and Rust modules | Path mapping generator check: 0 missing, 0 extra |
| **D2** | Statement Order | Definitions and statement sequence mirror source; deviations documented | Automated line/statement ratio audit; explain >10% or >20-line skew |
| **D3** | Functions & Linkage | Every source function has a matching Rust symbol with identical internal vs exported linkage | Symbol inventory diff empty; `nm -g` exported symbol check |
| **D4** | Exported / Registered API | Every registered command, opcode, or public entry matches name, arity, flags, and doc metadata | Dump runtime API table from both binaries and diff |
| **D5** | Macros, Constants & Enums | Compile-time constants, flags, and enum discriminants match verbatim | Inventory diff + `const` equality assertions |
| **D6** | Struct & Memory Layout | `#[repr(C)]` structs/unions/bitfields match field order, size, alignment, and tagged pointers | Bidirectional C `sizeof`/`offsetof` vs Rust `size_of`/`offset_of!` assertions |
| **D7** | Call Graph & Layering | Functions call the same architectural layers (no bypassing subsystems or collapsing layers silently) | Callee-set extraction diff; document intentional deviations |
| **D8** | External Interfaces & CLI | Headers, plugin/module ABI structs, and CLI flags/help/exit codes match | Header symbol diff + ABI offset check + `--help`/`--version` diff |
| **D9** | Control Flow & Error Model | Error codes, signal recovery, and cleanup ordering match (no silent `?` conversion altering semantics) | Fault-injection differential tests + reviewer control-flow check |
| **D10** | Memory, GC & Lifetimes | Custom allocators, pinning/rooting, weak refs, finalizers, and stack scanning behave identically | Object counts before/after collection; stress test under tight allocation thresholds |
| **D11** | Strings & Encodings | Internal encodings, codepoint tables, char classes, and line endings match | Round-trip UTF-8/UTF-16/legacy-encoding & malformed byte sequence diff |
| **D12** | Binary & Generated Artifacts | Compiled bytecode, snapshots/dumps, plugins, and generated tables are mutually interchangeable | Bidirectional load test (Ref artifact -> Rust; Rust artifact -> Ref) + byte-level output diff |
