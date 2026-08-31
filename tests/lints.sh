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
#   7. eight source guards, each proved to fire first            (LEDG-01/02/10)
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

# The two ledger modules checks 7a–7h are written against.
BOOKS_SRC="src/books.rs"
INVARIANTS_SRC="src/invariants.rs"

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

for f in "$PROBE_SRC" "$HAZARD_SRC" "$BORROW_PROBE_SRC" "$CFG_PROBE_SRC" \
         "$BOOKS_SRC" "$INVARIANTS_SRC" clippy.toml; do
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
assert_absent "a file carries a lint exemption for a determinism ban" \
    -En '#!?\[(allow|expect)\((warnings|clippy::(all|disallowed_types|disallowed_methods))' \
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
    fail "a shared borrow of the books held live across a call to transfer COMPILED — LEDG-02 leg 1 does not hold, or the probe's trailing use of the borrow was tidied away so the borrow no longer spans the mutation"
fi
case "$BORROW_OUT" in
    *"E0502"*) ;;
    *) fail "the borrow probe failed to build but produced no E0502 — it broke for an unrelated reason (a renamed constructor, a changed signature), so this check is reporting nothing about the borrow rule" ;;
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
    fail "an integration test CALLED a fault-injection method on Books — the corruption vocabulary has escaped the crate's own test configuration and is reachable by any consumer of sim"
fi
case "$CFG_OUT" in
    *"E0599"*) ;;
    *) fail "the reachability probe failed to build but produced no E0599 — it broke for an unrelated reason, so this check is reporting nothing about the fault-injection boundary" ;;
esac
echo "  check 6: the fault-injection vocabulary is refused from tests/ with E0599"

echo "OK: the lint gate blocks a hashed collection and a banned float method in tests/, all $FIRED resolvable method bans (floats + the clock) fire, no alias, exemption or non-portable generator escapes it, and both compile-fail probes are refused with the exact diagnostic they name"
