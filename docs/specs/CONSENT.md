# anemnez-consent: the decision engine's claims

The engine Iris embeds (IRIS.md §4.3). The contract is owned by Iris (I); C builds it, B reviews.
Each claim below is spoken for by named unit tests (`proofs/ut/nextest.prova.lua`).

<!-- claim: decide-is-pure -->
`decide` is a pure function of its input: the same input gives the same verdict, whatever order
the Grants arrive in. Time is an input; nothing reads a clock, a file or a signature.

<!-- claim: the-persons-act-is-a-confirmation -->
The person's act is an "are you sure?" (User, 2026-11-06). Only this call's own answer admits it,
or a person's confirmation that runs out within the bound its descriptor states (0: every time).
A standing, session, task or host Grant never admits it.

<!-- claim: the-deepest-root-wins -->
Among the Grants that cover a call, the one whose containing root is deepest wins, across the
four path scopes `project`, `cwd`, `home` and `system` (User, 2026-11-06). At equal depth Deny
beats Ask beats Allow.

<!-- claim: a-floor-is-never-lifted -->
A role's or the host's Deny (`Origin::Floor`, or a `Origin::Host` grant that denies) refuses
whatever narrower answer exists.

<!-- claim: an-unproven-place-authorizes-nothing -->
A Grant for another class of subject admits nothing. A lane never matches: it names a caller and
never authorizes one. The subject's place is kernel-read by the host or Agora, never claimed (User,
2026-11-06, Q6). When it could not be proven, the `cwd` and `project` rungs do not apply, so the
caller gets no narrower than home. Paths match component-wise, never by string.

<!-- claim: minds-are-never-blocked -->
An Ask is a normal outcome that an asker may park on (User, 2026-11-06). It is refused at once
(`E_ASK_UNATTENDED`) only when no surface can show a card and the asker cannot park.

<!-- claim: a-denied-call-never-raises-an-ask -->
Visibility and permission are independent: a capability may be shown and denied, and calling it
refuses at once, never asking. A denied capability is hidden unless a row naming the subject shows
it (the host passes `Hidden` as the default when `decide` would deny).

<!-- claim: an-ask-keeps-its-guards-kind -->
An Ask is raised as its guard's kind of card: a confirmation for the person's act, a grant card
(scopes and lifetimes) for a work guard, also when the person's answer was itself "ask me again".

<!-- claim: a-grant-lives-as-long-as-it-says -->
A Grant applies only while its lifetime holds: its session, its task, or until its time plus the
host's clock skew. Nothing by an unregistered name is admitted, above every Grant.
