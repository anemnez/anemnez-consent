//! anemnez-consent: Iris's decision engine (IRIS.md §4.3, step 1).
//!
//! Who may do what, decided purely from what the person said. [`decide`] takes every fact as
//! input, including the time, and does no I/O, reads no clock and checks no signature: a host
//! hands in Grants it has already verified. There is no permissive posture of any kind (User,
//! 2026-11-06: "once we have Iris, we no longer need Permissive anything").
//!
//! The rules the types hold:
//! - **Two kinds of Ask.** A guard on work ([`Guard::Work`]) is answered with a scope and a
//!   lifetime. The person's act ([`Guard::PersonAct`], [`Guard::PersonOnly`]) is a confirmation,
//!   "are you sure?", answered once or for a short bounded time. A standing grant never admits
//!   it, and neither does a host's.
//! - **A stated "no" from a role or the host is a floor** ([`Origin::Floor`]): nothing lifts it.
//! - **Among the answers that cover a call, the deepest containing root wins**, and a tie goes
//!   to Deny. So "Allow in my projects" carves out of "Ask in my home", which carves out of
//!   "Deny across the system" (the four path scopes, [`Scope`]).
//! - **A Grant for another class of subject admits nothing**, and a lane never matches: it names
//!   a caller, never authorizes one.
//! - **Visibility is a separate question** ([`presence`]); a shown, denied capability is refused
//!   and never asks.

pub mod scope;

pub use scope::{CanonPath, Cwd, PathError, Places, Scope, Target};

/// Milliseconds since the Unix epoch. The host's clock, passed in.
pub type Timestamp = u64;

/// What a capability is: a namespaced verb (`fs.write`, `profile.set_default_seat`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Capability {
    /// Its namespace (`fs`).
    pub namespace: String,
    /// Its verb (`write`).
    pub name: String,
}

/// Which kind of caller a subject is. Grants key on it, so a grant to one class never admits
/// another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubjectClass {
    /// An agent living in a Substrate.
    SubstrateAgent,
    /// A caller from outside, such as a harness over MCP. Never the person.
    External,
    /// A product acting as itself.
    Product,
}

/// Who is asking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subject {
    /// Its class.
    pub class: SubjectClass,
    /// Its stable id within the class.
    pub id: String,
    /// The lane it posts on, for attribution only: no Grant ever matches on it.
    pub lane: Option<String>,
}

/// Which subjects a Grant covers. There is deliberately no lane field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectMatch {
    /// The class it covers; a subject of any other class is never covered.
    pub class: SubjectClass,
    /// One subject, or every subject of the class.
    pub id: Option<String>,
}

impl SubjectMatch {
    fn covers(&self, subject: &Subject) -> bool {
        self.class == subject.class && self.id.as_ref().is_none_or(|id| *id == subject.id)
    }
}

/// An answer, three ways.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Permission {
    /// Go ahead.
    Allow,
    /// Ask the person.
    Ask,
    /// No.
    Deny,
}

/// What a capability declares about who must admit it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Guard {
    /// A piece of work: a covering Grant admits it, or its declaration decides.
    Work,
    /// The person's act, an "are you sure?". Admitted only by the person's own answer: this
    /// call's answer, or a person's Grant that runs out within `max_ms` of now. `max_ms` of 0
    /// means once, every time.
    PersonAct {
        /// The longest a person's Grant may run and still admit it.
        max_ms: u64,
    },
    /// The person's alone, once, every time (enrolling a device, an agent's own rows).
    PersonOnly,
}

/// A reference to the signed answer a Grant came from, opaque to the engine.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AnswerRef(pub String);

/// Where a Grant came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// The person answered, from an enrolled device.
    Person(AnswerRef),
    /// A role's or the host's own rule. A Floor Deny is never lifted.
    Floor,
    /// Computed by the host for this call (a directory the agent owns). It admits work only.
    Host {
        /// Why the host minted it.
        why: String,
    },
}

/// How long a Grant lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lifetime {
    /// Until the person changes it.
    Standing,
    /// For one session.
    Session(String),
    /// For one task; it ends with the task.
    Task(String),
    /// Until this time (plus the host's clock skew).
    Until(Timestamp),
}

/// A standing answer, verified by the host before it arrives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grant {
    /// Whom it covers.
    pub subject: SubjectMatch,
    /// What capability.
    pub capability: Capability,
    /// What it covers of that capability.
    pub scope: Scope,
    /// What it says.
    pub effect: Permission,
    /// Who said it.
    pub origin: Origin,
    /// How long it lives.
    pub lifetime: Lifetime,
}

/// The person's answer to the very Ask this call is retrying: once, never a stored row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answering {
    /// The answer.
    pub permission: Permission,
    /// The signed answer it is.
    pub answer: AnswerRef,
}

/// Every fact a decision is made from.
#[derive(Debug, Clone)]
pub struct Input<'a> {
    /// Who asks.
    pub subject: &'a Subject,
    /// What capability.
    pub capability: &'a Capability,
    /// What it acts on.
    pub target: &'a Target,
    /// The capability's own declaration; `None` when nothing by that name exists here.
    pub declared: Option<Permission>,
    /// Who must admit it.
    pub guard: Guard,
    /// Every Grant that might apply, in any order.
    pub grants: &'a [Grant],
    /// The person's answer to this call's own Ask, if it is being retried with one.
    pub answering: Option<&'a Answering>,
    /// Where the person's places are.
    pub places: &'a Places,
    /// The session and task the call runs in, for Session and Task Grants.
    pub session: Option<&'a str>,
    /// See `session`.
    pub task: Option<&'a str>,
    /// Now, by the host's clock.
    pub now: Timestamp,
    /// How far the host trusts clocks to disagree (`PRINCIPAL_TOKEN`'s `SKEW`).
    pub skew_ms: u64,
    /// Whether any surface can show the person a card right now.
    pub attended: bool,
    /// Whether the asker can go on and pick the answer up later (Q4: minds are never blocked).
    pub can_park: bool,
}

/// Why an Ask is raised.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AskKind {
    /// The person's act: a confirmation card, "are you sure?".
    PersonAct,
    /// A guard on work: a grant card, with scopes and lifetimes.
    WorkGuard,
}

/// Why something was admitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Admitted {
    /// Its declaration allows it.
    Declared,
    /// A Grant covers it.
    Grant(Origin),
    /// The person answered this very Ask.
    Answer(AnswerRef),
}

/// Why something was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DenyWhy {
    /// Nothing by that name exists here.
    Unregistered,
    /// A role's or the host's Deny: never lifted.
    Floor,
    /// The person said no.
    Stated(Origin),
    /// Its declaration denies it.
    Declared,
    /// It would ask, but no surface can show a card and the asker cannot park
    /// (`E_ASK_UNATTENDED`).
    Unattended,
}

/// The decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Go ahead.
    Allow(Admitted),
    /// No.
    Deny(DenyWhy),
    /// Ask the person.
    Ask(AskKind),
}

impl Grant {
    fn live(&self, input: &Input<'_>) -> bool {
        match &self.lifetime {
            Lifetime::Standing => true,
            Lifetime::Session(s) => input.session == Some(s.as_str()),
            Lifetime::Task(t) => input.task == Some(t.as_str()),
            Lifetime::Until(t) => input.now <= t.saturating_add(input.skew_ms),
        }
    }

    /// The depth at which it covers this call, when it applies at all.
    fn applies(&self, input: &Input<'_>) -> Option<usize> {
        if !(self.subject.covers(input.subject)
            && self.capability == *input.capability
            && self.live(input))
        {
            return None;
        }
        self.scope.covering_depth(input.target, input.places)
    }
}

/// Decide one call. Pure: the same input always gives the same verdict, whatever order the
/// Grants come in.
#[must_use]
pub fn decide(input: &Input<'_>) -> Verdict {
    let Some(declared) = input.declared else {
        return Verdict::Deny(DenyWhy::Unregistered);
    };
    let live: Vec<(&Grant, usize)> = input
        .grants
        .iter()
        .filter_map(|g| g.applies(input).map(|depth| (g, depth)))
        .collect();
    if live
        .iter()
        .any(|(g, _)| g.origin == Origin::Floor && g.effect == Permission::Deny)
    {
        return Verdict::Deny(DenyWhy::Floor);
    }
    let verdict = match input.guard {
        Guard::PersonOnly => answered(input).unwrap_or(Verdict::Ask(AskKind::PersonAct)),
        Guard::PersonAct { max_ms } => answered(input)
            .or_else(|| persons_confirmation(input, &live, max_ms))
            .unwrap_or(Verdict::Ask(AskKind::PersonAct)),
        Guard::Work => answered(input)
            .or_else(|| most_specific(&live))
            .unwrap_or(match declared {
                Permission::Allow => Verdict::Allow(Admitted::Declared),
                Permission::Ask => Verdict::Ask(AskKind::WorkGuard),
                Permission::Deny => Verdict::Deny(DenyWhy::Declared),
            }),
    };
    match verdict {
        Verdict::Ask(_) if !input.attended && !input.can_park => Verdict::Deny(DenyWhy::Unattended),
        other => other,
    }
}

/// The person's answer to this very Ask, when the call carries one.
fn answered(input: &Input<'_>) -> Option<Verdict> {
    let a = input.answering?;
    Some(match a.permission {
        Permission::Allow => Verdict::Allow(Admitted::Answer(a.answer.clone())),
        Permission::Deny => Verdict::Deny(DenyWhy::Stated(Origin::Person(a.answer.clone()))),
        Permission::Ask => Verdict::Ask(AskKind::PersonAct),
    })
}

/// A person's act admitted by a short confirmation the person gave: only a Person-origin Grant
/// running out within `max_ms` of now. A standing, session or task Grant never admits it, and
/// neither does a host's. A person's Deny covering it refuses it.
fn persons_confirmation(
    input: &Input<'_>,
    live: &[(&Grant, usize)],
    max_ms: u64,
) -> Option<Verdict> {
    let persons = live
        .iter()
        .map(|(g, _)| *g)
        .filter(|g| matches!(g.origin, Origin::Person(_)));
    if let Some(no) = persons.clone().find(|g| g.effect == Permission::Deny) {
        return Some(Verdict::Deny(DenyWhy::Stated(no.origin.clone())));
    }
    persons
        .filter(|g| g.effect == Permission::Allow)
        .find(|g| matches!(g.lifetime, Lifetime::Until(t) if max_ms > 0 && t <= input.now.saturating_add(max_ms)))
        .map(|g| Verdict::Allow(Admitted::Grant(g.origin.clone())))
}

/// The answer of the Grant whose containing root is deepest; at equal depth Deny beats Ask beats
/// Allow.
fn most_specific(live: &[(&Grant, usize)]) -> Option<Verdict> {
    let severity = |p: Permission| match p {
        Permission::Allow => 0u8,
        Permission::Ask => 1,
        Permission::Deny => 2,
    };
    let (best, _) = live
        .iter()
        .max_by_key(|(g, depth)| (*depth, severity(g.effect)))?;
    Some(match best.effect {
        Permission::Allow => Verdict::Allow(Admitted::Grant(best.origin.clone())),
        Permission::Ask => Verdict::Ask(AskKind::WorkGuard),
        Permission::Deny => Verdict::Deny(DenyWhy::Stated(best.origin.clone())),
    })
}

/// How much of a capability a subject sees, independent of whether it may call it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// Not listed at all.
    Hidden,
    /// Listed with its full description.
    Full,
    /// Listed by its summary.
    Minimal,
    /// Listed by name only.
    Index,
}

/// What `subject` sees of a capability: its own row's visibility when one names it, else the
/// capability's default. Permission plays no part (the visible-but-denied ruling): a shown,
/// denied capability is listed, and calling it gets [`DenyWhy`] at once, never an Ask.
#[must_use]
pub fn presence(
    subject: &Subject,
    default: Visibility,
    rows: &[(SubjectMatch, Visibility)],
) -> Visibility {
    rows.iter()
        .filter(|(m, _)| m.covers(subject))
        .max_by_key(|(m, _)| m.id.is_some())
        .map_or(default, |(_, v)| *v)
}

#[cfg(test)]
mod tests;
