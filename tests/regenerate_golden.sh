#!/usr/bin/env bash
#
# TICK-09, the review half: regenerate the committed golden run.
#
# `the_golden_run_reproduces` in tests/determinism.rs compares a fresh run
# against the bytes committed under tests/golden/. This script is the ONLY thing
# that writes those bytes. The separation is the whole point: a test that
# regenerated the artifact and then compared it would be comparing the generator
# with itself and would pass however far the economy drifted. Same discipline
# this repository already applies to clippy.toml and to schema/schema.json, and
# the command is named in the failing test's own message so a reader whose
# change was deliberate does not go looking for it — or, worse, hand-edit the
# committed bytes.
#
# THE CONFIGURATION IS DERIVED, NEVER COMMITTED. A second baseline.toml under
# tests/ would drift from the shipped one and would then certify a configuration
# nobody runs. So this script reads the shipped file and moves ONE leaf, the
# tick count, exactly as tests/determinism.rs does for the liveness leaf. Every
# other parameter of the golden run — including the seed — is the shipped
# parameter, which is what makes a deliberate parameter change show up here as a
# reviewable diff of the economy.
#
# The count assertions below are not decoration. With a reworded configuration a
# blind substitution is a silent no-op, the run would be a full decade, and the
# golden comparison would fail with a confusing message about file length
# instead of a clear one about the substitution.
#
# THE RUN RECORD IS EXCLUDED. run_meta.json carries the compiler version string
# and a wall clock; committing it would make a toolchain bump look like a change
# to the economy. It is excluded from the determinism diff by the same locked
# decision (EXCLUDED_FROM_DIFF in tests/determinism.rs), and this script
# excludes it by that rule rather than by copying a list of three names — a new
# log file a later phase adds flows into the golden run automatically and gets
# reviewed, instead of silently having no regression signal.
#
# No error-suppressing fallbacks anywhere: a missing input must surface as a
# failure, never default into a successful-looking regeneration.

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

SHIPPED="config/baseline.toml"
GOLDEN="tests/golden"

# The one leaf that moves, and what it becomes. Fifty ticks: see
# tests/golden/README.md for both reasons, one of which is a floor.
SHIPPED_LEAF="ticks = 3650"
GOLDEN_LEAF="ticks = 50"

# The single quarantined file, spelled once. Keep in step with
# EXCLUDED_FROM_DIFF in tests/determinism.rs.
EXCLUDED="run_meta.json"

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

# grep exits 0 for a match, 1 for "no match" and 2 for a real error. Counting
# under `set -e` without this guard would abort on a legitimate zero count and
# report nothing about why.
count_in() {
    local pattern="$1" file="$2" hits status
    set +e
    hits=$(grep -cE -e "$pattern" "$file")
    status=$?
    set -e
    if [ "$status" -gt 1 ]; then
        fail "could not search $file for $pattern (grep exit $status)"
    fi
    printf '%s' "$hits"
}

[ -f "$SHIPPED" ] || fail "$SHIPPED is missing — there is nothing to derive the golden configuration from"
[ -s "$SHIPPED" ] || fail "$SHIPPED is empty — the substitution below would be a no-op"

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT INT TERM

# --- 1. Derive the golden configuration by moving one leaf ----------------

LEAF_HITS=$(count_in "^${SHIPPED_LEAF}\$" "$SHIPPED")
if [ "$LEAF_HITS" -ne 1 ]; then
    fail "expected exactly one '${SHIPPED_LEAF}' leaf in $SHIPPED, found $LEAF_HITS — the shipped configuration was reworded and this substitution would have been a silent no-op, regenerating a full decade instead of the golden window"
fi

CONFIG="$WORK/golden.toml"
sed "s/^${SHIPPED_LEAF}\$/${GOLDEN_LEAF}/" "$SHIPPED" > "$CONFIG"

NEW_HITS=$(count_in "^${GOLDEN_LEAF}\$" "$CONFIG")
if [ "$NEW_HITS" -ne 1 ]; then
    fail "the substitution did not put '${GOLDEN_LEAF}' back exactly once (found $NEW_HITS)"
fi
OLD_HITS=$(count_in "^${SHIPPED_LEAF}\$" "$CONFIG")
if [ "$OLD_HITS" -ne 0 ]; then
    fail "the derived configuration still holds '${SHIPPED_LEAF}'"
fi

# One leaf moved and nothing else: the same two shape assertions the liveness
# override in tests/determinism.rs makes, for the same reason. The GRADE
# comments are what distinguish a textual override from a re-serialisation, and
# tests/provenance.rs makes them load-bearing.
if [ "$(wc -l < "$CONFIG")" -ne "$(wc -l < "$SHIPPED")" ]; then
    fail "the derived configuration changed the shape of the file, not one leaf in it"
fi
if [ "$(count_in '^# GRADE:' "$CONFIG")" -ne "$(count_in '^# GRADE:' "$SHIPPED")" ]; then
    fail "the derived configuration lost a source grade — it was a re-serialisation, not a textual substitution"
fi

# --- 2. Run it -------------------------------------------------------------
#
# No --seed: the seed is a shipped parameter like any other, so the golden run
# and the shipped configuration tell one story and a deliberate seed change
# shows up here as a diff rather than being overridden away.

cargo build --locked --quiet
RUN="$WORK/run"
./target/debug/sim --config "$CONFIG" --out "$RUN"

[ -d "$RUN" ] || fail "the run produced no output directory"

# --- 3. Copy every diffed file, and only those ----------------------------

mkdir -p "$GOLDEN"

COPIED=0
for produced in "$RUN"/*; do
    [ -f "$produced" ] || fail "$produced is not a regular file"
    name=$(basename "$produced")
    if [ "$name" = "$EXCLUDED" ]; then
        continue
    fi
    [ -s "$produced" ] || fail "the run wrote an empty $name — committing it would make the golden comparison hash two empty files and prove nothing"
    cp "$produced" "$GOLDEN/$name"
    COPIED=$((COPIED + 1))
done

if [ "$COPIED" -lt 3 ]; then
    fail "copied only $COPIED file(s) into $GOLDEN — the run directory should hold at least the tick series, the event stream and the provenance table"
fi
if [ -e "$GOLDEN/$EXCLUDED" ]; then
    fail "$GOLDEN/$EXCLUDED exists — the run record carries the compiler version string and a wall clock, and committing it would make a toolchain bump look like a change to the economy"
fi

TICK_LINES=$(wc -l < "$GOLDEN/ticks.csv")
if [ "$TICK_LINES" -ne 51 ]; then
    fail "$GOLDEN/ticks.csv has $TICK_LINES lines, expected 51 (a header plus 50 ticks) — the tick-count substitution did not reach the run"
fi

echo "OK: regenerated $COPIED file(s) in $GOLDEN from $SHIPPED with '${SHIPPED_LEAF}' -> '${GOLDEN_LEAF}'; $EXCLUDED excluded"
echo "    Review the diff before committing: git diff -- $GOLDEN"
