--- The deputed unit-test leg: `prova run ut` conducts cargo nextest ONCE for the whole workspace —
--- compilation and execution both live in a file-scoped fixture that emits one junit artifact —
--- then one proof adopts every case into the account, and sibling readers can bind claims to
--- named cases at zero additional compilations (conduct once, read many).
---
--- The `ut` switch (suite.lua) is intent; `cargo-nextest` stays a `requires` world fact: two facts
--- with two remedies. The profile `must_run`s the deputy, so `prova run ut` fails rather than
--- skips when nextest is missing — a profile is a contract, not a courtesy.
---
--- A workspace with ZERO unit tests reads RED here, deliberately: `prova run ut` asked for the
--- unit-test account and there is none to adopt. On a legacy project that has never written unit
--- tests, that red is the leg telling you where to start — not a gate to weaken.

-- Conduct the deputy once. The stale artifact is removed FIRST, so a deputy that dies before
-- emitting (a compile error) leaves nothing behind and the adoption fails loudly on "matched
-- nothing" — never a previous run's verdicts wearing this run's face. The deputy's exit code is
-- deliberately not asserted here: the adopting proof reports red with the deputed cases' own
-- names, which a fixture death would hide.
-- The `prova` profile (junit emission, fail-fast off) ships in .prova/nextest.toml and reaches
-- nextest as TOOL config — composing under, never colliding with, the project's own
-- .config/nextest.toml. A repo-defined [profile.prova] would win, deliberately.
local deputy = prova.fixture("nextest-junit", Scope.File, function()
	local artifact = prova.root .. "/target/nextest/prova/junit.xml"
	fs.remove_all(artifact)
	shell.run(
		{ "cargo", "nextest", "run", "--workspace", "--profile", "prova",
			"--tool-config-file", "prova:" .. prova.root .. "/.prova/nextest.toml" },
		{ cwd = prova.root, merge_stderr = true, timeout = "1800s" }
	)
	return artifact
end)

prova.test("the workspace's unit-test account holds — every nextest case adopted", {
	requires = { "cargo-nextest" },
	-- The house rule, framework-enforced (prova learn locks): cargo takes process-wide locks
	-- of its own, so every conduct that compiles declares the writer hold — bound across every
	-- prova instance at this home, so two agents and a human never race a build.
	locks = { prova.writes("cargo") },
}, function(t)
	junit.verify(t, { results = t:use(deputy) })
end)

-- Readers: one claim, one named unit test — a prose claim discharged by a specific case in the
-- deputy's own account, without a second run. As claims accrete, bind them like this:
--
--   prova.test("the parser rejects unterminated input, spoken for by its unit test", {
--   	requires = { "cargo-nextest" },
--   }, function(t)
--   	local report = junit.load(t:use(deputy))
--   	for _, c in ipairs(report.cases) do
--   		if c.name == "parser::tests::rejects_unterminated_input" then
--   			t:expect(c.outcome, c.name):equals("passed")
--   			return
--   		end
--   	end
--   	t:expect(false, "parser::tests::rejects_unterminated_input is missing from the account"):is_true()
--   end)

-- docs/specs/CONSENT.md: each claim the unit tests discharge, by the named cases that speak for it. A
-- case missing from the account is red, never a skip, so a renamed test cannot orphan its claim.
local function spoken_for(claim, cases)
	prova.test("claim " .. claim .. " — spoken for by its unit tests", {
		requires = { "cargo-nextest" },
		covers = "docs/specs/CONSENT.md#" .. claim,
	}, function(t)
		local report = junit.load(t:use(deputy))
		local seen = {}
		for _, c in ipairs(report.cases) do
			for _, wanted in ipairs(cases) do
				if c.name == wanted or c.name:sub(-#wanted - 2) == "::" .. wanted then
					seen[wanted] = true
					t:expect(c.outcome, c.name):equals("passed")
				end
			end
		end
		for _, wanted in ipairs(cases) do
			t:expect(seen[wanted] == true, wanted .. " is missing from the account"):is_true()
		end
	end)
end

spoken_for("decide-is-pure", { "tests::the_decision_never_depends_on_the_order_of_the_grants" })
spoken_for("the-persons-act-is-a-confirmation", { "tests::the_persons_act_is_a_confirmation_no_standing_grant_admits" })
spoken_for("the-deepest-root-wins", {
	"tests::the_users_worked_example_scopes_agents_by_risk",
	"tests::a_cwd_inside_a_project_beats_the_project",
	"tests::at_equal_specificity_a_deny_wins",
})
spoken_for("a-floor-is-never-lifted", { "tests::a_floor_deny_is_never_lifted_by_a_narrower_allow" })
spoken_for("an-unproven-place-authorizes-nothing", {
	"tests::a_grant_for_another_class_admits_nothing",
	"tests::a_lane_never_matches_and_a_path_never_matches_by_string",
	"tests::an_unproven_cwd_matches_nothing",
})
spoken_for("minds-are-never-blocked", { "tests::an_unattended_ask_refuses_only_when_the_asker_cannot_park" })
spoken_for("a-denied-call-never-raises-an-ask", { "tests::a_shown_denied_capability_refuses_and_never_asks" })
spoken_for("a-grant-lives-as-long-as-it-says", {
	"tests::an_until_grant_expires_after_its_time_plus_skew",
	"tests::a_session_grant_from_another_session_admits_nothing",
	"tests::nothing_registered_is_refused_above_everything",
})
