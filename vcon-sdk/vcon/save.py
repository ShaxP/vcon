"""Save API with runtime-injected namespace and quota enforcement."""

import json
import os
from pathlib import Path
import time

_save_root = None
_quota_bytes = 0


def _set_runtime_state(root_path, quota_mb):
    global _save_root, _quota_bytes
    _save_root = Path(root_path)
    _quota_bytes = int(quota_mb) * 1024 * 1024
    _save_root.mkdir(parents=True, exist_ok=True)


def _require_runtime_state():
    if _save_root is None:
        raise RuntimeError("vcon.save runtime state is not initialized")


def _validate_slot(slot):
    if not isinstance(slot, str) or not slot.strip():
        raise ValueError("slot must be a non-empty string")
    if "/" in slot or ".." in slot or "\\" in slot:
        raise ValueError("slot contains invalid path components")


def _slot_path(slot):
    _validate_slot(slot)
    _require_runtime_state()
    return _save_root / f"{slot}.json"


def _current_usage_bytes():
    if _save_root is None or not _save_root.exists():
        return 0
    total = 0
    for item in _save_root.iterdir():
        if item.is_file():
            total += item.stat().st_size
    return total


def write(slot, data):
    path = _slot_path(slot)
    encoded = json.dumps(data, separators=(",", ":")).encode("utf-8")

    previous_size = path.stat().st_size if path.exists() else 0
    usage_after = _current_usage_bytes() - previous_size + len(encoded)
    if usage_after > _quota_bytes:
        raise RuntimeError(
            f"save quota exceeded: {usage_after} bytes would exceed {_quota_bytes} bytes"
        )

    tmp_path = path.with_suffix(".tmp")
    with tmp_path.open("wb") as f:
        f.write(encoded)
    os.replace(tmp_path, path)


def read(slot):
    path = _slot_path(slot)
    if not path.exists():
        return None
    try:
        with path.open("rb") as f:
            return json.loads(f.read().decode("utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError):
        # Recovery semantics: quarantine unreadable/corrupt slot data and continue.
        stamp = int(time.time() * 1000)
        quarantine = path.with_suffix(f".corrupt.{stamp}.json")
        try:
            os.replace(path, quarantine)
        except OSError:
            pass
        return None


def list_slots():
    _require_runtime_state()
    slots = []
    for item in _save_root.iterdir():
        if item.is_file() and item.suffix == ".json":
            slots.append(item.stem)
    slots.sort()
    return slots
