#!/usr/bin/env bash
#
# TICK-02, second half: the schema drift test as an OBSERVED failure, not as a
# test that exists.
#
# `schema_matches_the_committed_file` compares the generator against
# `schema/schema.json`. A drift test that has never been seen to fail is
# indistinguishable from one comparing a file with itself — which is exactly
# what a schema-derive crate would have produced here, since the derive and the
# generated file would have been wrong in the same way and agreed forever. So
# this script does not check that the test exists; it perturbs the committed
# artifact, watches the test block, restores the file and watches it pass again.
#
# It is a shell script rather than a libtest case for the same reason
# tests/lints.sh is: it must assert that a test run FAILS, which libtest cannot
# express.
#
# THE PERTURBATION IS A COLUMN SWAP, not a corruption. Swapping two adjacent
# column entries leaves valid JSON and is exactly the realistic defect — a
# reordered column, which the analysis side reads positionally. A syntactically
# broken file would prove only that the test reads a file.
#
# The restore runs under a `trap`, and its result is CHECKED against a digest
# taken before the perturbation. A negative test that leaves a mutated artifact
# in the working tree has poisoned the thing it was meant to protect, and a
# mutated schema is exactly the kind of file that gets committed by accident.
#
# No error-suppressing fallbacks anywhere: a missing input must surface as a
# failure, never default into a passing comparison.

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

SCHEMA="schema/schema.json"
BACKUP="target/schema-drift-negative.backup"
DRIFT_TEST=(cargo test --locked --test log_schema schema_matches_the_committed_file)

# The column swapped. Adjacent, and neither is the last entry of its array, so
# the swap moves two identical line shapes and the result is still valid JSON.
SWAP_AT='"name": "total_money_cents"'

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

# Digest of a file's bytes. Two spellings, because neither is universal; a
# missing digest tool is a failure rather than a skipped comparison.
digest() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | cut -d' ' -f1
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | cut -d' ' -f1
    else
        fail "no sha256 tool found, so the restore could not be verified"
    fi
}

[ -f "$SCHEMA" ] || fail "$SCHEMA is missing — the drift test has nothing to compare against"

mkdir -p "$(dirname "$BACKUP")"
cp "$SCHEMA" "$BACKUP"
ORIGINAL=$(digest "$SCHEMA")

# Restore on EVERY exit path, including a failed check and an interrupt.
restore() {
    if [ -f "$BACKUP" ]; then
        cp "$BACKUP" "$SCHEMA"
    fi
}
cleanup() {
    restore
    rm -f "$BACKUP"
}
trap cleanup EXIT INT TERM

# --- 1. Perturb: swap two adjacent column entries -------------------------

awk -v marker="$SWAP_AT" '
    { line[NR] = $0 }
    END {
        at = 0
        for (i = 1; i <= NR; i++) {
            if (index(line[i], marker) > 0) { at = i }
        }
        if (at == 0 || at >= NR) { exit 3 }
        swap = line[at]; line[at] = line[at + 1]; line[at + 1] = swap
        for (i = 1; i <= NR; i++) print line[i]
    }
' "$BACKUP" > "$SCHEMA" || fail "could not find the column to swap ($SWAP_AT) in $SCHEMA"

if cmp -s "$BACKUP" "$SCHEMA"; then
    fail "the perturbation changed nothing, so the check below would prove nothing"
fi

# --- 2. The drift test must FAIL, and for the stated reason ---------------

set +e
PERTURBED_OUTPUT=$("${DRIFT_TEST[@]}" 2>&1)
PERTURBED_STATUS=$?
set -e

if [ "$PERTURBED_STATUS" -eq 0 ]; then
    fail "the drift test PASSED against a perturbed schema — it is not comparing what it claims to compare"
fi
case "$PERTURBED_OUTPUT" in
    *"schema drift at line"*) ;;
    *) fail "the drift test failed, but not with the drift diagnostic — it may have failed to build. Output:
$PERTURBED_OUTPUT" ;;
esac

echo "OK: the drift test was observed to FAIL on the perturbed schema (adjacent column swap), exit $PERTURBED_STATUS"

# --- 3. Restore, verified by digest ---------------------------------------

restore
RESTORED=$(digest "$SCHEMA")
if [ "$RESTORED" != "$ORIGINAL" ]; then
    fail "the restore did not return $SCHEMA to its original bytes ($ORIGINAL vs $RESTORED) — the working tree is left mutated"
fi

# --- 4. And the test must pass again, having actually run -----------------

set +e
RESTORED_OUTPUT=$("${DRIFT_TEST[@]}" 2>&1)
RESTORED_STATUS=$?
set -e

if [ "$RESTORED_STATUS" -ne 0 ]; then
    fail "the drift test does not pass on the restored schema. Output:
$RESTORED_OUTPUT"
fi
# A filter that matches nothing exits 0 and reports `0 passed`, which would make
# a renamed test look exactly like a passing one.
case "$RESTORED_OUTPUT" in
    *"1 passed"*) ;;
    *) fail "the drift test ran no tests — the name filter matches nothing, so both checks above proved nothing. Output:
$RESTORED_OUTPUT" ;;
esac

echo "OK: the drift test was observed to PASS again after the restore, with the file byte-identical to its committed bytes"
