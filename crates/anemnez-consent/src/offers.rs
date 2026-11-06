//! What a card offers the person, narrowest first.
//!
//! A work guard's card is a grant card: a scope from the four path scopes (or an argv prefix
//! for a command), and a lifetime. A person's act is a confirmation card: "are you sure?",
//! answered once, plus "for N minutes" only where the descriptor's bound allows it (User,
//! 2026-11-06). The narrowest choice comes first and is the default; nothing wider is ever
//! pre-selected.

use crate::scope::{Scope, Target};
use crate::{AskKind, Guard, Input, Permission, Timestamp};

/// The timed lifetimes a card offers when the host names none: 15 minutes, an hour, 8 hours.
/// The User, 2026-11-06: "ask in Home, and I may allow it for a few minutes". The set is the
/// host's to tune, so the person can change it later.
pub const DEFAULT_TIMED_MS: [u64; 3] = [15 * 60 * 1000, 60 * 60 * 1000, 8 * 60 * 60 * 1000];

/// A lifetime a card offers, before the person picks it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OfferedLifetime {
    /// This call only. Never stored as a Grant.
    Once,
    /// For the task the call runs in.
    Task(String),
    /// For the session the call runs in.
    Session(String),
    /// Until this time.
    Until(Timestamp),
}

/// What a card offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Offers {
    /// A grant card for a work guard: Allow or Deny, at the scopes that cover the call (deepest
    /// root first), for the lifetimes (once, then the timed ones shortest first, then the task
    /// and the session).
    Grant {
        /// Allow first, the default; Deny at any of the same scopes ("never in ~/.ssh").
        answers: Vec<Permission>,
        /// Narrowest first; the first is the default.
        scopes: Vec<Scope>,
        /// Once first, the default.
        lifetimes: Vec<OfferedLifetime>,
    },
    /// A confirmation for the person's act: Confirm or Cancel, and, where the descriptor allows,
    /// "for this long".
    Confirm {
        /// The longest "for N" the card may offer; `None` means once only.
        for_until: Option<Timestamp>,
    },
}

/// The card for an Ask of `kind` on `input`, with `timed_ms` the timed lifetimes the host offers
/// ([`DEFAULT_TIMED_MS`] when it names none).
#[must_use]
pub fn offers(input: &Input<'_>, kind: AskKind, timed_ms: &[u64]) -> Offers {
    match (kind, input.guard) {
        (AskKind::PersonAct, Guard::PersonAct { max_ms }) => Offers::Confirm {
            for_until: (max_ms > 0).then(|| input.now.saturating_add(max_ms)),
        },
        (AskKind::PersonAct, Guard::PersonOnly | Guard::Work) => {
            Offers::Confirm { for_until: None }
        }
        (AskKind::WorkGuard, _) => Offers::Grant {
            answers: vec![Permission::Allow, Permission::Deny],
            scopes: covering_scopes(input),
            lifetimes: lifetimes(input, timed_ms),
        },
    }
}

/// Every scope that covers the call, deepest containing root first. At equal depth (one root
/// under two names, say a cwd that is a project root) only the more particular is offered: the
/// subject's cwd, then a project, then home.
fn covering_scopes(input: &Input<'_>) -> Vec<Scope> {
    let mut candidates: Vec<Scope> = match input.target {
        Target::Command(argv) => (1..=argv.len())
            .rev()
            .map(|n| Scope::Command(argv[..n].to_vec()))
            .collect(),
        Target::Path(_) => vec![Scope::Cwd, Scope::Project, Scope::Home],
        Target::None => Vec::new(),
    };
    candidates.push(Scope::System);
    let mut covering: Vec<(usize, usize, Scope)> = candidates
        .into_iter()
        .enumerate()
        .filter_map(|(order, s)| {
            s.covering_depth(input.target, input.places)
                .map(|depth| (depth, order, s))
        })
        .collect();
    covering.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut out: Vec<Scope> = Vec::new();
    let mut last_depth = None;
    for (depth, _, scope) in covering {
        // Two scopes that both contain the target at the same depth share one root, so they grant
        // the same thing: offer only the more particular one.
        if last_depth == Some(depth) {
            continue;
        }
        last_depth = Some(depth);
        out.push(scope);
    }
    out
}

/// The lifetimes: once, the timed ones shortest first, then the task and the session.
fn lifetimes(input: &Input<'_>, timed_ms: &[u64]) -> Vec<OfferedLifetime> {
    let mut timed: Vec<u64> = timed_ms.iter().copied().filter(|ms| *ms > 0).collect();
    timed.sort_unstable();
    timed.dedup();
    let mut out = vec![OfferedLifetime::Once];
    out.extend(
        timed
            .into_iter()
            .map(|ms| OfferedLifetime::Until(input.now.saturating_add(ms))),
    );
    if let Some(task) = input.task {
        out.push(OfferedLifetime::Task(task.to_string()));
    }
    if let Some(session) = input.session {
        out.push(OfferedLifetime::Session(session.to_string()));
    }
    out
}
