# TypeSafe Rust SDK

Rust SDK for the [TypeSafe](https://typesafe.ai) API.

Ask questions about a piece of text and get answers with probabilities attached,
instead of a string you have to parse.

Want the command line rather than the library? That is
[douglance/jevon](https://github.com/douglance/jevon) — `cargo install jevon`.

## Does this fit your problem?

It fits when your program already has a decision to make, the options are known
in advance, and judgment is the hard part. It does not generate anything.

The test is mechanical: **if you can write the `match` arms before the call, it
fits. If you can't, it doesn't.**

[When jev fits](https://github.com/douglance/jevon/blob/main/docs/when-to-use.md)
covers the situations it is actually used for and, more usefully, the ones where
it is the wrong tool.

## Quickstart

```sh
cargo add typesafe-sdk-client typesafe-sdk-config typesafe-sdk-env \
          typesafe-sdk-http typesafe-sdk-questions
export TYPESAFE_API_KEY=...
```

A client is a configuration plus a transport. Nothing reads the environment
behind your back — `Process` is the thing that does it, and you pass it in:

```rust
use std::sync::Arc;

use typesafe_sdk_client::{Client, SystemOneRequest};
use typesafe_sdk_config::Builder;
use typesafe_sdk_env::Process;
use typesafe_sdk_http::Reqwest;
use typesafe_sdk_questions::{choice_of, questions};

let config = Builder::new().build(&Process)?;
let client = Client::with_transport(config, Arc::new(Reqwest::new()?));

let request = SystemOneRequest::new(
    "I was charged twice. Please fix this ASAP.",
    questions([("category", choice_of(
        "What is this ticket about?",
        ["billing", "technical", "other"],
    ))]),
);
let response = client.system_one(request).await?;
```

`Builder` takes each setting directly too, for callers whose configuration comes
from somewhere other than the environment:

```rust
let config = Builder::new()
    .api_key(secret_from_your_vault)
    .default_model("jev-latest")
    .build(&Process)?;
```

Hold one `Client` and reuse it. A fresh client per call pays for a TLS handshake
every time, which is most of the latency at small payloads.

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

This is the payoff of the precondition above: adding a label to the struct
breaks every `match` that does not handle it, at compile time rather than in
production.

## Which crates do I need?

Thirteen are published, but most callers name four or five. The split exists so
that a crate needing only the wire vocabulary does not drag in a TLS stack.

| Crate | Take it when |
|---|---|
| `typesafe-sdk-client` | Always. The façade you call. |
| `typesafe-sdk-config` | Always. Builds the configuration. |
| `typesafe-sdk-http` | Always, unless you supply your own `Transport`. |
| `typesafe-sdk-env` | Reading configuration from the environment. |
| `typesafe-sdk-questions` | Writing question sets by hand. |
| `typesafe-sdk-derive` | Deriving them from a struct instead. |
| `typesafe-sdk-answers` | Naming answer types explicitly. |
| `typesafe-sdk-error` | Matching on the error taxonomy. |
| `typesafe-sdk-models` | Listing the models an account can use. |
| `typesafe-sdk-retry` | Overriding the retry policy. |
| `typesafe-sdk-log` | Supplying a logger. |
| `typesafe-sdk-headers`, `typesafe-sdk-runtime` | Transitive. You rarely name them. |

## Testing without a network

`Transport` is a trait, and `typesafe-sdk-http` ships a `Mock` that answers from
a script. Tests against it run in microseconds and can assert what actually went
on the wire, including how many attempts were made:

```rust
use typesafe_sdk_http::{Exchange, Mock};

let transport = Arc::new(Mock::new(vec![Exchange::ok(r#"{"answers":{}}"#)]));
let client = Client::with_transport(config, transport.clone());

// ... call the client, then:
assert_eq!(transport.attempts(), 1);
```

This seam is what makes retry and timeout behaviour testable at all, rather than
hoped for.

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
the headers. `RequestOptions` overrides any of this for a single call.

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

The command crates and the `jev` binary used to sit above layer 4. They now live
in [douglance/jevon](https://github.com/douglance/jevon) and depend on this SDK
from crates.io, which means the CLI is held to the API that ships rather than to
whatever happens to be in this tree.

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
