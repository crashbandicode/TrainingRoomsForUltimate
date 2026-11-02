"""route_s6.py with the install gate on the line's start (`plugin=training_rooms/`), for builds whose one install line is cut before
`result=` by the diagnostics line limit (s6-04, 2026-11-02: 8,142 characters, reported to the lead). Everything else is route_s6's.
Usage: route_s6_cut.py <label>"""
import sys

import route_s6 as rt

_wait_line = rt.wait_line


def wait_line(path, needle, cap, what, after=0):
    if needle == "result=installed":
        needle = "plugin=training_rooms/"
    return _wait_line(path, needle, cap, what, after)


rt.wait_line = wait_line

if __name__ == "__main__":
    rt.to_training(sys.argv[1], ["right"] * 6, 6, ["up"])
