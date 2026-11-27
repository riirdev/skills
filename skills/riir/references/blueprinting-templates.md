# Phase 0 Blueprinting Templates

Use `riir-skills init-ledger <dir> --mode <mode> --apply` to generate starter files, then tailor them using these rules.

## 1. `PORTING.md` Template (Translation Rulings)

Every construct appearing repeatedly in the source language gets **one** canonical Rust ruling. Add a new numbered ruling (`P-01`, `P-02`, ...) whenever a new pattern is encountered; never allow two conflicting idioms for the same construct.

```markdown
# PORTING.md — Source-to-Rust Translation Rulings

## Baseline
- Reference revision: `<git-sha-or-version>` (read-only)
- Target Rust edition / toolchain: `2024` / `<rustc-version>`
- Migration route: `<Route A | Route B | Route C | Route D>`

## P-01: Primitive Types & Integer Overflow
- Mapping table: `int` -> `c_int`, `size_t` -> `usize`, JS `number` -> `f64` (or checked `i64` where integer-exact)
- Overflow policy: `<wrapping_* | checked_* | saturating_*>`

## P-02: Structs, Enums, Unions, Bitfields
- ABI structs use `#[repr(C)]` with compile-time `size_of` / `offset_of!` assertions.
- Bitfields map to fixed-width integer storage units + inline accessor methods.

## P-03: Strings & Encodings
- Internal representation: `<UTF-8 | WTF-8 | UTF-16 | raw byte buffer>`
- Boundary conversion rules (invalid sequences, NUL bytes, normalization).

## P-04: Error Handling & Non-Local Control Flow
- Source error returns -> `<Result<T, DomainError> | exact errno/status>`
- Cleanup / `defer` / `goto cleanup` -> `<RAII Drop guard | explicit state loop>`

## P-05: Concurrency & Global State
- Global mutable state policy: `<single owned Context struct | Atomic* | RwLock>` (no new `static mut` unless explicitly approved in Phase 1).

## P-06: Forbidden Patterns
- No `todo!()`, `unimplemented!()`, empty stubs, or silent error fallback.
- No `.unwrap()` where source handled an error or null state.
```

## 2. `LIFETIMES.tsv` Schema

Tab-separated ledger tracking ownership, deallocation, and boundary constraints across all critical structs, globals, and handles:

```tsv
file	kind	container	field_or_symbol	source_type	ownership	freed_by	thread_affinity	crosses_nonlocal_jump	layout_frozen	note
src/session.c	struct_field	Session	buf	char*	OWNED	free_session	single_thread	no	yes	Convert to Box<[u8]> / Vec<u8> in Phase 2
src/table.c	global	-	GLOBAL_INTERN_TAB	Table*	STATIC	never	mutex_guarded	no	no	Wrap in OnceLock<Mutex<Table>>
```

### Pointer Ownership Taxonomy (11 Classes)

Classify every pointer/nullable field in `LIFETIMES.tsv` using this fixed taxonomy before writing Rust structs:

| Class | Rust Representation | Source Evidence |
| --- | --- | --- |
| `OWNED` | `Box<T>` / `Option<Box<T>>` / `Vec<T>` | Struct allocates it in init and frees it in teardown |
| `SHARED` | `Rc<T>` / `Arc<T>` / intrusive refcount | Multiple owners; teardown decrements/releases refcount |
| `BORROW_PARAM` | `&'a T` | Assigned from constructor/init parameter outliving `self` |
| `BORROW_FIELD` | Sibling-field borrow (flag for index/handle refactor) | Points into a sibling field's owned allocation on `self` |
| `STATIC` | `&'static T` | Assigned from a process-global or singleton slot |
| `CALLER_BORROW` | Passed per-call (`&Context` / `&VM`), omitted from struct | Caller-owned execution context always live on the call stack |
| `BACKREF` | `*const Parent` / `Weak<Parent>` / index | Points back to the struct that owns `self` |
| `INTRUSIVE` | `*mut Node` | Linked-list link pointer recovering parent via offset math |
| `FFI` | `*mut T` / `*const T` | Crosses `extern "C"` boundary or comes from a `*_sys` crate |
| `ARENA` | Arena pointer / typed index handle | Freed wholesale by arena reset, never individually |
| `UNKNOWN` | `Option<NonNull<T>>` + tracking note | Cannot prove from one file alone; escalate to multi-vote review |

Any `UNKNOWN` or cyclic classification triggers focused review before implementation rather than silent raw-pointer drift.

## 3. Risk Spike Report Format

Before scaling past 3 files, resolve top technical unknowns in isolated spikes:

```markdown
## Spike: <Name>
- Hypothesis / Unknown: What could invalidate the migration plan?
- Reproducible Check: Exact command or test executed.
- Measured Result: Observed behavior (e.g., 35/35 struct layout assertions match; unwinding across boundary leaks Drop).
- Binding Ruling: What `PORTING.md` rule this spike establishes.
```

## 4. 3-File Pilot Scorecard

Select 3 small representative units before full fan-out:
1. **Pure Algorithm**: math, hashing, parser helper, or codec.
2. **Stateful Data Structure + API Surface**: collection, cache, or command handler registering public symbols.
3. **Configuration / Lookup Table**: struct-heavy constants, flags, or platform table.

Pass criteria before unlocking Batch 1:
- All 3 compile without stubs or warnings.
- Inventory/symbol check reports 0 missing and 0 invented symbols.
- Differential cases pass 100% against reference.
- Every surprise discovered during the pilot is codified into `PORTING.md`.

## 5. Crate DAG Cycle-Break & Source-Leakage Audit

### Cycle-Break Actions (Before Wiring Workspace Crates)

For every circular dependency edge found between planned crates, assign one of five explicit resolutions:

| Action | Meaning |
| --- | --- |
| `DELETE` | Dead symbol or unused legacy binding; drop instead of forwarding |
| `TYPE_ONLY` | Only the data layout/struct/enum is needed; extract to a leaf `*-types` crate |
| `MOVE_DOWN` | Move a mislayered helper down into a shared lower-tier crate |
| `FORWARD_DECL` | Break runtime mutual recursion behind an explicit trait, callback table, or narrow interface |
| `GENUINE` | True architectural coupling requiring two modules to stay in one crate |

### Source-Language Leakage Audit Checklist for `PORTING.md`

Before scaling workers, inspect `PORTING.md` for mechanical carryovers Rust makes unnecessary:
- **Allocator parameters**: Drop pervasive allocator args when `#[global_allocator]` suffices; keep explicit allocator params only for per-request arenas, FFI handoffs, or stored-for-later allocators.
- **Custom containers vs `std`**: Keep custom collections only for true layout/growth/deterministic-hash contracts (FFI-ABI lists, SoA structs, deterministic fast hashes); replace trivial wrappers with `Vec<T>` / `VecDeque<T>`.
- **Manual `deinit()` & cleanups**: Replace manual cleanup methods and boilerplate guards with `Drop` and `Clone` unless cleanup consumes external context/handles or frees arena-allocated items.
