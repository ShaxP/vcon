#!/usr/bin/env bash
set -u

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

RUN_DATE="$(date +%F)"
RUN_TS="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
REPORT_PATH="${1:-docs/reports/release-readiness-${RUN_DATE}.md}"
STEAM_REPORT_PATH="${2:-docs/reports/steam-deck-validation-${RUN_DATE}.md}"

mkdir -p "$(dirname "$REPORT_PATH")"
mkdir -p "$(dirname "$STEAM_REPORT_PATH")"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

CATEGORIES=(
  sandbox
  determinism
  render
  input
  physics
  storage
  packaging
  audio
)

category_command() {
  case "$1" in
    sandbox)
      printf '%s' "cargo test -p vcon-engine sandbox -- --nocapture && cargo test -p vcon-runtime --test sandbox_bypass_regression -- --nocapture"
      ;;
    determinism)
      printf '%s' "cargo test -p vcon-runtime --test determinism_replay -- --nocapture && cargo test -p vcon-runtime --test physics_box2d_replay -- --nocapture"
      ;;
    render)
      printf '%s' "cargo test -p vcon-runtime --test render_golden -- --nocapture && cargo test -p vcon-runtime --test runtime_smoke -- --nocapture"
      ;;
    input)
      printf '%s' "cargo test -p vcon-runtime --test controller_hotplug -- --nocapture && cargo test -p vcon-runtime --test input_diagnostics_smoke -- --nocapture"
      ;;
    physics)
      printf '%s' "cargo test -p vcon-engine physics -- --nocapture && cargo test -p vcon-runtime --test physics_demo_smoke -- --nocapture"
      ;;
    storage)
      printf '%s' "cargo test -p vcon-runtime --test save_smoke -- --nocapture"
      ;;
    packaging)
      printf '%s' "cargo test -p vcon-pack --test validate_smoke -- --nocapture"
      ;;
    audio)
      printf '%s' "cargo test -p vcon-runtime --test audio_playback_smoke -- --nocapture"
      ;;
    *)
      return 1
      ;;
  esac
}

run_category() {
  local category="$1"
  local command
  command="$(category_command "$category")"
  local output_file="$TMP_DIR/${category}.log"

  echo "[acceptance] ${category}: ${command}"
  set +e
  bash -lc "$command" >"$output_file" 2>&1
  local code=$?
  set -e

  echo "$code" >"$TMP_DIR/${category}.exit"
}

set -e
for category in "${CATEGORIES[@]}"; do
  run_category "$category"
done

STEAM_CMD="cargo test -p vcon-runtime --test performance_smoke -- --nocapture"
STEAM_LOG="$TMP_DIR/steam_deck.log"
echo "[acceptance] steam_deck_profile: ${STEAM_CMD}"
set +e
bash -lc "$STEAM_CMD" >"$STEAM_LOG" 2>&1
STEAM_EXIT=$?
set -e

OVERALL_PASS=1
for category in "${CATEGORIES[@]}"; do
  code="$(cat "$TMP_DIR/${category}.exit")"
  if [[ "$code" != "0" ]]; then
    OVERALL_PASS=0
  fi
done
if [[ "$STEAM_EXIT" != "0" ]]; then
  OVERALL_PASS=0
fi

{
  echo "# Stream E Release Readiness Report"
  echo
  echo "Run timestamp (UTC): ${RUN_TS}"
  echo "Branch: $(git rev-parse --abbrev-ref HEAD)"
  echo
  echo "## Acceptance Categories"
  echo
  echo "| Category | Status |"
  echo "|---|---|"
  for category in "${CATEGORIES[@]}"; do
    code="$(cat "$TMP_DIR/${category}.exit")"
    if [[ "$code" == "0" ]]; then
      status="PASS"
    else
      status="FAIL"
    fi
    echo "| ${category} | ${status} |"
  done
  echo
  echo "## Steam Deck Profile Validation"
  echo
  if [[ "$STEAM_EXIT" == "0" ]]; then
    echo "- Steam Deck profile smoke (performance_smoke): PASS"
  else
    echo "- Steam Deck profile smoke (performance_smoke): FAIL"
  fi
  echo "- Hardware-on-device validation: PENDING (manual run required on physical Steam Deck)."
  echo
  echo "## Hardening Coverage"
  echo
  echo "- Static dynamic-import regression checks: vcon-engine/src/sandbox.rs tests"
  echo "- Runtime sandbox bypass regressions: vcon-runtime/tests/sandbox_bypass_regression.rs"
  echo
  echo "## Overall"
  echo
  if [[ "$OVERALL_PASS" == "1" ]]; then
    echo "- Overall status: PASS (with hardware validation pending)."
  else
    echo "- Overall status: FAIL"
  fi
  echo
  echo "## Notes"
  echo
  echo "- Detailed logs are available from this run in temporary execution output."
} >"$REPORT_PATH"

{
  echo "# Steam Deck Validation Report"
  echo
  echo "Run date: ${RUN_DATE}"
  echo "Run timestamp (UTC): ${RUN_TS}"
  echo "Branch: $(git rev-parse --abbrev-ref HEAD)"
  echo
  if [[ "$STEAM_EXIT" == "0" ]]; then
    echo "- Profile-based frame pacing smoke (vcon-runtime/tests/performance_smoke.rs): PASS"
  else
    echo "- Profile-based frame pacing smoke (vcon-runtime/tests/performance_smoke.rs): FAIL"
  fi
  echo "- Controller compatibility evidence source: vcon-runtime/tests/controller_hotplug.rs + diagnostics smoke."
  echo "- Physical Steam Deck hardware run: PENDING."
  echo
  echo "## Conclusion"
  echo
  if [[ "$STEAM_EXIT" == "0" ]]; then
    echo "Automated Steam Deck profile checks pass; physical-device validation remains open."
  else
    echo "Automated Steam Deck profile checks failed; physical-device validation not started."
  fi
} >"$STEAM_REPORT_PATH"

echo "[acceptance] release report: ${REPORT_PATH}"
echo "[acceptance] steam deck report: ${STEAM_REPORT_PATH}"

if [[ "$OVERALL_PASS" != "1" ]]; then
  exit 1
fi
