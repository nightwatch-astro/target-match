# target-match

Pure-Rust, dependency-light target identification for astrophotography: given a
telescope pointing and field of view, rank which catalogued sky objects fall on the
frame — by sky position, never by name.

## Agent Guidance

- **Coordinates only, never names.** Matching is by sky position exclusively. A
  frame's `OBJECT`/`OBJCTNAME` string is inconsistent capture-software metadata and
  MUST NOT be a search key. A designation/name may appear on a result for display,
  but it never influences the match. Do not add name-based matching to the matcher.
- **Catalog-agnostic — no catalogue data lives here.** The crate performs no I/O and
  bundles no catalogue. Consumers supply objects via the `SkyObject` trait. SIMBAD /
  seed / cache concerns belong in a separate resolver crate (`simbad-resolver`), not
  here. If a one-call "resolve + rank" convenience is wanted, it lives in that
  resolver behind an optional feature that depends on `target-match` — never the
  reverse.
- **Keep it pure and dependency-light.** No runtime dependencies beyond `std`;
  `serde` is optional and off by default. Prefer `std` over adding a crate. Keep it
  MSVC-safe and `no_std`-friendly where practical.
- **De-hardcoded geometry.** Field-of-view math must be binning-aware and
  axis-independent (separate x/y pixel size and x/y pixels), and accept the field via
  optics, a pixel scale, or a direct FOV — not the single-square-pixel assumption the
  original `alm` code made. Radius is a policy (circumscribed / inscribed / multiplier
  / explicit), not a hardcoded half-diagonal.
- **Determinism is a contract.** Ranking is ascending by angular separation with
  stable tie-breaking; results MUST be reproducible across runs. Add a test with every
  ranking change.
- **No AI attribution in commits.** A pre-commit `git commit` guard enforces this.

## AGENTS Layering

- This root `AGENTS.md` applies to the whole repository unless a deeper file overrides it.
- Put repo-wide workflow, architecture, tool, and source-of-truth guidance here.
- Add nested `AGENTS.md` files only for subtrees that need materially different rules.

## Codex Project Settings

- Project and subfolder Codex overrides live in `.codex/config.toml`.
- MCP servers for this repo or subtree should be declared under `mcp_servers.<name>` in `.codex/config.toml`.
- Keep repo-specific Codex settings here and leave user-global defaults in `~/.codex/config.toml`.

## Architecture

<!-- BEGIN ps:architecture -->
Single library crate. The public surface lives in `src/lib.rs` across three modules:

- `angle` — angle and equatorial-coordinate primitives: `Angle` (degrees / radians /
  arcminutes / arcseconds / hours), `Equatorial { ra, dec }` (ICRS/J2000), sexagesimal
  parse **and** format (`HH:MM:SS` / `±DD:MM:SS` ⇄ decimal), and `separation` (haversine).
- `optics` — plate scale and field of view from focal length + pixel size (x/y) +
  binning (x/y) + sensor pixels, or a pixel scale, or a direct FOV; plus radius policies.
- `matcher` — the `SkyObject` input trait, match constraints (radius circle,
  rectangular FOV with optional rotation, nearest-N, is-framed), and deterministic
  `rank(...)`.

The module bodies are documented stubs; implementation is spec-driven (see `specs/`).
<!-- END ps:architecture -->

## Path Mapping

| Path | Contents |
|------|----------|
| `src/` | Library source (`angle`, `optics`, `matcher` modules) |
| `specs/` | Feature specifications (SpecKit) |
| `tests/` | Integration tests |
| `docs/` | Documentation (architecture, decisions/ADRs, research) |
| `scripts/` | Build tooling, automation |

## Build & Run

Library crate — no binary. Use the `justfile`:

- `just build` — `cargo build`
- `just test` — `cargo test`
- `just lint` — `cargo clippy --all-targets --all-features -- -D warnings`
- `just fmt` / `just fmt-check` — format / check formatting
- `just verify` — fmt-check + lint + test (the local gate)
- `just doc` — build API docs

## Repo

- **GitHub**: nightwatch-astro/target-match
- **Branch strategy**: feature branches off main, squash merge
