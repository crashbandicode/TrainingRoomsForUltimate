# Troubleshooting

## Start with what you see

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/06348008e027aeaf9cc70d69c9ab74930ce388bc" autoplay muted loop playsinline controls></video>

*The Save States notice identifies the disabled setting.* [Open the clip](../media/tut-notice-save-states-off.mp4)

| Symptom | Try this |
|---|---|
| No install line in `events.log` | Check the [plugin directory and game version](getting-started.md#installing). |
| Menu won't open | Close the game's menu; use the [permanent shortcut](../menu/display-and-controls.md#permanent-shortcut). |
| Only waiting rooms fail | Turn on [Lobby Practice](../menu/fighters-and-stages.md#lobby-practice) in Training before entering a room. |
| <a id='a-notice-says-no'></a>Save refused | Enable [Save States](../menu/saves.md#save-states); [make or bind a save](save-states.md) before loading. |
| One row is grey | Read its help line; a prerequisite may be off or a load still running. |
| Cycle Player does nothing | Check [Player Swapping](../menu/fighters-and-stages.md#player-swapping); wait for any current swap to finish. |
| Fighters wait for memory | Check [Loading](../menu/fighters-and-stages.md#loading); reduce Ready Fighters or Ready Hats if needed. |
| An input goes missing | Check [chord ownership](controls.md#online-deluxe) and [waiting-room controls](waiting-rooms.md). |
| <a id='where-the-logs-are-and-what-to-send'></a>Still stuck? | Copy `events.log`, `practice-diagnostics.log` and `practice-diagnostics.previous.log` from `sd:/ultimate/TrainingRooms/` before starting again. |
| With your report | Include game version, menu build tag, other plugins, steps and exact notice. |
