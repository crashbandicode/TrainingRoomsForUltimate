# rc.5 confirming re-run: `f9_body_of` skips an unpublished cell

Arm on the rebuilt release NRO after `symbols load` of its unstripped ELF (build id must match). Module `libtraining_rooms.nro`. No diag print.

The state word is the installed count `entry+0x4150`. Cell 0 is a body only when that count publishes index 0. The capture of `[entry+0x4160]` is the cell word inside the entry, not a dereference of the body.

| Point | Target | Registers | Captures | Condition | on_return | What a hit shows |
| --- | --- | --- | --- | --- | --- | --- |
| stale | `libtraining_rooms.nro!<training_rooms::runtime::roster::character_roster::sources::f9_body_of>` | `x0` entry, `x1` slot | `[x0+0x4150]` u64, `[x0+0x4160]` u64, `[x0+0x591c]` u8 | `[x0+0x4150] == 0` | yes (`x0`, `x1`) | The stale state: count 0, and the cell word may be aligned residue. The function returns. An `Unmapped Read` at this PC on that hit means the residue was dereferenced. No such line means the unpublished cell was skipped. |
| published | the same symbol | `x0` entry, `x1` slot | the same three | `[x0+0x4150] != 0` | yes (`x0`, `x1`) | A published window still reaches the body read. Pair the return with the slot. |

Disarm after the Training loader has serviced entries 2 and 3 once with count 0 and once with a count of 1, 2, or 3. One boot. Do not rebuild to answer this.
