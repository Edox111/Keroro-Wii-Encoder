# UI feedback audio

Add your licensed files here, then rebuild:

- click.mp3: buttons, checkboxes, and selection controls.
- success.mp3: a fully successful downscale batch.
- error.mp3: task failures and actionable caught errors.

These files are optional placeholders; unavailable or rejected playback never
blocks encoding. The main.ts HTML5 Audio manager caps volume at 0.4, reuses
players, and allows one UI clip at a time. UI sounds defaults on and is saved in
keroro.preferences.v1 as isMuted. Unchecking or disabling that checkbox silences
UI audio. Video preview and encoded audio are unaffected.

Optional character clips remain under sfx/[character].mp3. They use the same
manager and mute state. The legacy synthetic sfx/click.mp3 and sfx/complete.mp3
are not bundled or used by the new feedback hooks.
