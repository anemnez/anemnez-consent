use super::*;

fn p(s: &str) -> CanonPath {
    CanonPath::parse(s).expect("a canonical path")
}

fn fs_write() -> Capability {
    Capability {
        namespace: "fs".into(),
        name: "write".into(),
    }
}

fn seat() -> Capability {
    Capability {
        namespace: "profile".into(),
        name: "set_default_seat".into(),
    }
}

fn agent(id: &str) -> Subject {
    Subject {
        class: SubjectClass::SubstrateAgent,
        id: id.into(),
        lane: Some("C".into()),
    }
}

fn every_agent() -> SubjectMatch {
    SubjectMatch {
        class: SubjectClass::SubstrateAgent,
        id: None,
    }
}

fn person(n: &str) -> Origin {
    Origin::Person(AnswerRef(n.into()))
}

fn grant(
    subject: SubjectMatch,
    cap: Capability,
    scope: Scope,
    effect: Permission,
    origin: Origin,
) -> Grant {
    Grant {
        subject,
        capability: cap,
        scope,
        effect,
        origin,
        lifetime: Lifetime::Standing,
    }
}

fn places() -> Places {
    Places {
        home: Some(p("/Users/jimmie")),
        projects: vec![
            p("/Users/jimmie/personal/anemnez"),
            p("/Users/jimmie/work/ybor"),
        ],
        cwd: Some(Cwd {
            path: p("/Users/jimmie/scratch/adhoc"),
            proven: true,
        }),
    }
}

const NOW: Timestamp = 1_000_000;

fn decide_with(
    subject: &Subject,
    cap: &Capability,
    target: &Target,
    guard: Guard,
    grants: &[Grant],
    places: &Places,
) -> Verdict {
    decide(&Input {
        subject,
        capability: cap,
        target,
        declared: Some(Permission::Ask),
        guard,
        grants,
        answering: None,
        places,
        session: Some("s1"),
        task: None,
        now: NOW,
        skew_ms: 30_000,
        attended: true,
        can_park: true,
    })
}

/// The User's model, 2026-11-06, as the spec's worked example: scope agents by risk.
fn the_users_rules() -> Vec<Grant> {
    vec![
        // "freely code in my projects"
        grant(
            every_agent(),
            fs_write(),
            Scope::Project,
            Permission::Allow,
            person("projects"),
        ),
        // "this agent may modify its current directory, an ad hoc project"
        grant(
            SubjectMatch {
                class: SubjectClass::SubstrateAgent,
                id: Some("wren".into()),
            },
            fs_write(),
            Scope::Cwd,
            Permission::Allow,
            person("cwd"),
        ),
        // "ask when writing in my Home"
        grant(
            every_agent(),
            fs_write(),
            Scope::Home,
            Permission::Ask,
            person("home"),
        ),
        // "no agents operating outside my home"
        grant(
            every_agent(),
            fs_write(),
            Scope::System,
            Permission::Deny,
            person("system"),
        ),
    ]
}

#[test]
fn the_users_worked_example_scopes_agents_by_risk() {
    let rules = the_users_rules();
    let at = |who: &Subject, path: &str| {
        decide_with(
            who,
            &fs_write(),
            &Target::Path(p(path)),
            Guard::Work,
            &rules,
            &places(),
        )
    };
    let wren = agent("wren");
    let other = agent("tron");
    assert_eq!(
        at(
            &other,
            "/Users/jimmie/personal/anemnez/substrate/src/lib.rs"
        ),
        Verdict::Allow(Admitted::Grant(person("projects"))),
        "code in a project: go"
    );
    assert_eq!(
        at(&wren, "/Users/jimmie/scratch/adhoc/notes.md"),
        Verdict::Allow(Admitted::Grant(person("cwd"))),
        "wren's own ad hoc directory"
    );
    assert_eq!(
        at(&other, "/Users/jimmie/scratch/adhoc/notes.md"),
        Verdict::Ask(AskKind::WorkGuard),
        "another agent there is only in home: ask"
    );
    assert_eq!(
        at(&wren, "/Users/jimmie/Documents/resume.pages"),
        Verdict::Ask(AskKind::WorkGuard),
        "my résumé: I want to know"
    );
    assert_eq!(
        at(&wren, "/etc/passwd"),
        Verdict::Deny(DenyWhy::Stated(person("system"))),
        "outside home, across the system: no thanks"
    );
}

#[test]
fn the_decision_never_depends_on_the_order_of_the_grants() {
    let rules = the_users_rules();
    let targets = [
        "/Users/jimmie/personal/anemnez/a",
        "/Users/jimmie/scratch/adhoc/b",
        "/Users/jimmie/c",
        "/etc/d",
    ];
    let mut order: Vec<usize> = (0..rules.len()).collect();
    let expected: Vec<Verdict> = targets
        .iter()
        .map(|t| {
            decide_with(
                &agent("wren"),
                &fs_write(),
                &Target::Path(p(t)),
                Guard::Work,
                &rules,
                &places(),
            )
        })
        .collect();
    // Every permutation of four (Heap's algorithm, iteratively).
    let mut c = vec![0; order.len()];
    let mut i = 0;
    while i < order.len() {
        if c[i] < i {
            if i % 2 == 0 {
                order.swap(0, i);
            } else {
                order.swap(c[i], i);
            }
            let shuffled: Vec<Grant> = order.iter().map(|&k| rules[k].clone()).collect();
            for (t, want) in targets.iter().zip(&expected) {
                let got = decide_with(
                    &agent("wren"),
                    &fs_write(),
                    &Target::Path(p(t)),
                    Guard::Work,
                    &shuffled,
                    &places(),
                );
                assert_eq!(&got, want, "{t} under order {order:?}");
            }
            c[i] += 1;
            i = 0;
        } else {
            c[i] = 0;
            i += 1;
        }
    }
}

#[test]
fn at_equal_specificity_a_deny_wins() {
    let rules = vec![
        grant(
            every_agent(),
            fs_write(),
            Scope::Home,
            Permission::Allow,
            person("yes"),
        ),
        grant(
            every_agent(),
            fs_write(),
            Scope::Home,
            Permission::Deny,
            person("no"),
        ),
    ];
    let v = decide_with(
        &agent("wren"),
        &fs_write(),
        &Target::Path(p("/Users/jimmie/x")),
        Guard::Work,
        &rules,
        &places(),
    );
    assert_eq!(v, Verdict::Deny(DenyWhy::Stated(person("no"))));
}

#[test]
fn a_floor_deny_is_never_lifted_by_a_narrower_allow() {
    let rules = vec![
        grant(
            every_agent(),
            fs_write(),
            Scope::System,
            Permission::Deny,
            Origin::Floor,
        ),
        grant(
            every_agent(),
            fs_write(),
            Scope::Home,
            Permission::Allow,
            person("narrow"),
        ),
        grant(
            every_agent(),
            fs_write(),
            Scope::Cwd,
            Permission::Allow,
            Origin::Host {
                why: "owned dir".into(),
            },
        ),
    ];
    let v = decide_with(
        &agent("wren"),
        &fs_write(),
        &Target::Path(p("/Users/jimmie/x")),
        Guard::Work,
        &rules,
        &places(),
    );
    assert_eq!(v, Verdict::Deny(DenyWhy::Floor));
}

#[test]
fn a_grant_for_another_class_admits_nothing() {
    let rules = vec![grant(
        SubjectMatch {
            class: SubjectClass::External,
            id: None,
        },
        fs_write(),
        Scope::System,
        Permission::Allow,
        person("external"),
    )];
    let v = decide_with(
        &agent("wren"),
        &fs_write(),
        &Target::Path(p("/tmp/x")),
        Guard::Work,
        &rules,
        &places(),
    );
    assert_eq!(
        v,
        Verdict::Ask(AskKind::WorkGuard),
        "falls to the declaration"
    );
}

/// The User, 2026-11-06: the person's act is "an 'Are you sure?'-style question, less of an
/// authorization and more of a confirmation". No standing, session, task or host Grant admits
/// it; only the person's short confirmation within the descriptor's bound, or this call's answer.
#[test]
fn the_persons_act_is_a_confirmation_no_standing_grant_admits() {
    let target = Target::None;
    let five_min = 5 * 60 * 1000;
    let guard = Guard::PersonAct { max_ms: five_min };
    let standing = grant(
        every_agent(),
        seat(),
        Scope::System,
        Permission::Allow,
        person("always"),
    );
    let host = grant(
        every_agent(),
        seat(),
        Scope::System,
        Permission::Allow,
        Origin::Host { why: "x".into() },
    );
    let session = Grant {
        lifetime: Lifetime::Session("s1".into()),
        ..standing.clone()
    };
    for g in [standing.clone(), host, session] {
        let v = decide_with(
            &agent("wren"),
            &seat(),
            &target,
            guard,
            std::slice::from_ref(&g),
            &places(),
        );
        assert_eq!(
            v,
            Verdict::Ask(AskKind::PersonAct),
            "{g:?} must not admit it"
        );
    }
    let short = Grant {
        lifetime: Lifetime::Until(NOW + 60_000),
        ..standing.clone()
    };
    assert_eq!(
        decide_with(
            &agent("wren"),
            &seat(),
            &target,
            guard,
            std::slice::from_ref(&short),
            &places()
        ),
        Verdict::Allow(Admitted::Grant(person("always"))),
        "a confirmation for a minute, inside the five the descriptor allows"
    );
    let too_long = Grant {
        lifetime: Lifetime::Until(NOW + five_min + 1),
        ..standing
    };
    assert_eq!(
        decide_with(
            &agent("wren"),
            &seat(),
            &target,
            guard,
            &[too_long],
            &places()
        ),
        Verdict::Ask(AskKind::PersonAct),
        "longer than the descriptor's bound"
    );
    assert_eq!(
        decide_with(
            &agent("wren"),
            &seat(),
            &target,
            Guard::PersonAct { max_ms: 0 },
            &[short],
            &places()
        ),
        Verdict::Ask(AskKind::PersonAct),
        "a bound of 0 means once, every time"
    );
}

#[test]
fn an_unattended_ask_refuses_only_when_the_asker_cannot_park() {
    let target = Target::Path(p("/Users/jimmie/x"));
    let mut input = Input {
        subject: &agent("wren"),
        capability: &fs_write(),
        target: &target,
        declared: Some(Permission::Ask),
        guard: Guard::Work,
        grants: &[],
        answering: None,
        places: &places(),
        session: None,
        task: None,
        now: NOW,
        skew_ms: 0,
        attended: false,
        can_park: true,
    };
    assert_eq!(decide(&input), Verdict::Ask(AskKind::WorkGuard), "it parks");
    input.can_park = false;
    assert_eq!(decide(&input), Verdict::Deny(DenyWhy::Unattended));
}

#[test]
fn an_until_grant_expires_after_its_time_plus_skew() {
    let g = |until| Grant {
        lifetime: Lifetime::Until(until),
        ..grant(
            every_agent(),
            fs_write(),
            Scope::System,
            Permission::Allow,
            person("for:8h"),
        )
    };
    let t = Target::Path(p("/tmp/x"));
    let within = decide_with(
        &agent("wren"),
        &fs_write(),
        &t,
        Guard::Work,
        &[g(NOW - 20_000)],
        &places(),
    );
    assert!(matches!(within, Verdict::Allow(_)), "inside the 30 s skew");
    let past = decide_with(
        &agent("wren"),
        &fs_write(),
        &t,
        Guard::Work,
        &[g(NOW - 40_000)],
        &places(),
    );
    assert_eq!(past, Verdict::Ask(AskKind::WorkGuard));
}

#[test]
fn a_session_grant_from_another_session_admits_nothing() {
    let g = Grant {
        lifetime: Lifetime::Session("other".into()),
        ..grant(
            every_agent(),
            fs_write(),
            Scope::System,
            Permission::Allow,
            person("s"),
        )
    };
    let v = decide_with(
        &agent("wren"),
        &fs_write(),
        &Target::Path(p("/tmp/x")),
        Guard::Work,
        &[g],
        &places(),
    );
    assert_eq!(v, Verdict::Ask(AskKind::WorkGuard));
}

#[test]
fn nothing_registered_is_refused_above_everything() {
    let t = Target::None;
    let v = decide(&Input {
        subject: &agent("wren"),
        capability: &fs_write(),
        target: &t,
        declared: None,
        guard: Guard::Work,
        grants: &[grant(
            every_agent(),
            fs_write(),
            Scope::System,
            Permission::Allow,
            person("x"),
        )],
        answering: None,
        places: &places(),
        session: None,
        task: None,
        now: NOW,
        skew_ms: 0,
        attended: true,
        can_park: true,
    });
    assert_eq!(v, Verdict::Deny(DenyWhy::Unregistered));
}

#[test]
fn a_shown_denied_capability_refuses_and_never_asks() {
    let rules = vec![grant(
        every_agent(),
        fs_write(),
        Scope::System,
        Permission::Deny,
        person("no"),
    )];
    let me = agent("wren");
    assert_eq!(
        presence(
            &me,
            Visibility::Hidden,
            &[(every_agent(), Visibility::Minimal)]
        ),
        Visibility::Minimal,
        "shown, though denied"
    );
    let v = decide_with(
        &me,
        &fs_write(),
        &Target::Path(p("/tmp/x")),
        Guard::Work,
        &rules,
        &places(),
    );
    assert_eq!(v, Verdict::Deny(DenyWhy::Stated(person("no"))));
}

#[test]
fn a_lane_never_matches_and_a_path_never_matches_by_string() {
    let mut wren = agent("wren");
    wren.lane = Some("ANYTHING".into());
    let g = grant(
        every_agent(),
        fs_write(),
        Scope::Cwd,
        Permission::Allow,
        person("beck"),
    );
    let beck = Places {
        cwd: Some(Cwd {
            path: p("/tmp/beck"),
            proven: true,
        }),
        ..places()
    };
    let v = decide_with(
        &wren,
        &fs_write(),
        &Target::Path(p("/tmp/becky/x")),
        Guard::Work,
        &[g],
        &beck,
    );
    assert_eq!(
        v,
        Verdict::Ask(AskKind::WorkGuard),
        "/tmp/beck does not cover /tmp/becky"
    );
}

/// The deepest containing root wins: a cwd inside a project beats the project, either way round.
#[test]
fn a_cwd_inside_a_project_beats_the_project() {
    let inside = Places {
        cwd: Some(Cwd {
            path: p("/Users/jimmie/personal/anemnez/scratch"),
            proven: true,
        }),
        ..places()
    };
    let t = Target::Path(p("/Users/jimmie/personal/anemnez/scratch/x"));
    let rules = |project, cwd| {
        vec![
            grant(
                every_agent(),
                fs_write(),
                Scope::Project,
                project,
                person("project"),
            ),
            grant(every_agent(), fs_write(), Scope::Cwd, cwd, person("cwd")),
        ]
    };
    let v = decide_with(
        &agent("wren"),
        &fs_write(),
        &t,
        Guard::Work,
        &rules(Permission::Allow, Permission::Deny),
        &inside,
    );
    assert_eq!(v, Verdict::Deny(DenyWhy::Stated(person("cwd"))));
    let v = decide_with(
        &agent("wren"),
        &fs_write(),
        &t,
        Guard::Work,
        &rules(Permission::Deny, Permission::Allow),
        &inside,
    );
    assert_eq!(v, Verdict::Allow(Admitted::Grant(person("cwd"))));
}

/// An outside caller's claimed cwd matches nothing until its place is proven (IRIS.md Q6): it
/// gets no narrower than home.
#[test]
fn an_unproven_cwd_matches_nothing() {
    let claimed = Places {
        cwd: Some(Cwd {
            path: p("/Users/jimmie/scratch/adhoc"),
            proven: false,
        }),
        ..places()
    };
    let rules = the_users_rules();
    let v = decide_with(
        &agent("wren"),
        &fs_write(),
        &Target::Path(p("/Users/jimmie/scratch/adhoc/notes.md")),
        Guard::Work,
        &rules,
        &claimed,
    );
    assert_eq!(v, Verdict::Ask(AskKind::WorkGuard), "only home covers it");
}
