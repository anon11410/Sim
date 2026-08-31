#!/usr/bin/env bash
#
# CORE-09: the reproducibility contract, as checkable facts rather than
# convention. This is a build-tooling assertion, not a behaviour test —
# libtest cannot express "this file is tracked by git".
#
# No error-suppressing fallbacks anywhere: a missing input must surface as a
# failure, never default to a passing comparison.

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

# 1. The lockfile and the toolchain pin are tracked by git.
#    Both are load-bearing: the lockfile pins crate versions whose patch
#    releases may legally change a sampling algorithm, and the toolchain pin
#    fixes the compiler whose codegen the logs depend on.
git ls-files --error-unmatch Cargo.lock rust-toolchain.toml > /dev/null

# The dependency GRAPH, not the manifest. `--edges normal` is the
# library-and-binary graph: exactly what ships and what the simulation runs on.
# Computed once here because checks 2 and 4 both search it.
TREE=$(cargo tree --edges normal)

# 2. No data-parallelism dependency. Thread interleaving is nondeterministic;
#    the simulation is single-threaded by requirement.
#
#    Searched in the graph rather than in Cargo.toml. The previous
#    line-anchored `^[[:space:]]*rayon[[:space:]]*=` grep matched only the
#    inline dependency form at the start of a line: it missed the
#    `[dependencies.rayon]` table form, and — far more importantly — it missed
#    any TRANSITIVE rayon, which is the realistic way data parallelism enters a
#    dependency graph. Check 4 below already used the graph for getrandom and
#    explained why; this check now follows its own neighbour.
if echo "$TREE" | grep -Eq '(^|[^a-z-])rayon( |$|v)'; then
    fail "a data-parallelism crate (rayon) is reachable from the behaviour path"
fi

# 3. No per-machine codegen tuning. `-C target-cpu=native` changes codegen per
#    machine and destroys "same source => same log across machines".
#    The absence of .cargo/config.toml is itself the guarantee.
if [ -e .cargo/config.toml ] || [ -e .cargo/config ]; then
    fail ".cargo/config.toml exists — per-machine codegen override is possible"
fi

# Only build configuration can actually set the flag, so scope the search to
# those files rather than to prose that merely names it. The globs also cover
# CI workflow files that later plans add.
mapfile -t BUILD_CONFIG_FILES < <(git ls-files -- \
    'Cargo.toml' \
    'rust-toolchain.toml' \
    '.cargo/*' \
    '**/.cargo/*' \
    '.github/workflows/*')
if [ "${#BUILD_CONFIG_FILES[@]}" -eq 0 ]; then
    fail "found no build-configuration files to search — expected at least Cargo.toml"
fi

# grep exits 0 for a match, 1 for "no match" (a pass) and 2 for a real error
# (a fail). Collapsing 1 and 2 with `|| true` would let an unreadable file
# default to a passing comparison. `xargs` cannot be used here: it reports its
# own 123 for any non-zero child status, erasing that distinction.
set +e
OFFENDERS=$(grep -l 'target-cpu' -- "${BUILD_CONFIG_FILES[@]}")
GREP_STATUS=$?
set -e
if [ "$GREP_STATUS" -gt 1 ]; then
    fail "could not search build-configuration files for target-cpu (grep exit $GREP_STATUS)"
fi
if [ -n "$OFFENDERS" ]; then
    fail "these build-configuration files set target-cpu: $(echo "$OFFENDERS" | tr '\n' ' ')"
fi

# 4. No OS-entropy crate on the behaviour path. `getrandom` does appear under
#    the proptest DEV-dependency (proptest seeds its own generator and tempfile
#    names its own directories), and that is not a path the simulation can
#    reach — asserting over the full tree would be asserting something false.
#    `$TREE` is computed above check 2.
if echo "$TREE" | grep -q 'getrandom'; then
    fail "an OS-entropy crate (getrandom) is reachable from the behaviour path"
fi

# 4b. The release profile cannot silently wrap. This is the single most
#     load-bearing line in Cargo.toml (CORE-02 / D-10): verified in research
#     that a DEFAULT release build wrapped `i64::MAX - 1 + 6` to
#     -9223372036854775804, a plausible negative balance that a conservation
#     audit would report as a real number.
#
#     It was previously protected only by
#     `tests/tracer_end_to_end.rs::raw_i64_overflow_panics_when_overflow_checks_are_on`,
#     which under a plain `cargo test` is vacuous: the `test` profile inherits
#     `dev`, where overflow-checks is already on by default. Deleting the
#     setting left that test green in the debug run. This script — whose stated
#     job is the reproducibility contract as checkable facts — did not check it
#     at all, while checking four less critical facts.
#
#     Matched with awk rather than `grep -Pzo`, which needs a PCRE-enabled grep
#     that is not present everywhere. The scan is stateful over the profile
#     section so it cannot be satisfied by an `overflow-checks` line belonging
#     to some other profile.
if ! awk '
    /^[[:space:]]*\[/ { in_release = ($0 ~ /^[[:space:]]*\[profile\.release\][[:space:]]*$/) }
    in_release && /^[[:space:]]*overflow-checks[[:space:]]*=[[:space:]]*true[[:space:]]*$/ { found = 1 }
    END { exit(found ? 0 : 1) }
' Cargo.toml; then
    fail "[profile.release] does not set overflow-checks = true (CORE-02 / D-10)"
fi

# 5. The toolchain pin names the verified channel.
if ! grep -Eq '^[[:space:]]*channel[[:space:]]*=[[:space:]]*"1\.94\.1"' rust-toolchain.toml; then
    fail "rust-toolchain.toml does not pin channel 1.94.1"
fi

echo "OK: lockfile and toolchain tracked, no data-parallelism crate in the graph, no codegen override, no OS-entropy crate on the behaviour path, release profile checks overflow"
