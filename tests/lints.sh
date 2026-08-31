#!/usr/bin/env bash
#
# CORE-07, and clause (b) of the amended CORE-03: the determinism ban lists as
# an observed block, not as a configuration that exists.
#
# A lint never observed to block has never been shown to work — the same
# standard this project applies to its own invariants. So this script does not
# check that clippy.toml contains the right lines; it injects a known hazard
# and watches the gate stop it, then puts the tree back.
#
# It is a shell script rather than a libtest case because it must assert that
# a build FAILS, which libtest cannot express.
#
# Research found three enforcement holes, each of which would have shipped as a
# silently-passing gate. The four checks below close one each, plus the clean
# baseline:
#
#   1. the clean tree passes            (baseline: the gate is not stuck red)
#   2. an injected hazard fails         (hole: `cargo clippy` does not lint tests/)
#   3. every configured path fires      (hole: clippy ignores unresolvable paths in silence)
#   4. the escape hatches are absent    (hole: a type alias makes use sites invisible)
#
# Plan 02-06 added three more, for the parts of LEDG-01, LEDG-02 and LEDG-10
# that no compiler and no lint can express:
#
#   5. a shared borrow held across a mutation does not compile   (LEDG-02 leg 1)
#   6. the fault-injection vocabulary is unreachable from tests/ (LEDG-10)
#   7. eleven source guards, each proved to fire first              (LEDG-01/02/10)
#
# Checks 5 and 6 assert the specific DIAGNOSTIC CODE and not merely that the
# build failed, for the reason check 2 documents about itself. Check 7's guards
# are each proved to match a known hazard fixture before being asserted absent
# from the tree, because a grep pattern with a typo matches nothing and is
# indistinguishable from a clean tree — the grep form of the hole check 3 closes.
#
# No error-suppressing fallbacks anywhere: a missing input must surface as a
# failure, never default into a passing comparison.

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

# The flags are load-bearing and are never abbreviated in this file.
# Without --all-targets clippy does not lint tests/ at all, and check 2 —
# which injects its hazard there on purpose — would pass green.
CLIPPY_FLAGS=(--all-targets --all-features)

PROBE_SRC="tests/lint-probes/float_ban_probe.rs.txt"
HAZARD_SRC="tests/lint-probes/hazard.rs.txt"
PROBE_DST="tests/_probe.rs"
HAZARD_DST="tests/_hazard.rs"

BORROW_PROBE_SRC="tests/lint-probes/books_borrow_probe.rs.txt"
BORROW_PROBE_DST="tests/_borrow_probe.rs"
CFG_PROBE_SRC="tests/lint-probes/books_cfg_test_probe.rs.txt"
CFG_PROBE_DST="tests/_cfg_test_probe.rs"

# The two ledger modules checks 7a-7j are written against.
BOOKS_SRC="src/books.rs"
INVARIANTS_SRC="src/invariants.rs"

# The three modules plan 03-02 added to the halt path. The first two can carry
# or render a violation and hold no filesystem path, so guard 7h's full pattern
# reaches them unchanged. The third writes every diffed byte and holds a path by
# construction, so guard 7h-log covers it with one alternative dropped — see the
# comment above that clause for why the narrowing is bounded rather than free.
PHASES_SRC="src/phases.rs"
WORLD_SRC="src/world.rs"
LOG_SRC="src/log.rs"

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

# Restore the tree on EVERY exit path, including a failed check and an
# interrupt. A negative test that leaves its own hazard behind has poisoned
# the working tree it was meant to protect.
cleanup() {
    rm -f "$PROBE_DST" "$HAZARD_DST" "$BORROW_PROBE_DST" "$CFG_PROBE_DST"
}
trap cleanup EXIT INT TERM

# grep exits 0 for a match, 1 for "no match" (a pass) and 2 for a real error
# such as an unreadable file (a fail). Collapsing 1 and 2 with `|| true` would
# let an unreadable file default to a passing comparison — the precise class of
# silent hole this script exists to close.
assert_absent() {
    local what="$1"
    shift
    local hits status
    set +e
    hits=$(grep "$@")
    status=$?
    set -e
    if [ "$status" -gt 1 ]; then
        fail "could not search for $what (grep exit $status)"
    fi
    if [ -n "$hits" ]; then
        fail "$what — found: $(echo "$hits" | tr '\n' ' ')"
    fi
}

# Proof that a guard's pattern fires: it must match exactly the number of hazard
# lines its fixture holds, so every alternative in an alternation is exercised
# rather than only the first.
assert_fires() {
    local what="$1" pattern="$2" expected="$3" fixture="$4"
    local hits status
    set +e
    # -e, because a pattern may legitimately begin with a dash (guard 7g's
    # return-type pattern starts with "->") and grep would read it as a flag.
    hits=$(printf '%s\n' "$fixture" | grep -cE -e "$pattern")
    status=$?
    set -e
    if [ "$status" -gt 1 ]; then
        fail "guard $what: could not search its own hazard fixture (grep exit $status)"
    fi
    if [ "$hits" -ne "$expected" ]; then
        fail "guard $what: its pattern matched $hits of the $expected hazard lines in its own fixture — the pattern has a typo, so its silence on the real tree would prove nothing"
    fi
}

# The other half of the same discipline: a pattern must leave the PERMITTED
# spelling alone, or the guard fires on legitimate source and gets deleted.
assert_ignores() {
    local what="$1" pattern="$2" fixture="$3"
    local hits status
    set +e
    # -e, because a pattern may legitimately begin with a dash (guard 7g's
    # return-type pattern starts with "->") and grep would read it as a flag.
    hits=$(printf '%s\n' "$fixture" | grep -cE -e "$pattern")
    status=$?
    set -e
    if [ "$status" -gt 1 ]; then
        fail "guard $what: could not search its permitted-spelling fixture (grep exit $status)"
    fi
    if [ "$hits" -ne 0 ]; then
        fail "guard $what: its pattern matched $hits line(s) of its PERMITTED fixture — it would fire on legitimate source"
    fi
}

for f in "$PROBE_SRC" "$HAZARD_SRC" "$BORROW_PROBE_SRC" "$CFG_PROBE_SRC" \
         "$BOOKS_SRC" "$INVARIANTS_SRC" "$PHASES_SRC" "$WORLD_SRC" "$LOG_SRC" \
         clippy.toml; do
    [ -f "$f" ] || fail "required input $f is missing"
    [ -s "$f" ] || fail "required input $f is empty — a guard over an empty file passes trivially"
done

# ---------------------------------------------------------------------------
# 1. The clean tree passes.
# ---------------------------------------------------------------------------
# Establishes that anything the later checks observe is caused by what they
# injected, and that the gate has not simply been left red.

if ! cargo clippy "${CLIPPY_FLAGS[@]}" -- -D warnings > /dev/null 2>&1; then
    fail "the clean tree does not pass 'cargo clippy ${CLIPPY_FLAGS[*]} -- -D warnings'"
fi
echo "  check 1: clean tree passes the lint gate"

# ---------------------------------------------------------------------------
# 2. An injected hazard fails — in the directory the bare command misses.
# ---------------------------------------------------------------------------
# Verified in research: with a hashed collection sitting in tests/, plain
# `cargo clippy` exits 0 and `cargo clippy --all-targets` exits 101. The
# hazard is injected here, not in src/, so that this check would go green if
# anyone ever trimmed the flags.
#
# Both lints are asserted SEPARATELY. A bare "the build failed" assertion
# would stay green after one of the two ban lists was deleted, because the
# other hazard in the same file would still fail the build — the check would
# then be reporting the wrong lint's health.

cp "$HAZARD_SRC" "$HAZARD_DST"
# A non-zero status is the EXPECTED outcome here, so it is captured and
# examined rather than allowed to abort the script.
set +e
HAZARD_OUT=$(cargo clippy "${CLIPPY_FLAGS[@]}" -- -D warnings 2>&1)
HAZARD_STATUS=$?
set -e
rm -f "$HAZARD_DST"

if [ "$HAZARD_STATUS" -eq 0 ]; then
    fail "a hashed collection and a banned float method in tests/ did NOT fail the lint gate"
fi
case "$HAZARD_OUT" in
    *"use of a disallowed type"*) ;;
    *) fail "the injected hashed collection produced no disallowed-type diagnostic — the disallowed-types list is not enforcing" ;;
esac
case "$HAZARD_OUT" in
    *"use of a disallowed method"*) ;;
    *) fail "the injected float method produced no disallowed-method diagnostic — the disallowed-methods list is not enforcing" ;;
esac
echo "  check 2: an injected hazard in tests/ is blocked by both lists"

# ---------------------------------------------------------------------------
# 3. Every configured disallowed-methods path actually fires.
# ---------------------------------------------------------------------------
# Clippy accepts a disallowed-methods path it cannot resolve WITHOUT any
# diagnostic, so a typo in one of the 68 paths looks exactly like a working
# ban. The probe calls every entry that resolves on stable exactly once;
# comparing the diagnostic count against the marked call-site count turns that
# silence into a failure.
#
# The probe covers the two CLOCK bans as well as the floats. It did not
# originally, and that was the coverage hole: `std::time::SystemTime::now` and
# `std::time::Instant::now` were the only entries in clippy.toml whose
# resolution nothing asserted, while the wall-clock ban is one of the top
# determinism hazards in CLAUDE.md. Verified by corrupting the Instant path to
# `Instannt`: this check now reports 59 diagnostics against 60 call sites and
# fails, where before it would have passed green.
#
# Both numbers are computed — neither is written here as a literal, or the
# assertion would drift out of date the first time an entry is added.

set +e
MARKED=$(grep -c '// BANNEDCALL$' "$PROBE_SRC")
GREP_STATUS=$?
set -e
if [ "$GREP_STATUS" -gt 1 ]; then
    fail "could not count marked call sites in $PROBE_SRC (grep exit $GREP_STATUS)"
fi
if [ "$MARKED" -eq 0 ]; then
    fail "$PROBE_SRC contains no marked call sites — the probe cannot prove anything"
fi

cp "$PROBE_SRC" "$PROBE_DST"
# The probe is expected to fail the build, so a non-zero status carries no
# information: the OUTPUT is what is inspected.
set +e
CLIPPY_OUT=$(cargo clippy "${CLIPPY_FLAGS[@]}" 2>&1)
set -e
rm -f "$PROBE_DST"

# A count of zero is a legitimate reading here (it is exactly the failure this
# check exists to report), so grep's "no match" status 1 is distinguished from
# a real error rather than collapsed into it.
set +e
FIRED=$(printf '%s\n' "$CLIPPY_OUT" | grep -c 'use of a disallowed method')
GREP_STATUS=$?
set -e
if [ "$GREP_STATUS" -gt 1 ]; then
    fail "could not count disallowed-method diagnostics (grep exit $GREP_STATUS)"
fi

if [ "$FIRED" -ne "$MARKED" ]; then
    echo "  marked call sites in the probe : $MARKED" >&2
    echo "  disallowed-method diagnostics  : $FIRED" >&2
    fail "a configured disallowed-methods path did not fire — clippy ignores a path it cannot resolve in silence, so a typo in clippy.toml looks identical to a working ban"
fi
echo "  check 3: all $FIRED resolvable method bans (floats + the clock) fired, one per marked call site"

# ---------------------------------------------------------------------------
# 4. The escape hatches are absent.
# ---------------------------------------------------------------------------

# 4a. No type alias to a hashed collection. Verified in research: an alias
#     behind an exemption makes every downstream use site COMPLETELY invisible
#     to disallowed_types. The lint cannot see this; only a grep can.
#
#     The visibility group admits `pub(crate)`, `pub(super)` and `pub(in ...)`.
#     Anchoring on `pub[[:space:]]+` alone missed all of them — and `pub(crate)`
#     is precisely the visibility most likely to be used for an alias inside a
#     single crate, so the guard was blind to its own most probable case.
assert_absent "a type alias to a hashed collection exists under src/" \
    -rEn '^[[:space:]]*(pub([[:space:]]*\([^)]*\))?[[:space:]]+)?type[[:space:]]+[A-Za-z0-9_]+.*=.*Hash(Map|Set)' src/

# 4b. No lint exemption for EITHER determinism ban, anywhere in the crate's
#     Rust source. Scoped to tracked *.rs files because an attribute has effect
#     only in Rust source: the planning documents and CLAUDE.md quote the
#     attribute in prose and in code examples, and matching those would be
#     matching a description of the hole rather than the hole.
#
#     This searched only for `clippy::disallowed_types`, which left the larger
#     list unguarded: a single `#![allow(clippy::disallowed_methods)]` at the
#     top of a module disables all 68 float and clock bans at once. Check 3
#     proves the list RESOLVES; nothing proved no file opts out of it.
#     `clippy::all` and `warnings` are included for the same reason — both
#     silence the bans without naming them.
mapfile -t RUST_SOURCES < <(git ls-files -- '*.rs')
if [ "${#RUST_SOURCES[@]}" -eq 0 ]; then
    fail "found no tracked Rust source files to search — expected at least src/lib.rs"
fi
#
#     THE PATTERN MUST NOT ANCHOR ON THE FIRST ARGUMENT. It used to:
#     '#!?\[(allow|expect)\((warnings|...)' requires the banned lint name to be
#     the first thing inside the parentheses, so it matched
#     `#![allow(clippy::disallowed_types)]` and missed BOTH of the spellings a
#     developer is most likely to reach for —
#     `#[allow(dead_code, clippy::disallowed_methods)]`, added to silence one
#     unrelated warning, and `#[cfg_attr(test, allow(clippy::disallowed_methods))]`.
#     Each disables all 68 float and clock bans in its module while
#     `cargo clippy -- -D warnings` passes and this check reports nothing.
#     `[^)]*` before the alternation admits any earlier arguments; the optional
#     `cfg_attr\([^)]*,` group admits the conditional form.
EXEMPTION_PATTERN='#!?\[(cfg_attr\([^)]*,[[:space:]]*)?(allow|expect)\([^)]*(warnings|clippy::(all|disallowed_types|disallowed_methods))'
assert_fires 4b "$EXEMPTION_PATTERN" 4 '#![allow(clippy::disallowed_types)]
#[allow(dead_code, clippy::disallowed_methods)]
#[cfg_attr(test, allow(clippy::disallowed_methods))]
#[expect(warnings)]'
assert_ignores 4b "$EXEMPTION_PATTERN" '#[allow(dead_code)]
#[expect(unused_variables)]
#[derive(Debug, Clone)]
    let warnings = collect();'
assert_absent "a file carries a lint exemption for a determinism ban" \
    -En "$EXEMPTION_PATTERN" \
    -- "${RUST_SOURCES[@]}"

# 4c. No point-lookup wrapper module. D-06 declines to build the hatch in this
#     phase: not building it is the cheapest way to keep the ban honest, and
#     its absence is checkable where a promise not to misuse it is not.
if [ -e src/lookup.rs ]; then
    fail "src/lookup.rs exists — D-06 declines to build the point-lookup wrapper in this phase"
fi

# 4d. CORE-03 clause (b): the non-portable generators are banned by USE.
#     rand 0.10.2 makes SmallRng and the Xoshiro types unconditional — the
#     small_rng feature is gone but the types still compile — so they cannot be
#     removed from the dependency graph and absence is unachievable. The lint
#     entry stops the accident; this assertion is what makes the clause a
#     checkable fact about the source rather than a claim about the graph.
assert_absent "a non-portable generator type is named under src/" \
    -rEn 'SmallRng|Xoshiro256PlusPlus|Xoshiro128PlusPlus' src/

echo "  check 4: no alias, no exemption, no lookup wrapper, no non-portable generator"

# ---------------------------------------------------------------------------
# 5. A shared borrow held across a mutation does not compile (LEDG-02 leg 1).
# ---------------------------------------------------------------------------
# The ROADMAP phrases LEDG-02's criterion as "a test observing the books
# mid-transaction is impossible to write". Asserting that proves nothing; this
# check writes the test and watches the compiler refuse it.
#
# The E0502 assertion is load-bearing rather than decorative. A bare "the build
# failed" assertion would stay green when the probe stops compiling for an
# unrelated reason — a renamed constructor, a changed `transfer` signature — and
# the check would then be reporting the wrong thing's health. That is the same
# hole this script already documents for its own check 2.

cp "$BORROW_PROBE_SRC" "$BORROW_PROBE_DST"
# A non-zero status is the EXPECTED outcome, so it is captured and examined
# rather than allowed to abort the script.
set +e
BORROW_OUT=$(cargo build --tests 2>&1)
BORROW_STATUS=$?
set -e
rm -f "$BORROW_PROBE_DST"

if [ "$BORROW_STATUS" -eq 0 ]; then
    fail "a shared borrow of the books held live across a call to transfer COMPILED — LEDG-02 leg 1 does not hold, or the trailing use of the borrow in tests/lint-probes/books_borrow_probe.rs.txt was tidied away so the borrow no longer spans the mutation"
fi
case "$BORROW_OUT" in
    *"E0502"*) ;;
    *) fail "tests/lint-probes/books_borrow_probe.rs.txt failed to build but produced no E0502 — it broke for an unrelated reason (a renamed constructor, a changed signature), so this check is reporting nothing about the borrow rule" ;;
esac
echo "  check 5: a shared borrow held across a mutation is refused with E0502"

# ---------------------------------------------------------------------------
# 6. The fault-injection vocabulary is unreachable from tests/ (LEDG-10).
# ---------------------------------------------------------------------------
# Plan 02-05's negative tests rest entirely on the corruption methods being
# visible to the crate's own unit tests and to nothing else. That claim was
# otherwise only written in a doc comment; this check executes it.
#
# E0599 ("no method named ... found") is asserted specifically, for the reason
# check 5 gives: a bare build failure would stay green if the probe broke for an
# unrelated reason, and the boundary would go unguarded while the gate stayed
# green.

cp "$CFG_PROBE_SRC" "$CFG_PROBE_DST"
set +e
CFG_OUT=$(cargo build --tests 2>&1)
CFG_STATUS=$?
set -e
rm -f "$CFG_PROBE_DST"

if [ "$CFG_STATUS" -eq 0 ]; then
    fail "tests/lint-probes/books_cfg_test_probe.rs.txt COMPILED: an integration test called a fault-injection method on Books — the corruption vocabulary has escaped the crate's own test configuration and is reachable by any consumer of sim"
fi
case "$CFG_OUT" in
    *"E0599"*) ;;
    *) fail "tests/lint-probes/books_cfg_test_probe.rs.txt failed to build but produced no E0599 — it broke for an unrelated reason, so this check is reporting nothing about the fault-injection boundary" ;;
esac
echo "  check 6: the fault-injection vocabulary is refused from tests/ with E0599"

# ---------------------------------------------------------------------------
# 7. Eleven source guards, each proved to fire before it is trusted to be silent.
# ---------------------------------------------------------------------------
# These are the parts of LEDG-01, LEDG-02 and LEDG-10 that no compiler and no
# lint can express.
#
# THE DISCIPLINE, and it applies to all eleven. A grep pattern with a typo
# matches nothing, and a pattern that matches nothing looks exactly like a
# pattern that is silent because the tree is clean — the grep form of the hole
# check 3 exists to close for clippy's silently-unresolvable paths. So every
# guard below defines its pattern once, proves it MATCHES a hazard fixture
# holding the very thing it is meant to catch, and only then asserts the pattern
# is absent from the real files. Where a guard must also leave a legitimate
# lookalike alone, that is asserted against a second, PERMITTED fixture.
#
# Every guard also asserts its search set is non-empty, following check 4b's
# treatment of the tracked-file list: a guard over an empty set passes trivially
# and is then believed to be protecting something (02-RESEARCH.md Pitfall 4).
#
# `assert_fires` and `assert_ignores` are defined near the top of this file
# rather than here, because check 4b is under the same discipline and runs
# first. They were originally defined at this point, which is why 4b — the guard
# that shipped with a pattern anchored on the FIRST argument of an `allow(...)`,
# blind to `#[allow(dead_code, clippy::disallowed_methods)]` — had no proof that
# it fired at all.
#
# WHAT COUNTS AS A GUARD, since the number appears in this script's own output
# and two plans in one phase both had to decide it. A guard is COUNTED when a
# requirement or a roadmap criterion names it as an obligation of its own.
# Everything else is a CLAUSE of a guard that already exists, and clauses are
# not counted however much work they do.
#
#   counted:     `7f-agents`. ROADMAP Phase 3 criterion 7 is about it — the
#                agent types hold no balance — so it answers for itself.
#   not counted: the cash-setter clause of 7f, and `7h-log` below. Neither has
#                a requirement of its own; each narrows or extends the guard it
#                sits under, and each is proved by its own fixtures regardless.
#
# So the number does not move when a clause is added, and a reader comparing
# this script against a plan does not have to reconstruct the rule from the
# arithmetic. It moves only when a requirement or a criterion arrives.

# Absence over content held in a variable, so a guard can search a file with its
# line comments stripped or its test modules removed and still report real line
# numbers. Same grep-status discipline as assert_absent: status 2 is a failure
# and never a passing comparison.
assert_absent_in() {
    local what="$1" pattern="$2" content="$3"
    local hits status
    set +e
    hits=$(printf '%s\n' "$content" | grep -nE -e "$pattern")
    status=$?
    set -e
    if [ "$status" -gt 1 ]; then
        fail "could not search for $what (grep exit $status)"
    fi
    if [ -n "$hits" ]; then
        fail "$what — found: $(echo "$hits" | tr '\n' ' ')"
    fi
}

# Everything before the first `#[cfg(test)]` line: the code that ships in the
# binary that produced a run. Guards 7e and 7h are claims about THAT code. The
# unit-test modules below it legitimately load the shipped configuration from a
# path and legitimately set the liveness key on a Params value, and a guard that
# fired on them would be forbidding the tests from testing.
production_source() {
    awk 'index($0, "#[cfg(test)]") == 1 { exit } { print }' "$1"
}

mapfile -t SRC_FILES < <(git ls-files -- 'src/*.rs')
if [ "${#SRC_FILES[@]}" -eq 0 ]; then
    fail "found no tracked Rust sources under src/ to search — expected at least src/lib.rs"
fi

# Line comments stripped: a doc comment explaining a rule must not trip the
# guard that enforces it. Line numbers are preserved, one output line per input
# line, so a failure still names the real line in the real file.
BOOKS_CODE=$(sed 's://.*::' "$BOOKS_SRC")
INVARIANTS_CODE=$(production_source "$INVARIANTS_SRC" | sed 's://.*::')
INVARIANTS_PRODUCTION=$(production_source "$INVARIANTS_SRC")

for content in "$BOOKS_CODE" "$INVARIANTS_CODE" "$INVARIANTS_PRODUCTION"; do
    [ -n "$content" ] || fail "a guard's search content is empty — a guard over an empty set passes trivially"
done

# 7a. No callback in the ledger.
#
#     An exclusive `&mut self` borrow constrains an EXTERNAL observer. A
#     callback is an INTERNAL one and is exempt by construction: a `&mut Books`
#     method taking `hook: impl Fn(&Books)` hands that hook a mid-transaction
#     view, and it compiles and runs clean. Reproduced in research — the hook
#     observed a total of 50 cents against an opening stock of 100. So leg 1 of
#     LEDG-02 alone is FALSE, and this guard is the leg that closes it.
#
#     The next person to add a mutating method with a logging hook must find
#     that reason here rather than read the guard as arbitrary. Read the journal
#     after the call instead — that is what it is for.
CALLBACK_PATTERN='impl[[:space:]]+Fn(Mut|Once)?[[:space:]<(]|dyn[[:space:]]+Fn|Box<[[:space:]]*dyn|&[[:space:]]*dyn[[:space:]]'
assert_fires 7a "$CALLBACK_PATTERN" 6 '    pub fn transfer(&mut self, amount: Money, hook: impl Fn(&Books)) {}
    pub fn transfer(&mut self, amount: Money, hook: impl FnMut(&Books)) {}
    pub fn transfer(&mut self, amount: Money, hook: impl FnOnce(&Books)) {}
    pub fn transfer(&mut self, hook: &mut dyn FnMut(&Books)) {}
    observer: Box<dyn Observer>,
    pub fn transfer(&mut self, observer: &dyn Observer) {}'
assert_ignores 7a "$CALLBACK_PATTERN" '    pub fn accounts(&self) -> impl Iterator<Item = Account> + '"'"'_ {
impl Books {
    fn record(&mut self, draft: Posting) {'
assert_absent_in "guard 7a: a mutating ledger method takes a callback, which is an INTERNAL observer the borrow checker cannot constrain (LEDG-02 leg 2; a hook of this shape observed a total of 50 against an opening 100 and compiled clean). Read the journal after the call instead" \
    "$CALLBACK_PATTERN" "$BOOKS_CODE"

# 7b. No shared mutability in the ledger.
#
#     The clippy entries from task 2 cover all of these crate-wide EXCEPT
#     RefCell and Arc, which the clean tree cannot carry (see the comment in
#     clippy.toml). This guard is what covers those two inside the ledger, and
#     it also catches an alias or a re-export that a type-path lint cannot see.
SHARED_MUT_PATTERN='RefCell|(^|[^A-Za-z0-9_])Cell[<:]|UnsafeCell|OnceCell|LazyCell|(^|[^A-Za-z0-9_])Rc[<:]|(^|[^A-Za-z0-9_])Arc[<:]|Mutex|RwLock|OnceLock'
assert_fires 7b "$SHARED_MUT_PATTERN" 10 '    balances: RefCell<Vec<Money>>,
    counter: Cell<u32>,
    raw: UnsafeCell<Money>,
    memo: OnceCell<Money>,
    memo: LazyCell<Money>,
    owner: Rc<Books>,
    owner: Arc<Books>,
    guard: Mutex<Books>,
    guard: RwLock<Books>,
    memo: OnceLock<Money>,'
assert_ignores 7b "$SHARED_MUT_PATTERN" '    household_cash: Vec<Money>,
    let cells = 3;
    fn cash_at(&self, slot: AccountSlot) -> Money {'
assert_absent_in "guard 7b: the ledger names a shared-mutability or reference-counted wrapper, which hands out a mid-transaction view through a shared reference (LEDG-02 leg 3). The books dissolve the two-mutable-borrows problem by having one owner, not by sharing one" \
    "$SHARED_MUT_PATTERN" "$BOOKS_CODE"

# 7c. The one permitted shared-mutability site, and the one that has none.
#
#     This is the guard clippy.toml points at for the two types the clean tree
#     cannot ban. It is STRONGER than a books-only rule: a new use anywhere in
#     the crate has to be argued for rather than merely not noticed.
#
#     ITS TWO SCOPE EDGES, stated here because a future reader deciding whether
#     it is enough must find both rather than infer equivalence:
#       - STRONGER than the lint entry it substitutes for, because a source grep
#         catches a type alias or a re-export a type-path lint cannot see.
#       - NARROWER, because it searches src/ only, while a clippy.toml entry
#         under --all-targets would also have reached tests/ and benches/.
set +e
REFCELL_FILES=$(grep -rlE 'RefCell' -- "${SRC_FILES[@]}" | sort | tr '\n' ' ')
GREP_STATUS=$?
set -e
if [ "$GREP_STATUS" -gt 1 ]; then
    fail "guard 7c: could not search for RefCell under src/ (grep exit $GREP_STATUS)"
fi
if [ "$REFCELL_FILES" != "src/rng.rs " ]; then
    fail "guard 7c: RefCell appears under src/ in [$REFCELL_FILES] — expected exactly src/rng.rs, the debug-only sub-stream re-entry guard (D-04, T-1-13). This is the one permitted shared-mutability site and it is why clippy.toml cannot carry a RefCell entry; a new one anywhere else is an LEDG-02 hole. NOTE the guard's scope edges: it is stronger than the lint entry it substitutes for (a grep catches an alias or a re-export a type-path lint cannot see) and narrower (it searches src/ only, while the lint under --all-targets would also cover tests/ and benches/)"
fi
ARC_PATTERN='(^|[^A-Za-z0-9_])Arc[<:]'
assert_fires 7c "$ARC_PATTERN" 2 '    owner: Arc<Books>,
    let books = Arc::new(books);'
assert_ignores 7c "$ARC_PATTERN" '    let arch = 3;
    fn march(&self) {}'
assert_absent "guard 7c: Arc is named under src/. clippy.toml cannot ban it — proptest's prop_oneof! expands to code naming it in tests/ — so this grep is the whole of its enforcement, and the simulation is single-threaded (CORE-04)" \
    -rnE "$ARC_PATTERN" -- "${SRC_FILES[@]}"

# 7d. No compiled-out invariants (LEDG-10, ROADMAP criterion 4 read literally).
#
#     An invariant that a build profile compiles out is not an invariant of the
#     binary that produced a run. Searched over the RAW files, comments
#     included, on the same terms as the float-name rule in tests/numeric_det.rs:
#     the way to say "this is not a debug_assert" in a doc comment is to not
#     write the token.
#
#     The pattern must NOT match `cfg(test)`. That is a DIFFERENT predicate and
#     it is what plan 02-05's fault-injection vocabulary legitimately uses, so
#     the permitted fixture below asserts the distinction explicitly rather than
#     leaving it to be inferred from the pattern.
COMPILED_OUT_PATTERN='debug_assert'
assert_fires 7d "$COMPILED_OUT_PATTERN" 3 '        debug_assert!(self.cash_residual_cents == 0);
        debug_assert_eq!(books.total_money(), opening);
    #[cfg(debug_assertions)]'
assert_ignores 7d "$COMPILED_OUT_PATTERN" '#[cfg(test)]
impl Books {
    #[cfg(test)]
    mod corrupt {'
assert_absent "guard 7d: a ledger module names the debug-only assertion vocabulary. An invariant a build profile can compile out is not an invariant of the binary that produced a run (LEDG-10). Note that cfg(test) is a different predicate and is permitted — the corruption vocabulary uses it" \
    -nE "$COMPILED_OUT_PATTERN" "$BOOKS_SRC" "$INVARIANTS_SRC"

#     THE SCOPE ABOVE IS TWO FILES, AND LEDG-10'S CLAIM IS ABOUT THE INVARIANT
#     PATH. Phase 3 puts the tick loop that calls `CheckSet::run` in a new file
#     (src/world.rs, per the note at the top of src/books.rs). A Phase 3
#     `debug_assert!` around that call, or a `#[cfg(debug_assertions)]` on it
#     for a "fast release run", removes the invariant phase from the binary that
#     produces the actual run — and `overflow-checks = true` does NOT enable
#     `debug_assertions`, so the release profile would be silently unchecked.
#     The two-file list would say nothing.
#
#     Guard 7f faced the identical inheritance problem and handled it by
#     recording the extension as a ROADMAP success criterion. That is the weaker
#     instrument of the two: it is a promise a future reader has to keep. So the
#     scope is widened HERE instead, over every tracked source under src/, with
#     the one legitimate site carved out by file exactly as guard 7c carves out
#     src/rng.rs for RefCell. A new file that names the vocabulary fails this
#     guard on the commit that adds it, with no promise to remember.
#
#     THE WIDER CLAUSE STRIPS LINE COMMENTS, and the two-file clause above does
#     not. That difference is deliberate rather than an oversight. For the two
#     ledger modules the rule is the strict one the float-name rule in
#     tests/numeric_det.rs uses: the way to say "this is not a debug_assert" in
#     a doc comment is to not write the token. Across the rest of src/ that
#     would forbid src/numeric.rs from explaining, in prose, that its finiteness
#     check is deliberately NOT a `debug_assert!` — which is a comment worth
#     having. Stripping comments in the wider clause keeps the strict rule where
#     it is load-bearing and still catches the attribute anywhere.
DEBUG_GATE_FILES=""
for src_file in "${SRC_FILES[@]}"; do
    set +e
    printf '%s\n' "$(sed 's://.*::' "$src_file")" | grep -qE "$COMPILED_OUT_PATTERN"
    GREP_STATUS=$?
    set -e
    if [ "$GREP_STATUS" -gt 1 ]; then
        fail "guard 7d: could not search $src_file for the debug-only vocabulary (grep exit $GREP_STATUS)"
    fi
    if [ "$GREP_STATUS" -eq 0 ]; then
        DEBUG_GATE_FILES="$DEBUG_GATE_FILES$src_file "
    fi
done
if [ "$DEBUG_GATE_FILES" != "src/rng.rs " ]; then
    fail "guard 7d: the debug-only assertion vocabulary appears under src/ in [$DEBUG_GATE_FILES] — expected exactly src/rng.rs (the debug-only sub-stream re-entry guard, D-04, T-1-13). An invariant a build profile can compile out is not an invariant of the binary that produced a run (LEDG-10), and overflow-checks = true does not enable debug_assertions, so a release run would be silently unchecked. This scope covers every tracked source under src/ so that the file Phase 3 adds for the tick loop is guarded on the commit that adds it"
fi

# 7e. One read site for the liveness gate.
#
#     A second read would put the configuration back on a per-tick path and
#     reintroduce the scattered conditional the check-set-at-construction design
#     exists to remove.
#
#     Counted as reads of the KEY — the qualified field access — and not as
#     occurrences of the bare identifier. The local binding the one read site
#     initialises is used again a few lines later to filter the check table, and
#     that use is the design working rather than a second read. Line comments
#     are stripped first, so a doc comment naming the key neither satisfies nor
#     breaks the count.
set +e
GATE_FILES=$(grep -rlE 'liveness_enabled' -- "${SRC_FILES[@]}" | sort | tr '\n' ' ')
GREP_STATUS=$?
set -e
if [ "$GREP_STATUS" -gt 1 ]; then
    fail "guard 7e: could not search for the liveness key under src/ (grep exit $GREP_STATUS)"
fi
if [ "$GATE_FILES" != "src/config.rs src/invariants.rs " ]; then
    fail "guard 7e: the liveness key is named under src/ in [$GATE_FILES] — expected exactly src/config.rs (where it is declared) and src/invariants.rs (where it is read)"
fi
GATE_READ_PATTERN='\.liveness_enabled'
assert_fires 7e "$GATE_READ_PATTERN" 2 '        let enabled = params.invariants.liveness_enabled;
        if params.invariants.liveness_enabled { }'
assert_ignores 7e "$GATE_READ_PATTERN" '        let liveness_enabled = true;
        .filter(|(id, _, _)| liveness_enabled || *id != CheckId::Liveness)'
set +e
GATE_READS=$(printf '%s\n' "$INVARIANTS_CODE" | grep -cE "$GATE_READ_PATTERN")
GREP_STATUS=$?
set -e
if [ "$GREP_STATUS" -gt 1 ]; then
    fail "guard 7e: could not count reads of the liveness key (grep exit $GREP_STATUS)"
fi
if [ "$GATE_READS" -ne 1 ]; then
    fail "guard 7e: the liveness key is read $GATE_READS times in the production half of $INVARIANTS_SRC — expected exactly 1. The gate is decided once, when the check set is built; a second read puts the configuration back on the per-tick path"
fi

# 7f. Only the ledger writes a balance (LEDG-01).
#
#     The first half is a POSITIVE property about where a balance may be
#     written, and it is the half that carries the weight in this phase. The
#     second half — no function named set_cash anywhere under src/ — is the
#     WEAKER of the two: the agent types it is nominally about do not exist
#     until Phase 3, so it is a guard over a set that cannot yet contain the
#     thing it forbids.
#
#     THE INHERITED OBLIGATION IS **ROADMAP PHASE 3 SUCCESS CRITERION 7**, where
#     plan 02-01 recorded it: the commit that introduces `Household` and `Firm`
#     must extend this guard to name them. A reader planning Phase 3 reads the
#     roadmap criteria and never greps a lint script, which is why the
#     obligation lives there and is only pointed at from here.
mapfile -t NON_LEDGER_SRC < <(git ls-files -- 'src/*.rs' | grep -v "^${BOOKS_SRC}$")
if [ "${#NON_LEDGER_SRC[@]}" -eq 0 ]; then
    fail "guard 7f: found no tracked Rust sources under src/ other than $BOOKS_SRC — the search set is empty and the guard would pass trivially"
fi
BALANCE_PATTERN='\b(household_cash|firm_cash|household_stock|firm_stock|firm_headcount)\b'
assert_fires 7f "$BALANCE_PATTERN" 5 '    self.household_cash[index] = value;
    firm.firm_cash += wage;
    let held = self.household_stock[index];
    self.firm_stock[index] = units;
    self.firm_headcount[index] = count;'
assert_ignores 7f "$BALANCE_PATTERN" '    let total = books.firm_cash_total();
    let n = books.total_headcount();'
assert_absent "guard 7f: a file outside the ledger names a private balance identifier. Only $BOOKS_SRC may write a balance (LEDG-01). ROADMAP Phase 3 success criterion 7 carries the inherited obligation: the commit that introduces Household and Firm must extend this guard to name them" \
    -nE "$BALANCE_PATTERN" -- "${NON_LEDGER_SRC[@]}"
SET_CASH_PATTERN='fn[[:space:]]+set_cash[[:space:]]*[(<]'
assert_fires 7f-set_cash "$SET_CASH_PATTERN" 2 '    pub fn set_cash(&mut self, who: Account, value: Money) {}
    fn set_cash<T>(&mut self, value: T) {}'
assert_ignores 7f-set_cash "$SET_CASH_PATTERN" '    pub fn set_headcount(&mut self, slot: FirmSlot, count: u32) -> Option<u32> {
    fn write_cash(&mut self, slot: AccountSlot, value: Money) {'
assert_absent "guard 7f: a cash setter is declared under src/. The books own the quantity; there is nothing for an agent to set (LEDG-01). This is the weaker half of 7f — the types it is about arrive in Phase 3, see ROADMAP Phase 3 success criterion 7" \
    -rnE "$SET_CASH_PATTERN" -- "${SRC_FILES[@]}"

# 7f-agents. Household and Firm hold no balance (ROADMAP Phase 3 criterion 7).
#
#     THIS IS THE INHERITED OBLIGATION 7f ABOVE POINTS AT, discharged in the
#     same commit that introduced the two types. The half of 7f that searches
#     for a private balance identifier cannot see a field spelled `cash` on
#     `struct Household`, and neither can the set_cash half: the defect
#     criterion 7 is actually about matches neither pattern. 7f was silent on
#     the agent types not because they were clean but because it could not see
#     them.
mapfile -t AGENT_SRC < <(git ls-files -- 'src/world.rs')
if [ "${#AGENT_SRC[@]}" -eq 0 ]; then
    fail "guard 7f-agents: src/world.rs is not tracked — the guard would pass trivially"
fi

# (a) An agent type may not declare a money-TYPED field. Anchored to struct-field
#     syntax (trailing comma, line-anchored) so a legitimate local binding of type
#     Money is left alone — a pattern without that anchor fires on
#     `let held: Money = books.cash_of(a).unwrap();`, which was reproduced.
MONEY_FIELD='^[[:space:]]*(pub[[:space:]]+)?[a-z_]+[[:space:]]*:[[:space:]]*(crate::money::)?(Money|Cents)[[:space:]]*,[[:space:]]*$'
assert_fires 7f-agents "$MONEY_FIELD" 4 '    pub cash: Money,
    balance: Money,
    pub wallet : Cents,
    pub hoard: crate::money::Money,'
assert_ignores 7f-agents "$MONEY_FIELD" '    pub price_cents: i64,
    pub id: HouseholdId,
    fn cash_of(&self, a: Account) -> Option<Money> {
    let held: Money = books.cash_of(a).unwrap();
    let m: Money = Money::ZERO;'
assert_absent "guard 7f-agents: an agent type in src/world.rs declares a money-typed field. The books own every cent (LEDG-01, ROADMAP Phase 3 criterion 7)" \
    -nE "$MONEY_FIELD" -- "${AGENT_SRC[@]}"

# (b) A balance-SHAPED field name, whatever its type. An i64 named `cash` evades
#     (a) entirely and is the same defect.
BALANCE_NAME='^[[:space:]]*(pub[[:space:]]+)?(cash|balance|funds|wallet|savings|deposits|inventory|stock|units_held|headcount|employees)[[:space:]]*:'
assert_fires 7f-agents-name "$BALANCE_NAME" 4 '    pub cash: i64,
    balance: u64,
    pub inventory: i64,
    headcount: u32,'
assert_ignores 7f-agents-name "$BALANCE_NAME" '    pub price_cents: i64,
    pub id: HouseholdId,
    let cash: Money = books.cash_of(a).unwrap();
    fn cash(&self) -> Money {'
assert_absent "guard 7f-agents: an agent type in src/world.rs declares a balance-shaped field. The books own the quantity" \
    -nE "$BALANCE_NAME" -- "${AGENT_SRC[@]}"

# (c) The types this criterion is about must actually EXIST here, or (a) and (b)
#     are guards over nothing — the same "pin the probe to a declaration count"
#     discipline guard 7j already uses.
for t in Household Firm; do
    grep -qE "^(pub )?struct $t\b" "${AGENT_SRC[@]}" \
      || fail "guard 7f-agents: struct $t is not declared in src/world.rs — the guard polices a set that does not contain the types criterion 7 names"
done

# 7g. No accessor hands out the mutation point.
#
#     A function returning a mutable reference gives the caller the very
#     mutation point transfer and the three goods operations exist to
#     monopolise, and NO search for a setter NAME would find it. This guard is
#     on the return type, deliberately. Line comments are stripped first so the
#     doc comment stating the rule does not trip it.
MUT_RETURN_PATTERN='->.*&[^,;)]*mut[[:space:]]'
assert_fires 7g "$MUT_RETURN_PATTERN" 3 '    pub fn cash_mut(&mut self, who: Account) -> &mut Money { }
    pub fn journal_mut(&mut self) -> Option<&mut Vec<Posting>> { }
    pub fn slot_mut(&mut self) -> &'"'"'a mut Money { }'
assert_ignores 7g "$MUT_RETURN_PATTERN" '    pub fn journal(&self) -> &[Posting] {
    fn write_cash(&mut self, slot: AccountSlot, value: Money) {
    pub fn set_headcount(&mut self, slot: FirmSlot, count: u32) -> Option<u32> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'"'"'_>) -> std::fmt::Result {'
assert_absent_in "guard 7g: a function in the ledger returns a mutable reference, handing the caller the mutation point the transfer exists to monopolise (LEDG-01, LEDG-02). No search for a setter name would find this, which is why the guard is on the return type" \
    "$MUT_RETURN_PATTERN" "$BOOKS_CODE"

# 7h. No environment in a halt message.
#
#     A violation's rendered form reaches standard error and is read beside a
#     diffed log. A path, a host name, a wall-clock reading or a process
#     identifier in it breaks the determinism rule before it is an
#     information-disclosure question (TICK-06, T-02-39).
#
#     The runtime half of this rule is the no-path assertion in plan 02-05's
#     message tests; this is the source half, and neither is sufficient alone.
#     Scoped to the production half of each file: the unit-test modules below
#     legitimately load the shipped configuration from a path.
#
#     BOTH LEDGER MODULES ARE SEARCHED, and the second is not optional. Every
#     `Violation` variant that carries a posting interpolates it through
#     `render_posting`, which formats it with `impl Display for Posting` — and
#     that impl lives in src/books.rs. Searching src/invariants.rs alone covers
#     the outer half of the string and leaves the inner half unguarded, which is
#     where roughly half of every halt message is actually rendered.
#
#     THE RULE, PLAINLY, NOW THAT THERE ARE TWO PATTERNS. The FULL pattern below
#     applies to every module that can render a halt message: the two ledger
#     modules, plus the pipeline and the agent module plan 03-02 added. The
#     NARROWED pattern of guard 7h-log applies to the one module whose job is to
#     hold a path, and it is narrower in exactly ONE alternative — the path type
#     — and only for as long as that module renders no violation at all. That
#     last condition is asserted, not assumed; see 7h-log.
#
#     WHY THE PIPELINE AND AGENT MODULES JOINED THE FULL SET. Both can carry a
#     violation to the caller that renders it, and neither holds a filesystem
#     path, so the full pattern reaches them unchanged. Without them, a
#     violation rendered through a NEW implementation in either would put a
#     path, a clock reading or a process identifier into a halt message with
#     nothing watching (T-03-28) — the source half of TICK-06, whose runtime
#     half is the stderr assertion in tests/determinism.rs.
#
#     A NOTE ON WHY THIS GREP IS THE WHOLE ENFORCEMENT for the process id.
#     `std::process::id` is not in clippy.toml's disallowed-methods, and adding
#     it makes the clean tree fail check 1: tests/config_strict.rs:275 and
#     tests/tracer_end_to_end.rs:33 both call it to build a unique temporary
#     path, which is legitimate test scaffolding and neither on the behaviour
#     path nor in a halt message. Verified by adding the entry — `error: use of
#     a disallowed method `std::process::id` --> tests/config_strict.rs:275`.
#     (The second line reference read :21 until plan 03-06 remeasured it; plan
#     03-02's edits to that file moved the call, and a stale citation in a
#     comment explaining why a lint entry was DECLINED is how the reasoning
#     stops being checkable.)
#     Check 4b forbids the `#[allow(...)]` that would silence it, so there is no
#     legal escape. Same class of exclusion as the RefCell and Arc entries
#     clippy.toml documents, and handled the same way: the lint entry is
#     declined and this source guard carries the rule.
ENVIRONMENT_PATTERN='env::|env!|(^|[^A-Za-z0-9_])Path(Buf)?[^A-Za-z0-9_]|SystemTime|Instant|std::process'
assert_fires 7h "$ENVIRONMENT_PATTERN" 7 '        let home = std::env::var("HOME");
        let dir = env!("CARGO_MANIFEST_DIR");
        let p = Path::new("/tmp");
        let p: PathBuf = dir.into();
        let now = SystemTime::now();
        let t = Instant::now();
        let pid = std::process::id();'
assert_ignores 7h "$ENVIRONMENT_PATTERN" '    let path_of_least_resistance = 1;
    let instant = 1;
    write!(f, "tick {tick}: {posting}")'
BOOKS_PRODUCTION=$(production_source "$BOOKS_SRC")
PHASES_PRODUCTION=$(production_source "$PHASES_SRC")
WORLD_PRODUCTION=$(production_source "$WORLD_SRC")
for pair in "$BOOKS_SRC:$BOOKS_PRODUCTION" "$PHASES_SRC:$PHASES_PRODUCTION" "$WORLD_SRC:$WORLD_PRODUCTION"; do
    [ -n "${pair#*:}" ] || fail "guard 7h: the production half of ${pair%%:*} is empty — a guard over an empty set passes trivially"
done
assert_absent_in "guard 7h: one of the four modules that can render a halt message ($INVARIANTS_SRC, $BOOKS_SRC, $PHASES_SRC, $WORLD_SRC) names a path, clock or process type. A halt message carries integers and identities only — a wall-clock reading, a process id or a path in it breaks determinism before it is an information-disclosure question (TICK-06). The two ledger modules are both searched because a violation renders its posting through impl Display for Posting, which lives in $BOOKS_SRC; the other two joined the set because either can carry a violation to the code that renders it (T-03-28)" \
    "$ENVIRONMENT_PATTERN" "$INVARIANTS_PRODUCTION
$BOOKS_PRODUCTION
$PHASES_PRODUCTION
$WORLD_PRODUCTION"

# 7h-log. The same rule for the module that legitimately holds a path — narrowed
#         in exactly one alternative, and bounded so the narrowing cannot widen.
#
#         A CLAUSE OF 7h, not a guard of its own: no requirement or roadmap
#         criterion names it, so the count above does not move. See the
#         counting rule in this check's preamble.
#
#         src/log.rs writes every diffed byte, and the run-directory writer
#         holds a filesystem path because that is its entire job. The full
#         pattern would block the module outright; applying nothing would leave
#         the environment unguarded in the one module that touches the disk. So
#         the narrowed pattern keeps the clock types, the environment accessor,
#         the environment macro and the process type, and drops ONLY the path
#         alternative. Diff the two patterns: that one alternative is the whole
#         difference.
#
#         AND THE NARROWING IS BOUNDED. It is legitimate only while no halt
#         message is rendered in this module — plan 03-02's recorded decision,
#         which declined a violation record in the log module for exactly this
#         reason: "narrowing a guard to accommodate new code is how a guard
#         stops meaning anything". The second assertion below is what keeps
#         those two facts coupled. If a later phase decides to emit a violation
#         record into the event stream, it fails and puts the decision back in
#         front of a human instead of the hole quietly widening (T-03-29).
NARROWED_ENVIRONMENT_PATTERN='env::|env!|SystemTime|Instant|std::process'
assert_fires 7h-log "$NARROWED_ENVIRONMENT_PATTERN" 5 '        let home = std::env::var("HOME");
        let dir = env!("CARGO_MANIFEST_DIR");
        let now = SystemTime::now();
        let t = Instant::now();
        let pid = std::process::id();'
assert_ignores 7h-log "$NARROWED_ENVIRONMENT_PATTERN" '        let p = Path::new("/tmp");
        pub fn new(dir: &Path) -> io::Result<RunWriter> {
        let p: PathBuf = base.join("ticks.csv");
        use std::path::Path;'
LOG_PRODUCTION=$(production_source "$LOG_SRC")
[ -n "$LOG_PRODUCTION" ] || fail "guard 7h-log: the production half of $LOG_SRC is empty — a guard over an empty set passes trivially"
assert_absent_in "guard 7h-log: $LOG_SRC names a clock, the environment or the process type. This module is exempt from guard 7h's PATH alternative only — the run-directory writer holds a path by construction — and from nothing else (TICK-06)" \
    "$NARROWED_ENVIRONMENT_PATTERN" "$LOG_PRODUCTION"

# The bound on that narrowing. Line comments stripped first, so a doc comment
# stating this very rule does not trip it — the same treatment guards 7d and 7g
# give their own patterns.
VIOLATION_TYPE_PATTERN='\bViolation\b'
assert_fires 7h-log "$VIOLATION_TYPE_PATTERN" 2 '    fn render(v: &Violation) -> String {
        Violation::Conservation { tick, .. } => {}'
assert_ignores 7h-log "$VIOLATION_TYPE_PATTERN" '        let violations = 0;
        struct ViolationCount(u32);
        fn invariant_violations() -> u32 { 0 }'
LOG_PRODUCTION_CODE=$(printf '%s\n' "$LOG_PRODUCTION" | sed 's://.*::')
assert_absent_in "guard 7h-log: $LOG_SRC renders or names a Violation, and guard 7h-log's narrowed pattern is only honest while it does not. Plan 03-02 DECIDED to keep the halt message out of the module that holds a filesystem path, and this assertion is what couples that decision to the narrowing it justifies. If emitting a violation record here is now deliberate, that decision has to be reopened — widen the pattern back to guard 7h's, or move the rendering out (T-03-29)" \
    "$VIOLATION_TYPE_PATTERN" "$LOG_PRODUCTION_CODE"


# 7i. Every fault-injection method is behind the test gate (LEDG-10).
#
#     Check 6 executes the boundary from the outside — the probe calls all four
#     corruption methods from tests/ and the build is refused with E0599. This
#     is the source half, and it guards the case the probe cannot: a FIFTH
#     method added outside the `#[cfg(test)] impl Books` block. The probe names
#     the four that exist; it cannot name one nobody has written yet, and the
#     four it does name would still fail to resolve, so check 6 would still be
#     green while a method that writes state the public API cannot reach had
#     shipped in the library.
#
#     Counted two ways and compared. The first counts every `corrupt_*`
#     DECLARATION in the file, with line comments stripped so a doc comment
#     naming one does not inflate it. The second counts only those inside a
#     block opened by a column-zero `#[cfg(test)]` and closed by a column-zero
#     `}`. A declaration outside the gate makes the two disagree.
CORRUPT_DECL_PATTERN='fn[[:space:]]+corrupt_[A-Za-z0-9_]+'
assert_fires 7i "$CORRUPT_DECL_PATTERN" 3 '    pub(crate) fn corrupt_silent_cash(&mut self, who: Account, delta_cents: i64) {
    pub fn corrupt_appended_posting(&mut self, draft: Posting) -> Posting {
    fn corrupt_stock(&mut self, slot: AccountSlot, units: i64) {'
assert_ignores 7i "$CORRUPT_DECL_PATTERN" '        books.corrupt_silent_cash(household(0), -1);
    fn corrupted(&self) -> bool { false }'
set +e
CORRUPT_COUNT=$(printf '%s\n' "$BOOKS_CODE" | grep -cE "$CORRUPT_DECL_PATTERN")
GREP_STATUS=$?
set -e
if [ "$GREP_STATUS" -gt 1 ]; then
    fail "guard 7i: could not count the fault-injection declarations (grep exit $GREP_STATUS)"
fi
if [ "$CORRUPT_COUNT" -eq 0 ]; then
    fail "guard 7i: $BOOKS_SRC declares no corrupt_* method — the guard has nothing to protect and the negative tests in src/invariants.rs have nothing to seed a fault with"
fi
GATED_COUNT=$(awk '/^#\[cfg\(test\)\]/{g=1} /^}/{g=0} g && /fn[[:space:]]+corrupt_/{n++} END{print n+0}' "$BOOKS_SRC")
if [ "$CORRUPT_COUNT" -ne "$GATED_COUNT" ]; then
    fail "guard 7i: $BOOKS_SRC declares $CORRUPT_COUNT corrupt_* methods but only $GATED_COUNT of them sit inside a #[cfg(test)] block. Every method that writes state the public API cannot reach must be invisible to consumers of sim (LEDG-10); check 6 proves the four that exist are refused from tests/, and this is what stops a fifth being added outside the gate"
fi

# 7j. The probe calls every one of them.
#
#     Check 6 asserts the build fails with E0599, which is a claim about the
#     methods the probe NAMES. If the probe names three of four, the fourth can
#     leave the test configuration with check 6 still green.
set +e
PROBE_CALLS=$(grep -coE 'books\.corrupt_[A-Za-z0-9_]+\(' "$CFG_PROBE_SRC")
GREP_STATUS=$?
set -e
if [ "$GREP_STATUS" -gt 1 ]; then
    fail "guard 7j: could not count the probe's corruption calls (grep exit $GREP_STATUS)"
fi
if [ "$PROBE_CALLS" -ne "$CORRUPT_COUNT" ]; then
    fail "guard 7j: $CFG_PROBE_SRC calls $PROBE_CALLS corruption methods but $BOOKS_SRC declares $CORRUPT_COUNT. Check 6's E0599 assertion only covers the methods the probe names, so an unnamed one could leave the #[cfg(test)] block with check 6 still reporting success"
fi

echo "  check 7: eleven source guards (7a callbacks, 7b/7c shared mutability, 7d compiled-out invariants, 7e one gate read, 7f/7f-agents/7g balance writes, 7h halt-message environment across all four modules that can render one — $INVARIANTS_SRC, $BOOKS_SRC, $PHASES_SRC, $WORLD_SRC — with clause 7h-log covering $LOG_SRC under a pattern narrowed in the path alternative alone and bounded by an assertion that it renders no Violation, 7i/7j the fault-injection gate), each proved to fire on a hazard fixture first"

echo "OK: the lint gate blocks a hashed collection and a banned float method in tests/, all $FIRED resolvable method bans (floats + the clock) fire, no alias, exemption or non-portable generator escapes it, both compile-fail probes are refused with the exact diagnostic they name, and eleven source guards are silent on a tree each of them was first watched firing on — 7h over the four halt-message modules, its 7h-log clause over the path-holding writer"
