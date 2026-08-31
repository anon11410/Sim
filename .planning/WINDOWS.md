---
schema_version: 1
open_count: 20
waived_count: 0
fixed_count: 4
total_count: 24
last_updated: 2026-08-31T11:49:41.130Z
---

# Broken Windows Ledger

> Cross-phase defect register. With `workflow.windows_enforce` enabled, `/gsd-ship` blocks while `open_count > 0`.
> Waive with `gsd-tools windows waive <id> "<reason>"` (reason required).
> Mark fixed with `gsd-tools windows fixed <id>`.

| id | phase | kind | file | line | description | status | reason | recorded_at | resolved_at |
|----|-------|------|------|------|-------------|--------|--------|-------------|-------------|
| 1 | 01 | deviation | .planning/REQUIREMENTS.md |  | Tasks 1 and 4 assert grep -c '^- \\[ \\] \\*\\*' == 87 as a no-rows-lost tripwire; it reports 84 because plan 01-01 checked off CORE-02/08/09. Total rows (^- \\[[ x]\\] \\*\\*) is 87. Criterion baseline is stale, not a file defect. | open |  | 2026-08-30T23:17:58.131Z |  |
| 2 | 01 | deviation | config/baseline.toml |  | V-4: theta=0.75 sense contradicts the key name price_inaction_prob_ppm; flagged in config/PROVENANCE.md for the Phase 6 gate, not corrected from memory (D-20) | open |  | 2026-08-31T00:14:39.114Z |  |
| 3 | 02 | deviation | .planning/phases/02-books-journal-and-invariants/02-02-PLAN.md |  | Task 2 verify command 'cargo test --locked --lib books invariants' is not valid cargo syntax (one positional TESTNAME only); use 'cargo test --locked --lib -- books invariants'. Plans 02-03..02-07 must not copy the broken form. | open |  | 2026-08-31T09:22:48.072Z |  |
| 4 | 02 | stub | src/books.rs | 118 | Posting::units_out/units_in/goods_residual_units are zero on every posting this phase produces; plan 02-03 gives them values and 02-04 adds the goods-conservation check that reads them | fixed |  | 2026-08-31T09:22:48.317Z | 2026-08-31T09:38:20.748Z |
| 5 | 02 | deviation | .planning/phases/02-books-journal-and-invariants/02-03-PLAN.md |  | Task 1 acceptance criterion "grep -c 'pub fn produce' src/books.rs prints 1" is unsatisfiable: the pattern is a substring of 'pub fn produced', the accessor the same plan mandates in <artifacts_produced>. It prints 2. Anchored form 'pub fn produce\\(' prints 1. Same for consume/consumed. | open |  | 2026-08-31T09:38:20.990Z |  |
| 6 | 02 | deviation | src/invariants.rs |  | Violation carries Option<Box<Posting>> rather than Option<Posting>: a second posting-bearing variant pushed the enum past clippy::result_large_err's 128-byte threshold, which under -D warnings refuses to compile every Result in the crate that propagates a violation. Plans 02-04/02-05 construct Violations and must box the posting. | open |  | 2026-08-31T09:38:29.928Z |  |
| 7 | 02 | deviation | tests/invariant_halt.rs |  | Modified outside plan 02-03's files_modified: its active-check sequence assertion is [MoneyConservation, Liveness] and inserting goods conservation at position two made it fail. Plan 02-04 inserts two more checks and must update the same two assertions (here and src/invariants.rs#the_gate_decides_the_exact_sequence_of_active_checks). | open |  | 2026-08-31T09:38:30.201Z |  |
| 8 | 02 | deviation | src/books.rs |  | Plan 02-04 task 2 modified src/books.rs though its <files> lists only src/invariants.rs: check_non_negative cannot walk accounts without an enumerator (Books::accounts) and no accessor exposed the per-slot firm generations needed to build a firm Account. Also added PostError::EmptyExchange. | open |  | 2026-08-31T09:55:20.419Z |  |
| 9 | 02 | deviation | tests/invariant_halt.rs |  | Modified outside plan 02-04's files_modified, discharging ledger entry 7: the active-check sequence assertions now read five checks with the gate on and four with it off. Behavioural claims untouched. Plans 02-05..02-07 that add a check must update the same two assertions (here and src/invariants.rs#the_gate_decides_the_exact_sequence_of_active_checks). | open |  | 2026-08-31T09:55:20.666Z |  |
| 10 | 02 | deviation | src/invariants.rs |  | ZeroSumDetail ships 8 variants, not the 6 in plan 02-04's <artifacts_produced>: SplitParties (a one-party kind naming two accounts) and EmptyExchange (the action text's 'both legs non-zero' clause, which the six listed variants cannot express). Plan 02-05's message tests should expect eight. | open |  | 2026-08-31T09:55:20.916Z |  |
| 11 | 02 | deviation | .planning/ROADMAP.md |  | roadmap.update-plan-progress rewrote the Phase 2 plan checklist as a side effect; reverted per the wave shared-artifact rule (STATE.md and ROADMAP.md are owned by the orchestrator while sibling plans 02-06/02-07 are outstanding). Plans 02-06 and 02-07 will hit the same side effect and must revert it too. | open |  | 2026-08-31T10:11:22.368Z |  |
| 12 | 02 | unmet-truth | tests/ledger_props.rs |  | The must_have truth 'ending a tick leaves both running residuals untouched' is asserted by ending_a_tick_leaves_the_residuals_and_the_balances_untouched but cannot fail from an integration test: on the honest path the books conserve, so both residuals are already zero at every boundary. Verified by mutation - adding 'self.cash_residual_cents = 0' to Books::end_of_tick leaves the property green. The version with teeth needs a seeded non-zero residual and therefore needs the pub(crate) corruption vocabulary, which tests/ cannot reach. Plan 02-06 owns the fault-injection unit tests and should add it there. | fixed |  | 2026-08-31T10:22:33.391Z | 2026-08-31T10:43:12.885Z |
| 13 | 02 | deviation | clippy.toml |  | Plan 02-06 task 2 lists nine disallowed-types entries including std::sync::Arc; only eight were added. Arc makes the clean tree fail check 1: proptest's prop_oneof! expands to code naming it, producing 9 diagnostics across 7 call sites in tests/ledger_props.rs and tests/money_props.rs, and check 4b forbids a lint exemption anywhere in tracked Rust source. Same class of finding as the plan's own RefCell exclusion; recorded in the clippy.toml comment and covered by guard 7c (Arc absent from src/). Later phases adding a proptest strategy must not re-add the entry. | open |  | 2026-08-31T10:43:27.921Z |  |
| 14 | 02 | deviation | tests/lints.sh |  | Guards 7e and 7h are scoped to the production half of src/invariants.rs (everything before the first #[cfg(test)] line) and 7e counts the qualified field read '.liveness_enabled' rather than the bare identifier. Written literally as the plan specifies, both fail on the real tree: the unit-test modules legitimately load the shipped config from a path (7h) and set the key on a Params value (7e), and the one production read binds a local that is used again a few lines later to filter the check table (7e). The scoping is stated in the script comments. | open |  | 2026-08-31T10:43:28.153Z |  |
| 15 | 02 | deviation | src/books.rs |  | Modified outside plan 02-06's files_modified, discharging ledger entry 12 at the orchestrator's instruction: mod tests gains ending_a_tick_leaves_a_seeded_non_zero_residual_of_either_kind_untouched, which seeds a non-zero cash and goods residual with the pub(crate) corruption vocabulary before crossing the tick boundary. Mutation-verified in both profiles. No production code changed. | open |  | 2026-08-31T10:43:28.412Z |  |
| 16 | 02 | deviation | src/books.rs |  | Code-review fix CR-01 added PostError::EmptyTransfer and ZeroSumDetail::EmptyTransfer. Books::transfer now refuses Money::ZERO before the self-dealing check, so transfer(a, a, 0) reports EmptyTransfer where it previously reported SelfDealing. Phase 6's partial-payroll and Phase 8's dividend call sites must treat a zero-amount transfer as a refusal rather than an Ok(Money::ZERO): the returned amount is still the amount moved (LEDG-03), but the Err arm is now reachable for an amount a caller computed as zero. | open |  | 2026-08-31T11:25:43.540Z |  |
| 17 | 02 | deviation | src/invariants.rs |  | Code-review fix CR-01/WR-08 grew ZeroSumDetail from 8 variants to 10: EmptyTransfer (a transfer with no cash on either leg, the counterpart of EmptyExchange) and UnitsInTheWrongDirection (a production that also released units, or a consumption that also received them - reported as UnitLegsDiffer before, whose message contradicted the equal numbers it carried). invariants::message::DETAIL_SHAPES is 10; detail_position and every_detail are exhaustive matches, so a later phase adding a variant is named by the compiler. | open |  | 2026-08-31T11:26:08.883Z |  |
| 18 | 02 | deviation | clippy.toml |  | Code-review finding WR-05 asked for std::process::id in disallowed-methods. DECLINED, with the same reasoning clippy.toml already records for RefCell and Arc: tests/config_strict.rs:275 and tests/tracer_end_to_end.rs:21 call it to build a unique temporary path, so the entry makes tests/lints.sh check 1 fail (verified: error: use of a disallowed method std::process::id --> tests/config_strict.rs:275) and check 4b forbids the #[allow(...)] that would silence it. tests/lints.sh guard 7h carries the rule instead, now over the production half of src/books.rs as well as src/invariants.rs. No BANNEDCALL line added, so check 3 still reads 60 against 60. A later phase must not re-add the entry without first moving those two temp-path helpers. | open |  | 2026-08-31T11:26:09.121Z |  |
| 19 | 02 | deviation | tests/lints.sh |  | Code-review fixes WR-03/04/05/06 changed tests/lints.sh: assert_fires and assert_ignores moved above check 4b (they were defined in section 7, below their new first use); check 4b's exemption pattern now matches the banned lint in any argument position and inside cfg_attr, and has assert_fires/assert_ignores proofs; guard 7d gained a second clause over every tracked src/*.rs with line comments stripped, carving out src/rng.rs, so the file Phase 3 adds for the tick loop is guarded on the commit that adds it; guard 7h searches src/books.rs's production half too; guards 7i and 7j are new (every corrupt_* declaration is inside a #[cfg(test)] block, and the cfg-test probe calls every one of them). Section 7 is now ten guards, not eight - the count appears in three echo strings and two comments. | open |  | 2026-08-31T11:26:09.369Z |  |
| 20 | 02 | unmet-truth | tests/ledger_props.rs |  | Code-review finding WR-02: posting_residuals_agree_with_the_balance_derived_quantities is structurally 0 == 0 from an integration test, because record derives both residuals from the same argument the balance write used. Verified by mutation - deriving record's cash residual from total_money() leaves all 8 properties in the file green. DISCHARGED by books::tests::the_two_residual_sources_move_apart_when_only_one_of_them_is_told, which appends postings whose legs disagree while touching no balance and is the only thing that fails under that mutation. The property's doc comment now records what it does and does not prove, and names the unit test. Same shape as ledger entry 12. | fixed |  | 2026-08-31T11:26:09.601Z | 2026-08-31T11:26:22.135Z |
| 21 | 02 | deviation | .planning/ROADMAP.md |  | Code-review finding WR-06 asked for the guard-7d scope extension to be recorded as a ROADMAP success criterion, the way guard 7f's inherited obligation is. NOT WRITTEN: this fix pass is explicitly barred from editing .planning/ROADMAP.md and .planning/STATE.md. The obligation is discharged rather than deferred - guard 7d now searches every tracked src/*.rs with src/rng.rs carved out by name, so a Phase 3 src/world.rs naming debug_assert or cfg(debug_assertions) fails the guard on the commit that adds it, with no promise for a future reader to keep. Watched firing on exactly that shape. | open |  | 2026-08-31T11:26:09.827Z |  |
| 22 | 02 | deviation | src/books.rs |  | Code-review fix WR-01 added BooksError::EndowmentOutOfRange and an up-front checked closed-form endowment gate in Books::new, and WR-07 added Books::goods_residual_units_for(GoodId). Both are new public API in a review-fix pass. The first is required because the money-side gate was a saturating running sum; the second because check_goods read one residual outside its per-good loop. Phase 5 changes goods_residual_units_for's body and check_goods does not move; the GOODS doc comment now lists the four things Phase 5 actually inherits instead of promising that nothing moves. | open |  | 2026-08-31T11:26:10.066Z |  |
| 23 | 02 | unmet-truth | src/invariants.rs | 581 | ROADMAP Phase 2 criterion 2 ('the negative test passes for EVERY check') was met for four of the five checks. check_goods (goods conservation, LEDG-05) had no negative test: every call site reaching it asserted Ok(()), the localisation test called first_breaking_goods_posting directly, and the message module rendered a hand-built GoodsConservation the check never produced. Verified by 02-VERIFICATION mutation M10 - replacing the whole body with 'if true { return Ok(()); }' left all 239 tests green. Plan 02-05 scoped exactly four violation classes (LEDG-04/06/07/10) and goods was never in scope, so the 'every check' self-audit never ran and nothing recorded the omission. | fixed |  | 2026-08-31T11:49:16.631Z | 2026-08-31T11:49:20.831Z |
| 24 | 02 | deviation | src/books.rs |  | Closure of ledger entry 23. Books gains a fifth corruption method, corrupt_silent_stock(Account, GoodId, i64) - #[cfg(test)] pub(crate), no feature flag, no production surface - because no existing corruption could reach the goods check's balance-derived arm without also moving the journal arm. invariants::goods gains two negative tests, one per arm: an_exchange_whose_unit_legs_disagree_is_a_goods_leak_and_is_localised (journal residual 2, delta_units 0, posting Some) and units_conjured_outside_the_posting_path_break_the_identity_and_name_no_posting (delta_units -7, journal residual 0, posting None). Mutation-verified three ways: neutering the whole body fails both, neutering journal_residual_units fails only the first, neutering delta_units fails only the second. Two cross-phase obligations. (a) tests/lints.sh guard 7j pins the probe call count to the declaration count, so a sixth corruption method needs a matching line in tests/lint-probes/books_cfg_test_probe.rs.txt - it refused this commit until the line was added. (b) Phase 5 rewrites total_stock, produced, consumed and goods_residual_units_for to be per-good; these two tests are what will catch a rewrite that breaks the check, and their expected produced/stock values are derived from params (firms x initial_inventory_units), not read back from the books. | open |  | 2026-08-31T11:49:41.130Z |  |

````json
[
  {
    "id": 1,
    "kind": "deviation",
    "phase": "01",
    "file": ".planning/REQUIREMENTS.md",
    "line": null,
    "description": "Tasks 1 and 4 assert grep -c '^- \\[ \\] \\*\\*' == 87 as a no-rows-lost tripwire; it reports 84 because plan 01-01 checked off CORE-02/08/09. Total rows (^- \\[[ x]\\] \\*\\*) is 87. Criterion baseline is stale, not a file defect.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-30T23:17:58.131Z",
    "resolved_at": null
  },
  {
    "id": 2,
    "kind": "deviation",
    "phase": "01",
    "file": "config/baseline.toml",
    "line": null,
    "description": "V-4: theta=0.75 sense contradicts the key name price_inaction_prob_ppm; flagged in config/PROVENANCE.md for the Phase 6 gate, not corrected from memory (D-20)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-31T00:14:39.114Z",
    "resolved_at": null
  },
  {
    "id": 3,
    "kind": "deviation",
    "phase": "02",
    "file": ".planning/phases/02-books-journal-and-invariants/02-02-PLAN.md",
    "line": null,
    "description": "Task 2 verify command 'cargo test --locked --lib books invariants' is not valid cargo syntax (one positional TESTNAME only); use 'cargo test --locked --lib -- books invariants'. Plans 02-03..02-07 must not copy the broken form.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-31T09:22:48.072Z",
    "resolved_at": null
  },
  {
    "id": 4,
    "kind": "stub",
    "phase": "02",
    "file": "src/books.rs",
    "line": 118,
    "description": "Posting::units_out/units_in/goods_residual_units are zero on every posting this phase produces; plan 02-03 gives them values and 02-04 adds the goods-conservation check that reads them",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-08-31T09:22:48.317Z",
    "resolved_at": "2026-08-31T09:38:20.748Z"
  },
  {
    "id": 5,
    "kind": "deviation",
    "phase": "02",
    "file": ".planning/phases/02-books-journal-and-invariants/02-03-PLAN.md",
    "line": null,
    "description": "Task 1 acceptance criterion \"grep -c 'pub fn produce' src/books.rs prints 1\" is unsatisfiable: the pattern is a substring of 'pub fn produced', the accessor the same plan mandates in <artifacts_produced>. It prints 2. Anchored form 'pub fn produce\\(' prints 1. Same for consume/consumed.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-31T09:38:20.990Z",
    "resolved_at": null
  },
  {
    "id": 6,
    "kind": "deviation",
    "phase": "02",
    "file": "src/invariants.rs",
    "line": null,
    "description": "Violation carries Option<Box<Posting>> rather than Option<Posting>: a second posting-bearing variant pushed the enum past clippy::result_large_err's 128-byte threshold, which under -D warnings refuses to compile every Result in the crate that propagates a violation. Plans 02-04/02-05 construct Violations and must box the posting.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-31T09:38:29.928Z",
    "resolved_at": null
  },
  {
    "id": 7,
    "kind": "deviation",
    "phase": "02",
    "file": "tests/invariant_halt.rs",
    "line": null,
    "description": "Modified outside plan 02-03's files_modified: its active-check sequence assertion is [MoneyConservation, Liveness] and inserting goods conservation at position two made it fail. Plan 02-04 inserts two more checks and must update the same two assertions (here and src/invariants.rs#the_gate_decides_the_exact_sequence_of_active_checks).",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-31T09:38:30.201Z",
    "resolved_at": null
  },
  {
    "id": 8,
    "kind": "deviation",
    "phase": "02",
    "file": "src/books.rs",
    "line": null,
    "description": "Plan 02-04 task 2 modified src/books.rs though its <files> lists only src/invariants.rs: check_non_negative cannot walk accounts without an enumerator (Books::accounts) and no accessor exposed the per-slot firm generations needed to build a firm Account. Also added PostError::EmptyExchange.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-31T09:55:20.419Z",
    "resolved_at": null
  },
  {
    "id": 9,
    "kind": "deviation",
    "phase": "02",
    "file": "tests/invariant_halt.rs",
    "line": null,
    "description": "Modified outside plan 02-04's files_modified, discharging ledger entry 7: the active-check sequence assertions now read five checks with the gate on and four with it off. Behavioural claims untouched. Plans 02-05..02-07 that add a check must update the same two assertions (here and src/invariants.rs#the_gate_decides_the_exact_sequence_of_active_checks).",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-31T09:55:20.666Z",
    "resolved_at": null
  },
  {
    "id": 10,
    "kind": "deviation",
    "phase": "02",
    "file": "src/invariants.rs",
    "line": null,
    "description": "ZeroSumDetail ships 8 variants, not the 6 in plan 02-04's <artifacts_produced>: SplitParties (a one-party kind naming two accounts) and EmptyExchange (the action text's 'both legs non-zero' clause, which the six listed variants cannot express). Plan 02-05's message tests should expect eight.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-31T09:55:20.916Z",
    "resolved_at": null
  },
  {
    "id": 11,
    "kind": "deviation",
    "phase": "02",
    "file": ".planning/ROADMAP.md",
    "line": null,
    "description": "roadmap.update-plan-progress rewrote the Phase 2 plan checklist as a side effect; reverted per the wave shared-artifact rule (STATE.md and ROADMAP.md are owned by the orchestrator while sibling plans 02-06/02-07 are outstanding). Plans 02-06 and 02-07 will hit the same side effect and must revert it too.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-31T10:11:22.368Z",
    "resolved_at": null
  },
  {
    "id": 12,
    "kind": "unmet-truth",
    "phase": "02",
    "file": "tests/ledger_props.rs",
    "line": null,
    "description": "The must_have truth 'ending a tick leaves both running residuals untouched' is asserted by ending_a_tick_leaves_the_residuals_and_the_balances_untouched but cannot fail from an integration test: on the honest path the books conserve, so both residuals are already zero at every boundary. Verified by mutation - adding 'self.cash_residual_cents = 0' to Books::end_of_tick leaves the property green. The version with teeth needs a seeded non-zero residual and therefore needs the pub(crate) corruption vocabulary, which tests/ cannot reach. Plan 02-06 owns the fault-injection unit tests and should add it there.",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-08-31T10:22:33.391Z",
    "resolved_at": "2026-08-31T10:43:12.885Z"
  },
  {
    "id": 13,
    "kind": "deviation",
    "phase": "02",
    "file": "clippy.toml",
    "line": null,
    "description": "Plan 02-06 task 2 lists nine disallowed-types entries including std::sync::Arc; only eight were added. Arc makes the clean tree fail check 1: proptest's prop_oneof! expands to code naming it, producing 9 diagnostics across 7 call sites in tests/ledger_props.rs and tests/money_props.rs, and check 4b forbids a lint exemption anywhere in tracked Rust source. Same class of finding as the plan's own RefCell exclusion; recorded in the clippy.toml comment and covered by guard 7c (Arc absent from src/). Later phases adding a proptest strategy must not re-add the entry.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-31T10:43:27.921Z",
    "resolved_at": null
  },
  {
    "id": 14,
    "kind": "deviation",
    "phase": "02",
    "file": "tests/lints.sh",
    "line": null,
    "description": "Guards 7e and 7h are scoped to the production half of src/invariants.rs (everything before the first #[cfg(test)] line) and 7e counts the qualified field read '.liveness_enabled' rather than the bare identifier. Written literally as the plan specifies, both fail on the real tree: the unit-test modules legitimately load the shipped config from a path (7h) and set the key on a Params value (7e), and the one production read binds a local that is used again a few lines later to filter the check table (7e). The scoping is stated in the script comments.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-31T10:43:28.153Z",
    "resolved_at": null
  },
  {
    "id": 15,
    "kind": "deviation",
    "phase": "02",
    "file": "src/books.rs",
    "line": null,
    "description": "Modified outside plan 02-06's files_modified, discharging ledger entry 12 at the orchestrator's instruction: mod tests gains ending_a_tick_leaves_a_seeded_non_zero_residual_of_either_kind_untouched, which seeds a non-zero cash and goods residual with the pub(crate) corruption vocabulary before crossing the tick boundary. Mutation-verified in both profiles. No production code changed.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-31T10:43:28.412Z",
    "resolved_at": null
  },
  {
    "id": 16,
    "kind": "deviation",
    "phase": "02",
    "file": "src/books.rs",
    "line": null,
    "description": "Code-review fix CR-01 added PostError::EmptyTransfer and ZeroSumDetail::EmptyTransfer. Books::transfer now refuses Money::ZERO before the self-dealing check, so transfer(a, a, 0) reports EmptyTransfer where it previously reported SelfDealing. Phase 6's partial-payroll and Phase 8's dividend call sites must treat a zero-amount transfer as a refusal rather than an Ok(Money::ZERO): the returned amount is still the amount moved (LEDG-03), but the Err arm is now reachable for an amount a caller computed as zero.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-31T11:25:43.540Z",
    "resolved_at": null
  },
  {
    "id": 17,
    "kind": "deviation",
    "phase": "02",
    "file": "src/invariants.rs",
    "line": null,
    "description": "Code-review fix CR-01/WR-08 grew ZeroSumDetail from 8 variants to 10: EmptyTransfer (a transfer with no cash on either leg, the counterpart of EmptyExchange) and UnitsInTheWrongDirection (a production that also released units, or a consumption that also received them - reported as UnitLegsDiffer before, whose message contradicted the equal numbers it carried). invariants::message::DETAIL_SHAPES is 10; detail_position and every_detail are exhaustive matches, so a later phase adding a variant is named by the compiler.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-31T11:26:08.883Z",
    "resolved_at": null
  },
  {
    "id": 18,
    "kind": "deviation",
    "phase": "02",
    "file": "clippy.toml",
    "line": null,
    "description": "Code-review finding WR-05 asked for std::process::id in disallowed-methods. DECLINED, with the same reasoning clippy.toml already records for RefCell and Arc: tests/config_strict.rs:275 and tests/tracer_end_to_end.rs:21 call it to build a unique temporary path, so the entry makes tests/lints.sh check 1 fail (verified: error: use of a disallowed method std::process::id --> tests/config_strict.rs:275) and check 4b forbids the #[allow(...)] that would silence it. tests/lints.sh guard 7h carries the rule instead, now over the production half of src/books.rs as well as src/invariants.rs. No BANNEDCALL line added, so check 3 still reads 60 against 60. A later phase must not re-add the entry without first moving those two temp-path helpers.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-31T11:26:09.121Z",
    "resolved_at": null
  },
  {
    "id": 19,
    "kind": "deviation",
    "phase": "02",
    "file": "tests/lints.sh",
    "line": null,
    "description": "Code-review fixes WR-03/04/05/06 changed tests/lints.sh: assert_fires and assert_ignores moved above check 4b (they were defined in section 7, below their new first use); check 4b's exemption pattern now matches the banned lint in any argument position and inside cfg_attr, and has assert_fires/assert_ignores proofs; guard 7d gained a second clause over every tracked src/*.rs with line comments stripped, carving out src/rng.rs, so the file Phase 3 adds for the tick loop is guarded on the commit that adds it; guard 7h searches src/books.rs's production half too; guards 7i and 7j are new (every corrupt_* declaration is inside a #[cfg(test)] block, and the cfg-test probe calls every one of them). Section 7 is now ten guards, not eight - the count appears in three echo strings and two comments.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-31T11:26:09.369Z",
    "resolved_at": null
  },
  {
    "id": 20,
    "kind": "unmet-truth",
    "phase": "02",
    "file": "tests/ledger_props.rs",
    "line": null,
    "description": "Code-review finding WR-02: posting_residuals_agree_with_the_balance_derived_quantities is structurally 0 == 0 from an integration test, because record derives both residuals from the same argument the balance write used. Verified by mutation - deriving record's cash residual from total_money() leaves all 8 properties in the file green. DISCHARGED by books::tests::the_two_residual_sources_move_apart_when_only_one_of_them_is_told, which appends postings whose legs disagree while touching no balance and is the only thing that fails under that mutation. The property's doc comment now records what it does and does not prove, and names the unit test. Same shape as ledger entry 12.",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-08-31T11:26:09.601Z",
    "resolved_at": "2026-08-31T11:26:22.135Z"
  },
  {
    "id": 21,
    "kind": "deviation",
    "phase": "02",
    "file": ".planning/ROADMAP.md",
    "line": null,
    "description": "Code-review finding WR-06 asked for the guard-7d scope extension to be recorded as a ROADMAP success criterion, the way guard 7f's inherited obligation is. NOT WRITTEN: this fix pass is explicitly barred from editing .planning/ROADMAP.md and .planning/STATE.md. The obligation is discharged rather than deferred - guard 7d now searches every tracked src/*.rs with src/rng.rs carved out by name, so a Phase 3 src/world.rs naming debug_assert or cfg(debug_assertions) fails the guard on the commit that adds it, with no promise for a future reader to keep. Watched firing on exactly that shape.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-31T11:26:09.827Z",
    "resolved_at": null
  },
  {
    "id": 22,
    "kind": "deviation",
    "phase": "02",
    "file": "src/books.rs",
    "line": null,
    "description": "Code-review fix WR-01 added BooksError::EndowmentOutOfRange and an up-front checked closed-form endowment gate in Books::new, and WR-07 added Books::goods_residual_units_for(GoodId). Both are new public API in a review-fix pass. The first is required because the money-side gate was a saturating running sum; the second because check_goods read one residual outside its per-good loop. Phase 5 changes goods_residual_units_for's body and check_goods does not move; the GOODS doc comment now lists the four things Phase 5 actually inherits instead of promising that nothing moves.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-31T11:26:10.066Z",
    "resolved_at": null
  },
  {
    "id": 23,
    "kind": "unmet-truth",
    "phase": "02",
    "file": "src/invariants.rs",
    "line": 581,
    "description": "ROADMAP Phase 2 criterion 2 ('the negative test passes for EVERY check') was met for four of the five checks. check_goods (goods conservation, LEDG-05) had no negative test: every call site reaching it asserted Ok(()), the localisation test called first_breaking_goods_posting directly, and the message module rendered a hand-built GoodsConservation the check never produced. Verified by 02-VERIFICATION mutation M10 - replacing the whole body with 'if true { return Ok(()); }' left all 239 tests green. Plan 02-05 scoped exactly four violation classes (LEDG-04/06/07/10) and goods was never in scope, so the 'every check' self-audit never ran and nothing recorded the omission.",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-08-31T11:49:16.631Z",
    "resolved_at": "2026-08-31T11:49:20.831Z"
  },
  {
    "id": 24,
    "kind": "deviation",
    "phase": "02",
    "file": "src/books.rs",
    "line": null,
    "description": "Closure of ledger entry 23. Books gains a fifth corruption method, corrupt_silent_stock(Account, GoodId, i64) - #[cfg(test)] pub(crate), no feature flag, no production surface - because no existing corruption could reach the goods check's balance-derived arm without also moving the journal arm. invariants::goods gains two negative tests, one per arm: an_exchange_whose_unit_legs_disagree_is_a_goods_leak_and_is_localised (journal residual 2, delta_units 0, posting Some) and units_conjured_outside_the_posting_path_break_the_identity_and_name_no_posting (delta_units -7, journal residual 0, posting None). Mutation-verified three ways: neutering the whole body fails both, neutering journal_residual_units fails only the first, neutering delta_units fails only the second. Two cross-phase obligations. (a) tests/lints.sh guard 7j pins the probe call count to the declaration count, so a sixth corruption method needs a matching line in tests/lint-probes/books_cfg_test_probe.rs.txt - it refused this commit until the line was added. (b) Phase 5 rewrites total_stock, produced, consumed and goods_residual_units_for to be per-good; these two tests are what will catch a rewrite that breaks the check, and their expected produced/stock values are derived from params (firms x initial_inventory_units), not read back from the books.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-31T11:49:41.130Z",
    "resolved_at": null
  }
]
````
