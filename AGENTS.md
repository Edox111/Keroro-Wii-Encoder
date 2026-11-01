# Project instructions

## Clean builds and release artifacts

Before every Tauri release build, clean the application's old release artifacts and generated installer/frontend output. Use the configured `beforeBuildCommand`, which runs `npm run clean:release` before `npm run build`; do not bypass this hook.

The canonical Windows installer command is `npm run build:windows`. Direct `npm run tauri -- build` and `npm run tauri -- build --no-bundle` also use the cleanup hook.

The cleanup script runs `cargo clean -p keroro-wii-encoder --release` with the project's manifest, removes the resolved release `bundle` directory and frontend `dist`, and stops on errors or a locked executable. Ask the user to exit a running app through its tray menu rather than terminating an active encoding job.

Keep recursive cleanup inside the project and reject symbolic links or junctions. Use `npm run clean:release -- -DryRun` to inspect cleanup targets without deleting release files.
