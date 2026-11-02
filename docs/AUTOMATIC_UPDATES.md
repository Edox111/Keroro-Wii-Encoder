# Automatic updates

The application uses vanilla TypeScript, not React. `src/updates.ts` starts once
after `src/main.ts` initializes the desktop UI. Development builds do not check
for updates. Release builds check only after a real public key replaces
`YOUR_GENERATED_PUBLIC_KEY_HERE` in `src-tauri/tauri.conf.json`.

Checks and downloads run in the background without locking encoding controls.
After a verified download, a fixed bottom-corner notice displays the offered
version and **Restart Now**. There is no automatic installation merely because
an encode finishes. The button remains disabled while encoding, preview generation,
a folder picker, a modal, or another protected task is active.

An explicit click rechecks idle state and uses the existing native
`prepare_update_install` guard before installation. That reservation blocks new
jobs while installation hands off to NSIS. Installing first is essential: calling
`relaunch()` alone would just restart the old app. Network, manifest, signature,
and pre-handoff installation failures leave normal app use available. Disposal
removes the notice and closes updater resources; downloads are bounded by a timeout.

## One-time signing setup

Tauri updater signatures are mandatory and free to generate. They are separate
from Windows Authenticode certificates and do not remove SmartScreen warnings.
Run the following yourself in PowerShell; keep the private key outside the repo:

```powershell
New-Item -ItemType Directory -Force "$env:USERPROFILE\.tauri" | Out-Null
npm run tauri signer generate -- -w "$env:USERPROFILE\.tauri\keroro-wii-encoder.key"
```

Back up the private key and any password securely. Never commit, upload, or paste
the private key into the application configuration. Put the contents of the
generated `.key.pub` file into `plugins.updater.pubkey`, not its filename.

The configured endpoint is:

```text
https://github.com/Edox111/keroro-wii-encoder/releases/latest/download/latest.json
```

The placeholder key deliberately disables startup checks. This feature cannot
deliver an update until a real key and signed release manifest are published.

## Publish each update

1. Increase the app version above the installed version in npm, Cargo, Tauri, and
   the corresponding lockfiles.
2. Set signing environment variables in the build terminal. The private-key
   variable accepts a file path; supply the password securely if one was used:

   ```powershell
   $env:TAURI_SIGNING_PRIVATE_KEY = "$env:USERPROFILE\.tauri\keroro-wii-encoder.key"
   npm run build:release
   ```

3. Upload the generated NSIS setup executable to the matching GitHub release.
   With `bundle.createUpdaterArtifacts: true`, Tauri also generates a `.sig`.
4. Copy `latest.json.example` to `latest.json`, fill in real values, then upload
   it as an asset of the same GitHub release alongside the signed installer and
   `.sig`. Use the actual `.sig` file contents as the signature:

   ```json
   {
     "version": "1.1.1",
     "notes": "Describe this release.",
     "platforms": {
       "windows-x86_64": {
         "url": "https://github.com/Edox111/keroro-wii-encoder/releases/download/v1.1.1/Keroro.Wii.Encoder_1.1.1_x64-setup.exe",
         "signature": "REPLACE_WITH_CONTENTS_OF_THE_GENERATED_SIG_FILE"
       }
     }
   }
   ```

   The URL above is an example: use the exact URL of your uploaded asset. Do not
   publish placeholder signatures. Do not change an installer after signing it.

The currently public application must first receive a build containing this
updater integration; an older build cannot acquire an updater it does not have.

## Windows installation and restart

`plugins.updater.windows.installMode` is `passive`: it skips the wizard's
interactive steps but displays installer progress. The per-machine installation
can still require Windows UAC approval; the application does not bypass UAC.

On Windows, `update.install({ restartAfterInstall: true })` exits the old app and
passes restart responsibility to NSIS. The custom template retains `.onInstSuccess`
handling for `/R`, `/P`, and `/UPDATE`. `relaunch()` is the fallback on platforms
where installation returns normally. Live signed installation/UAC/restart tests
are required before publishing; mocked lifecycle tests do not certify that flow.

Update checks/downloads contact GitHub over HTTPS. Media conversion remains
local; there is no telemetry or media upload. Signature verification remains
enabled.

References: [Tauri updater](https://v2.tauri.app/plugin/updater/) and
[JavaScript updater API](https://v2.tauri.app/reference/javascript/updater/).

## Tauri 2 configuration and React reference

Tauri 2 uses `plugins.updater` and `bundle.createUpdaterArtifacts`, not the v1
`bundle.updater.active` configuration. The updater and process plugins, capabilities,
and native handlers are already registered; this change does not modify them.
The production frontend remains TypeScript/Vite and starts its existing controller
from `src/main.ts`. No React dependency or `App.jsx` is introduced.

If a React frontend is introduced later, this wrapper reuses the same controller
and its native install guard without capturing stale encoding state:

```jsx
import { useEffect, useRef } from "react";
import { startAutomaticUpdates } from "./updates";

export function useBackgroundUpdater(lifecycle) {
  const current = useRef(lifecycle);
  current.current = lifecycle;
  useEffect(() => {
    if (import.meta.env.DEV) return;
    return startAutomaticUpdates({
      canInstall: () => current.current.canInstall(),
      setInstalling: (value) => current.current.setInstalling(value),
    });
  }, []);
}
```

The host supplies the existing idle predicate and installation-lock callback;
these must include active encodes and preview work. The controller imports
`check` from `@tauri-apps/plugin-updater` and `relaunch` from
`@tauri-apps/plugin-process`; the old `@tauri-apps/api/updater` and
`@tauri-apps/api/process` modules are not used in Tauri 2.

The notice uses `update.version`, not a fixed 1.1.0 string. An installed 1.1.0
build needs a newer manifest version (for example 1.1.1) to offer an update.
The example manifest is a template, not a signed release. No update feed or
installer has been uploaded by this implementation.
