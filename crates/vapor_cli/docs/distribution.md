# Steam app/depot content staging

Vapor-Root is the app/depot source root. Its `App-Source.vapor.toml` declares
`[root]`, `[root.steam]`, and the split-depot file membership. The shipped app
root carries `App.vapor.toml` as its runtime app marker. The reviewed
first-party Engine, Game, and Packagepack runtime closure is bundled under
`content/default/`; mutable Workshop Library state remains in Vapor User Data.
Vapor-Root does not duplicate Cargo or Git submodule membership.

The Loo-Cast workspace still authors three content items: Spacetime Engine,
Loo-Cast Game, and Loo-Cast Packagepack. The Packagepack Cargo binary directly
links the Engine and Game crates, so its target build is the composed runnable
product. Depot staging contains the Packagepack manifest, that target binary,
and any declared required runtime libraries. It rejects component
directories, loose Engine/Game libraries, Vapor files, and undeclared files.

Vapor runtime files are a separate standalone distribution. The Steam app must
not contain the Vapor runtime, source checkouts, docs, developer tools,
toolchain caches, logs, receipts, or user state. Steam launches the Packagepack
executable directly. Vapor home owns Vapor files and mutable state.

## Staging behavior

`root package --release-targets` rebuilds the allowlisted staging tree under
`output/root/content` in Vapor home. `root publish --dry-run --release-targets`
repeats the stage, validates the required manifest paths, fingerprints the
payload, and writes SteamPipe preview files without uploading. A real publish
re-stages and verifies the fingerprint before upload. Neither operation copies
from the full Vapor home or adds undeclared files.

The `root.runtime.targets` list describes ABI targets available for this
product. `--target TARGET` selects a deliberate subset for a preview; the real
release should include the complete declared target matrix. Target-specific
includes must name the same target as the depot's operating-system kind.

Vapor keeps source checkouts, compiler state, tools, extracted-source cache,
installation records, settings, logs, and generated output in Vapor home. The
product source tree is an explicit input, not the owner of Vapor state.

[root.steam.depots.windows]
id = 2122623

[[root.steam.depots.windows.include]]
root = "source"
from = "resources/vapor/shell-scripts/windows/vapor-launch.cmd"
to = "bin/vapor-launch.cmd"
required = true

[[root.steam.depots.windows.include]]
root = "installation"
from = "bin/x86_64-pc-windows-gnullvm"
to = "bin/x86_64-pc-windows-gnullvm"
target = "x86_64-pc-windows-gnullvm"
required = true

[[root.steam.depots.windows.include]]
root = "installation"
from = "bin/x86_64-pc-windows-gnullvm/libunwind.dll"
to = "bin/x86_64-pc-windows-gnullvm/libunwind.dll"
target = "x86_64-pc-windows-gnullvm"
required = true

[root.runtime]
targets = [
    "x86_64-unknown-linux-gnu",
    "x86_64-pc-windows-gnullvm",
]
```

The depot IDs are the Steamworks DepotIDs. The include list is the source of
truth for file membership. `root = "source"` copies from the Vapor-Root source
checkout; `root = "installation"` copies from the Steam app root. `target`
limits an include to package/publish commands that stage that runtime target.

`root publish --dry-run` reconstructs clean Steam depot payloads under
`<Vapor User Data>/output/root/content` and writes a preview SteamPipe app
build VDF plus one depot build VDF per staged depot without uploading. The
payload is split by the manifest-declared Steam depot includes:

- `common/` contains OS-neutral files: `App.vapor.toml`, default App role
  manifests, and shared Engine assets. Authored tools and shell scripts are
  copied into Vapor User Data; the Installer's required bootstrap code is
  compiled into its shipped binary. Generated documentation and examples also
  stay outside the Steam depot.
- `linux/` contains Linux runtime files: selected Linux `bin/<target>/`
  application binaries including `vapor`, `vapor-entrypoint`,
  `vapor-installer`, `vapor-gui` (all GUI modes), `vapor-client-service`,
  and `vapor-ide-host`, plus `libsteam_api.so`, `bin/vapor-launch.sh`, and the
  Linux default Packagepack binary.
- `windows/` contains Windows runtime files: selected Windows `bin/<target>/`
  application binaries including `vapor.exe`, `vapor-entrypoint.exe`,
  `vapor-installer.exe`, `vapor-gui.exe` (all GUI modes),
  `vapor-client-service.exe`, and `vapor-ide-host.exe`, including
  `steam_api64.dll` and other required Windows
  runtime DLLs, plus `bin/vapor-launch.cmd` and the Windows default Packagepack
  binary.

The service owns the Steamworks native library metadata and runtime packaging
rule. `vapor-client-service` and `vapor` embed a relative loader path on Linux;
the service resolves its sibling DLL on Windows. Installer launches without requiring the shared
service so recovery remains available when service startup fails.

Source repositories, Cargo build targets, Cargo registries, Git checkouts,
Steam authentication, installer logs, generated Vapor User Data, and SteamPipe
cache state are not staged.
The examples payload is source material for learning and templating; it is
copied without its `.git/` or `target/` directories and is not Workshop content.
SteamPipe VDF file bodies live as checked-in source templates under
`resources/steam/steampipe-templates/`; Vapor Shell loads them from the active
source checkout and fills IDs, paths, branch, and description values at publish
time. The templates are distribution policy, not Shell crate assets and not
runtime depot content unless `App-Source.vapor.toml` explicitly includes them.

Source-controlled support assets live under visible `resources/` paths. Logs,
diagnostics, downloaded content, caches, receipts, and developer toolchains
belong to Vapor User Data. The Steam app instance remains replaceable.

When `[root.runtime].targets` is declared, no target flag means release-matrix
staging. Repeat `--target` to build and stage a deliberate custom subset. Use
`--host-only` for quick local packages that should not require or advertise
other platform launch scripts. Staging always includes the common depot plus
the platform depots implied by the selected targets, and fails when a selected
`bin/<target>/` directory is missing after the build refresh.

Only the designated CI distribution job may compile/promote Cargo's optimized
release profile or upload Steam/Workshop content. `root package` stages existing
release outputs; local `root build` and `root deploy` remain developer-profile
operations. This keeps local work in `dev` and distributed artifacts out of
the development output tree.

Real `root publish` does not accept custom target subsets, `--host-only`, or
`--skip-build`. It validates, builds, promotes, stages, and uploads the complete
declared Linux+Windows runtime matrix. Those escape hatches exist only for
`root package` target selection and `root publish --dry-run` staging previews.

Release staging is one app build with split depot roots. Steamworks should
mount the common depot for every OS, the Linux depot only for Linux, and the
Windows depot only for Windows. This prevents Linux Steam clients from selecting
the Windows/Proton launch path because of a mixed mono-depot payload.

For a release assembled from artifacts built on multiple machines, import the
target-specific app binaries into `bin/<target>/`, then use
`root publish --skip-build --dry-run` only as a preview that confirms the
imported files stage correctly.

Steam's default launch starts the persistently selected playable composition
through the Client service. The app manifest's `default` root only seeds an
empty first-run selection. Its optional `chooser` launch option opens Launcher, Studio, IDE,
Installer, and Shell. The CLI mirrors these routes with
`vapor client launch [MODE]`.
Installer opens its GUI product when available and falls back to the precompiled
installer. Shell stays a terminal recovery option:

```text
bin/x86_64-unknown-linux-gnu/vapor-entrypoint
  -> bin/x86_64-unknown-linux-gnu/vapor launch selected

bin/x86_64-unknown-linux-gnu/vapor-entrypoint chooser
  -> bin/x86_64-unknown-linux-gnu/vapor-gui --mode chooser
  -> re-enters vapor-entrypoint with the selected mode
     studio -> vapor-studio
     ide -> vapor-ide
     installer -> vapor-installer-gui
     shell -> Konsole -> vapor

bin/x86_64-unknown-linux-gnu/vapor-entrypoint shell
  -> Konsole -> bin/vapor-launch.sh --hold shell
  -> bin/x86_64-unknown-linux-gnu/vapor

bin/x86_64-unknown-linux-gnu/vapor-entrypoint installer
  -> bin/x86_64-unknown-linux-gnu/vapor-installer-gui
  -> if GUI is absent: Konsole -> bin/vapor-launch.sh --hold installer
  -> bin/x86_64-unknown-linux-gnu/vapor-installer

bin\x86_64-pc-windows-gnullvm\vapor-entrypoint.exe
  -> bin\x86_64-pc-windows-gnullvm\vapor.exe launch selected

bin\x86_64-pc-windows-gnullvm\vapor-entrypoint.exe chooser
  -> bin\x86_64-pc-windows-gnullvm\vapor-gui.exe --mode chooser
  -> re-enters vapor-entrypoint with the selected mode
     studio -> vapor-studio.exe
     ide -> vapor-ide.exe
     installer -> vapor-installer-gui.exe
     shell -> cmd /C -> vapor

bin\x86_64-pc-windows-gnullvm\vapor-entrypoint.exe shell
  -> cmd /C -> bin\vapor-launch.cmd --hold shell
  -> bin\x86_64-pc-windows-gnullvm\vapor.exe
  -> cmd /K after Vapor exits

bin\x86_64-pc-windows-gnullvm\vapor-entrypoint.exe installer
  -> bin\x86_64-pc-windows-gnullvm\vapor-installer-gui.exe
  -> if GUI is absent: cmd /C -> bin\vapor-launch.cmd --hold installer
  -> bin\x86_64-pc-windows-gnullvm\vapor-installer.exe
  -> cmd /K after Vapor Installer exits
```

The launch scripts dispatch an explicit Installer request to `vapor-installer`
and send GUI requests to the target entrypoint. Opening the ordinary Shell does
not implicitly run Installer setup; setup and role changes belong to the
Installer workflow. The Windows Shell uses the console's normal echoed line
input because Reedline does not reliably paint the current input line in some
Windows console hosts. Entry points and scripts are not a substitute for
target-specific app or content payloads; Vapor itself, engines, games, tools,
and dynamic libraries all need runtime outputs staged per supported target.

The publish preflight requires Vapor User Data Rust/Cargo and cross-build tooling for
the release matrix, Vapor-managed Git for explicit Git-backed workflows, and
SteamCMD for real upload. An operator-linked Git provider is an explicit
override; host PATH discovery is not a release bootstrap path. Player-mode and
developer-mode setup are owned by app-root tools under
`<app-root>/resources/vapor/tools/`.

The root source owns the tool payload under `resources/vapor/tools`; root
build/package/publish copies that payload into the installed app root. The
generated mutable state those tools create remains outside SteamPipe staging.
`vapor-installer` still uses the same shared implementation in-process so
first-run setup does not depend on `rust-script` already being available.

Workshop publishing is separate from the default App depot payload. Loo-Cast
can publish searchable Engine, Game, and Packagepack items; Steam Play uses
the depot-owned closure before any Workshop acquisition.
