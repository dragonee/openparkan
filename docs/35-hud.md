# The cockpit HUD — the panels, the radar, the weapons and the messages

What the player sees over the world while driving a unit: the weapons list at
the top right, the message box at the top, the radar in the middle at the
bottom, the target panel at the bottom left and the player's own unit at the
bottom right. This page reads how `iron3d.dll` builds and draws them, and
measures the result against the install's art and a recording of Mission 01.

**Every claim is tagged**, as in [15-behaviour.md](15-behaviour.md):
- *measured* is re-derived by `openparkan verify`;
- *read* comes from the disassembly at the address given;
- *derived* follows from the two;
- *guess* fits and is not established.

The HUD's art is the `textures` resource of `ui/game_resources.cfg`: pages of
`ui/ui.lib`, each a 256 × 256 `Texm` ([02-texm.md](02-texm.md)). Its fonts are
the `fonts` resource beside it ([34-progression.md](34-progression.md#after-the-outcome--read-and-measured)).

## The weapons and the messages

To be written.

## The radar and the indicators below it

To be written.

## The target panel and the player's own unit

To be written.
