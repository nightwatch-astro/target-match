# Stack Decisions

Recorded during project setup. This crate is intentionally minimal and pure.

## Language & toolchain

- **Rust**, edition **2021**, `rust-version = 1.74`.
- Toolchain pinned to **stable** with `rustfmt` + `clippy` (`rust-toolchain.toml`).
- Task runner: **just** (`justfile`).

## Dependencies

Policy: the crate is **pure Rust with no runtime dependencies** — just `std`. It stays
MSVC-safe and `no_std`-friendly at heart; the work is trigonometry and string parsing.

- **`serde` 1** *(optional, off-by-default `serde` feature)* — `Serialize`/`Deserialize`
  on the public coordinate and match types for JSON and other formats. Not a default, so
  consumers who don't need it pay nothing.
- **`proptest` 1** *(dev-only)* — property-based testing of the geometric invariants
  (separation symmetry, radius-boundary inclusivity, sexagesimal round-trip).

Deliberately **not** added: any astronomy framework (the crate does only the geometry it
needs), any I/O / async / HTTP / database crate (it is catalog-agnostic and does no I/O),
and any crate pulling C/system libraries.

## Scope boundary

- **In scope**: coordinate/angle primitives, FOV/plate-scale geometry, and coordinate-only
  matching/ranking over caller-supplied objects.
- **Out of scope**: catalogue data, name resolution (SIMBAD/Sesame), plate-solving
  (deriving coordinates from pixels), and any persistence. Name resolution belongs in the
  sibling `simbad-resolver` crate; a "resolve + rank" convenience, if wanted, lives there
  behind an optional feature depending on `target-match`.

## Licensing

- **Apache-2.0** (`LICENSE`, `Cargo.toml` `license = "Apache-2.0"`).
- Matches the sibling extracted crates (`fits-header`, `xisf-header`). Revisit to dual
  `MIT OR Apache-2.0` before the first crates.io publish if broader compatibility is wanted.

## Quality gates

- `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `cargo doc` — locally via
  `just verify` and in GitHub Actions CI (Linux, Windows, macOS).
- `pre-commit`: hygiene hooks + `cargo fmt`/`cargo clippy` + `gitleaks` secret scan on push.

## Agentic tooling

- Managed with **APM** (`apm.yml` / `apm.lock.yaml`). Installed packages: `language-rust`,
  `lsp-rust` (rust-analyzer), `speckit` + `steering-speckit`, `hooks-attribution-guard`,
  `steering-git-workflow`, `release-please`.
- **SpecKit** scaffolded under `.specify/` with `/speckit-*` commands.
