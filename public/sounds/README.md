# Completion sound

Place your sound at `public/sounds/completion.mp3` or `public/sounds/completion.wav`.
Vite copies it unchanged to `/sounds/` for the packaged app. MP3 is tried first,
then WAV; missing files or blocked playback are ignored without rejecting any
encoding operation. No empty or invalid placeholder media is included.

Settings > Play completion sound controls this cue independently of UI clicks.
Volume is capped at 0.4. The cue runs once when an admitted batch or entire file
queue finishes, including mixed per-file results. It never runs for previews,
paused jobs, cancelled/interrupted queues, or a historical completed snapshot.
