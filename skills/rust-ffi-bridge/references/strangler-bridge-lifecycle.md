# Strangler-Fig Bridge Lifecycle

Incremental rewrites preserve continuous shipping by swapping subsystems piece-by-piece behind a bridge. The danger is stalling halfway in a permanent hybrid state.

## Phase 1: Establish the Seam
- Identify natural module boundaries with narrow, serializable or handle-based interfaces (parsers, planners, codecs, protocol engines, storage layers).
- Avoid chatty per-object FFI calls inside tight loops; cross the boundary at batch or request granularity.
- Route unported paths to the legacy implementation behind the same contract.

## Phase 2: Burn Down Legacy Call Sites
Track two numbers on every PR/batch:
- `legacy_call_sites_remaining`
- `bridge_exports_active`

Never add new legacy callers to a bridged interface. Flip callers outward from leaf utilities toward top-level orchestration.

## Phase 3: Shadow & Promote
Before retiring a legacy path:
1. Run shadow mode (legacy answers live traffic; Rust runs in parallel and logs any diff).
2. Require zero mismatches across recorded truth cases and stress benchmarks.
3. Flip primary execution to Rust while keeping legacy unreachable from hot paths.

## Phase 4: Lift & Delete the Bridge
Once a subsystem is 100% Rust:
1. Remove the temporary FFI export table, N-API/PyO3/C glue, and fallback branches in one clean move.
2. Collapse redundant serialization/copy steps that only existed for the bridge.
3. Remove dual-build glue (e.g., mixed CMake/Cargo bridge targets) so builds return to pure Cargo.
4. Re-measure startup latency and memory RSS to confirm the runtime/process boundary tax is gone.
