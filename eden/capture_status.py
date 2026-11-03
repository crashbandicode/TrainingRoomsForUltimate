"""Evidence gate for the idle native replay adapter, not the numbered recorder.

A capture preference is distinct from a native record's latched arm state.
Older diagnostics omit that distinction and cannot pass this acceptance gate.
"""
import re


def idle_preference(line: str | None, enabled: bool) -> bool:
    if not line or "[NamedMacroDiag] status " not in line:
        return False
    pairs = re.findall(r"\b(capture_[a-z_]+)=(\w+)\b", line)
    fields = dict(pairs)
    if len(fields) != len(pairs):
        return False
    return (
        fields.get("capture_ready") == "true"
        and fields.get("capture_phase") == "Idle"
        and fields.get("capture_armed") == "false"
        and fields.get("capture_desired") == str(enabled).lower()
    )
