//! The matching engine: input trait, constraints, and ranking.
//!
//! Planned surface (pending the SpecKit spec under `specs/`):
//!
//! - `SkyObject` — the input trait a caller's catalogue type implements
//!   (position accessors only); `target-match` reads position and never a name.
//! - Match constraints: within a radius (circle), within a rectangular field of
//!   view (optionally rotated by a camera position angle), nearest-N, is-framed.
//! - `rank(...)` — deterministic ranking by ascending angular separation, with
//!   stable tie-breaking so ordering is reproducible across runs.
//!
//! Matching never inspects a name or designation — see the crate-level docs for
//! why (the coordinates-only rule).
//!
//! Depends only on [`crate::angle`] and [`crate::optics`].

// TODO(spec): implement per the target-match specification under `specs/`.
