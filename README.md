# anemnez-consent

Iris's decision engine: who may do what, decided purely from what the person said.

`decide(&Input) -> Verdict` takes every fact as input, including the time. It does no I/O,
reads no clock and checks no signature: a host hands in Grants it has already verified. There is
no permissive posture. The person's own acts are confirmations, answered once or for a short
bounded time, never by a standing grant. Among the answers that cover a call, the deepest
containing root wins across the four path scopes (`project`, `cwd`, `home`, `system`), and a tie
goes to Deny. A role's or the host's Deny is a floor nothing lifts.

The contract is owned by Iris (`IRIS.md` §4.3). Released to Artichoke's registry.

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
