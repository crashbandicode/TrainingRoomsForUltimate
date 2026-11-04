import re

def cycle_request(line, role):
    adapter = "MiiOrdinaryPlayer" if role == 0 else "MiiOrdinaryCpu"
    return re.search(r"\[" + adapter + r"\] (bank switch|submit) ticket=(Ticket \{[^}]+\}) .*?target=(0x[0-9a-f]+)\b", line)


def cycle_result(line, role, request):
    # The bank may become ready on its fallback member while this transaction
    # refuses. Only this request/target and its released adapter receipt count.
    requested = cycle_request(request, role) if request else None
    if requested is None:
        return None
    adapter = "MiiOrdinaryPlayer" if role == 0 else "MiiOrdinaryCpu"
    mode, ticket, target = requested.groups()
    if any(f"[{adapter}] {event} ticket={ticket} " in line
           for event in ("bank switch refused", "bank build refused", "refused")):
        return "refused"
    event = "bank ready" if mode == "bank switch" else "ready"
    if (f"[{adapter}] {event} ticket={ticket} " in line
            and re.search(r"\btarget=" + target + r"\b", line)
            and "released=true" in line):
        return "ready"
    return None


