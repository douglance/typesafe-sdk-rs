# Changelog

## v0.6.2 (2026-09-19)

### Changed

- The README is a library README now. It shows how to build a `Client` — which
  it never did, despite calling one in the first example — says which of the
  thirteen crates a caller actually needs, documents the `Mock` transport for
  testing without a network, and states the precondition for the tool being a
  fit at all. Both code samples are compiled against the crates before release.

## v0.6.1 (2026-09-19)

### Changed

- Metadata and README now describe a library rather than a library plus a CLI.
  No API changed; this exists so the published pages stop documenting commands
  that live in another repository.

## Unreleased

### Removed

- The command crates and the `jev` binary, which moved to
  [douglance/jevon](https://github.com/douglance/jevon) and now consume this
  SDK from crates.io. `cargo install jevon` is unchanged; this repository is
  the library only.

### Changed

- Reimplemented in Rust as a workspace of small crates, with a `typesafe`
  command-line interface built on [incurs](https://github.com/douglance/incurs).
  The HTTP contract, question vocabulary, retry policy, error taxonomy and
  configuration precedence are unchanged; the TypeScript implementation is
  preserved at the `ts-final` tag.

### Added

- `#[derive(Questions)]`, which turns a struct into a question set and gives
  each choice question an enum of its labels, so a `match` over the outcomes is
  exhaustive.
- A `typesafe` binary serving the same commands over the terminal, MCP, shell
  completions and agent skill files.
- Per-call overrides for the timeout, retry policy and headers, and access to
  the request id on success as well as on failure.

## v0.6.0 (2026-09-15)

### Breaking changes

- accept `Score.criteria` as an ordered sequence instead of a dictionary keyed by integers

## v0.5.7 (2026-09-11)

This is the initial public release of TypeSafe JavaScript and TypeScript SDK.
Learn more in the [documentation](https://docs.typesafe.ai/).
