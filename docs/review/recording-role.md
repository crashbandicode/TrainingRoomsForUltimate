# Recording: which role you are on (page and pop-up)

**Pending, 2026-11-22.** Three decisions, one picture. RR1 changes the look; RR2 and RR3 are about the role memory you asked for.
Mock-up, not an Eden still (preview-vis-sol, source `aad616bc`, branch `rc15/recording-menu-mix`). Nothing on this page is implemented yet.

Your words: "in recording on the menu when you change roles it's not clear what role you're on. we should make it clear like we do in the pop up." and "when you bring up the recording dialogue menu with ZR+D-Right it should remember which role you had picked before vs always restarting on p1."

## RR1: the Recording page shows the pop-up's role tabs (default: as pictured)

- The page gets the same plate as the pop-up, in the same place on the top line: "Player" and "CPU", the current one lit red.
- One key cap, **ZR**, to the right of the plate. ZR already changes the role on the page; L and R change the menu page there, so they cannot be the role buttons as in the pop-up.
- The small "TrainingRooms" label at the top right is not drawn on the Recording page, as in the pop-up. It stays on the other pages.
- The tabs show on the whole Recording page, and on a file's own screen opened from it.
- Nothing else on the page changes.

![The Recording page with the pop-up's role tabs](recording-role/recording-role-proposal.png)

Rows 1 and 2 are the build you have. Rows 3 and 4 are composed from native stills; the "ZR" cap is drawn in.

## RR2: the page and the pop-up each keep their own role (default: two roles)

- **Pop-up role:** who Record captures and whose macros and loops the pop-up lists. This is what you asked to be remembered.
- **Page role (ZR):** whose files the page lists. It is remembered the same way.
- They stay separate, as today. With one shared role, pressing ZR on the page would also change who the pop-up records next time.
- The other option: **one shared role** for both. Say so if you prefer it.

## RR3: how long the role is remembered (default: until the game is closed)

- The same as the menu's "reopen where you left": through closing the menu, a Training reset and leaving Training.
- After the game is closed and started again, both start on Player. Nothing is written to the card.
- The other option: **keep it on the card**, so it also survives a restart.
