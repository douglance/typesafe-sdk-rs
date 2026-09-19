# TypeSafe Rust SDK

Rust SDK for the [TypeSafe](https://typesafe.ai) API.

Ask questions about a piece of text and get answers with probabilities attached,
instead of a string you have to parse.

The command-line interface lives in its own repository,
[douglance/jevon](https://github.com/douglance/jevon), and consumes this SDK
from crates.io:

```sh
cargo install jevon
```

## Quickstart

```sh
cargo add typesafe-sdk-client typesafe-sdk-questions
export TYPESAFE_API_KEY=...
```

```rust
use typesafe_sdk_client::{Client, SystemOneRequest};
use typesafe_sdk_questions::{choice_of, questions};

let request = SystemOneRequest::new(
    "I was charged twice. Please fix this ASAP.",
    questions([("category", choice_of(
        "What is this ticket about?",
        ["billing", "technical", "other"],
    ))]),
);
let response = client.system_one(request).await?;
```

## Typed questions

Declaring a question set generates an enum of its labels, so a `match` over the
outcomes is exhaustive and a mistyped label is a compile error:

```rust
use typesafe_sdk_derive::Questions;

#[derive(Questions)]
struct Ticket {
    /// What is this ticket about?
    #[choice(billing, technical, other)]
    category: (),
    /// How urgent is it?
    #[score("can wait", "this week", "today")]
    urgency: (),
}

let response = client.system_one(SystemOneRequest::new(text, Ticket::questions())).await?;
match Ticket::answers(&response)?.category {
    TicketCategory::Billing => route_to_finance(),
    TicketCategory::Technical => route_to_engineering(),
    TicketCategory::Other => triage(),
}
```

## Configuration

| Variable | Default | Purpose |
|---|---|---|
| `TYPESAFE_API_KEY` | — | Required. |
| `TYPESAFE_BASE_URL` | `https://api.typesafe.ai` | API root. |
| `TYPESAFE_DEFAULT_MODEL` | `jev-latest` | Model used when a request names none. |
| `TYPESAFE_LOG_LEVEL` | `warn` | `debug`, `info`, `warn`, `error`, `off`. |

An explicit argument wins over the environment, which wins over the default. A
variable set to whitespace counts as unset. `jev doctor`, from the CLI, shows
what resolved and what is missing.

Requests retry twice by default, on 408, 429 and 5xx as well as connection
failures and timeouts, with exponential backoff from 500ms to a 5s cap. A
server's `Retry-After` is honoured exactly when it asks for a minute or less.
The 10s timeout applies per attempt and covers the full response body, not just
the headers.

## Agents

Agent surfaces — MCP, shell completions and skill files — belong to the CLI.
See [douglance/jevon](https://github.com/douglance/jevon).

## Layout

Crates are layered, and a crate may depend only on its own layer or below. The
table lives in `xtask/src/layers.rs`, and a crate missing from it fails the
build.

```
0  error · env · runtime · headers          pure values, no I/O
1  questions · answers · models · retry · log
2  derive                                   the proc macro
3  http · config
4  client                                   the SDK
5  lint · lsp                               tools built on it
6  tests                                    cross-crate conformance
```

The command crates and the `jev` binary used to sit above layer 4. They now
live in [douglance/jevon](https://github.com/douglance/jevon) and depend on
this SDK from crates.io, which means the CLI is held to the API that ships
rather than to whatever happens to be in this tree.

## Development

```sh
cargo fmt --all --check
cargo xtask check      # file and function size, complexity, layering
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

All four must pass. `cargo xtask check` is the authority on structure: review
does not reliably notice that a file crossed 150 lines or that a leaf crate
acquired a socket. Files are capped at 150 lines and functions at 30, with
cognitive complexity 7 — an escape hatch exists but must name the metric and
give a reason:

```rust
// typesafe-allow file-length reason: the layer map reads best as one table.
```

Tests that talk to the real API run only when `TYPESAFE_API_KEY` is set and skip
cleanly otherwise, so an ordinary `cargo test` stays offline and deterministic.
Run them before releasing: they are the only check that proves the SDK talks to
TypeSafe rather than to its own assumptions.

## History

This repository began as the official TypeScript SDK and was reimplemented in
Rust. The TypeScript implementation is preserved at the `ts-final` tag, and
upstream remains at [typesafe-ai/typesafe-sdk-js](https://github.com/typesafe-ai/typesafe-sdk-js).

## License

MIT
