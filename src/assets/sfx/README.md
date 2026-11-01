# Optional character clips

Licensed clips may be named keroro.mp3, tamama.mp3, giroro.mp3, kururu.mp3,
and dororo.mp3. Vite discovers these files when rebuilding. No character voice
recordings are supplied. Missing character clips leave the normal click hook.

All clips share the Audio manager in src/main.ts and the UI sounds preference.
The legacy synthesized click.mp3 and complete.mp3 in this folder are unused;
place new UI feedback files in src/assets/ as described in ../README.md.
