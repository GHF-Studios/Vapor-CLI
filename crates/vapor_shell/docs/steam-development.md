# Steam development workflow

## Launch and install

Vapor does not use `installscript.vdf` for first-run app install. Steam install scripts
are not available on Linux/SteamOS, and Vapor needs one cross-platform first-run
model for Windows and Linux testers.

## Steamworks app, packages, and depots

Vapor should publish one app build made from three root depots:

- **common depot**: OS-neutral `App.vapor.toml`, docs, app scripts, and
  examples.
  Steamworks OS rule: all operating systems.
- **linux depot**: Linux launch script and Linux `bin/<target>/` runtime
  binaries, including `vapor-entrypoint`. Steamworks OS rule: Linux.
- **windows depot**: Windows launch script, Windows `bin/<target>/` runtime
  binaries including `vapor-entrypoint.exe`, and required runtime DLLs.
  Steamworks OS rule: Windows.

The root `App-Source.vapor.toml` records those IDs and each depot's include
list under `[root.steam.depots.*]`. Do not publish a split build until the real
Steamworks depot IDs and file mappings are configured there:

```toml
[root.steam]
app-id = 2122620
development-branch = "vapor-dev"

[root.steam.depots.common]
id = 2122621

[[root.steam.depots.common.include]]
root = "source"
from = "App.vapor.toml"
to = "App.vapor.toml"
required = true

[root.steam.depots.linux]
id = 2122622

[[root.steam.depots.linux.include]]
root = "source"
from = "resources/vapor/shell-scripts/linux/vapor-launch.sh"
to = "bin/vapor-launch.sh"
required = true

[[root.steam.depots.linux.include]]
root = "installation"
from = "bin/x86_64-unknown-linux-gnu"
to = "bin/x86_64-unknown-linux-gnu"
target = "x86_64-unknown-linux-gnu"
required = true

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
```

The project root has the full include list, including docs, launch scripts, and
examples. `vapor-entrypoint[.exe]` is a promoted Vapor-Steam-Entrypoint binary and
belongs in the promoted/imported `bin/<target>/` directory before depot
staging. Windows runtime DLLs belong in the promoted/imported
`bin/<windows-target>/` directory before depot staging.

Use packages as access/license containers, not as source branches:

- **developer package**: private/internal access to the app and every app depot;
- **closed-alpha/key package**: tester access to the same runtime depots;
- **future public/release package**: customer access once the app is ready.

Branches remain build channels. Keep `vapor-dev` as the development beta branch
for uploaded test builds. Add more branches only when there is a concrete build
promotion need, such as a stable alpha branch distinct from internal dev.

The depot must include `vapor-entrypoint[.exe]`, `vapor-gui[.exe]`,
`vapor-installer[.exe]`, and `vapor-ide-host[.exe]` under `bin/<target>`;
the Vapor Shell binary is also required for the Shell recovery option. Steam's
default launch option is **Vapor Studio** and must invoke the native entrypoint
with the `studio` argument. The entrypoint starts the sibling Vapor-GUI binary
with the selected mode; this is a GUI path and does not open a terminal.
Installer falls back to the precompiled bootstrap if Vapor-GUI is missing.
Keep the Shell recovery option separate:

- **Linux Vapor Studio (default)**: executable
  `bin/x86_64-unknown-linux-gnu/vapor-entrypoint`,
  arguments `studio`.
- **Linux Play**: executable
  `bin/x86_64-unknown-linux-gnu/vapor-entrypoint`,
  arguments `play` (Studio launches the selected profile composition).
- **Linux Vapor Shell**: executable
  `bin/x86_64-unknown-linux-gnu/vapor-entrypoint`,
  arguments `shell`.
- **Linux Vapor Installer**: executable
  `bin/x86_64-unknown-linux-gnu/vapor-entrypoint`,
  arguments `installer`.
- **Windows Vapor Studio (default)**: executable
  `bin\x86_64-pc-windows-gnullvm\vapor-entrypoint.exe`,
  arguments `studio`.
- **Windows Play**: executable
  `bin\x86_64-pc-windows-gnullvm\vapor-entrypoint.exe`,
  arguments `play` (Studio launches the selected profile composition).
- **Windows Vapor Shell**: executable
  `bin\x86_64-pc-windows-gnullvm\vapor-entrypoint.exe`,
  arguments `shell`.
- **Windows Vapor Installer**: executable
  `bin\x86_64-pc-windows-gnullvm\vapor-entrypoint.exe`,
  arguments `installer`.

The Linux and Windows entrypoints launch Studio for both `studio` and `play`;
`play` adds Studio's `--play` mode, which builds and runs the selected profile
composition. `shell` and `installer` continue through the terminal wrapper. Set
the Steam default launch option to Studio, not Play, so users arrive at the GUI
without automatically launching a composition.
Launch-time bootstrap failure is recorded under
`<app-root>/.vapor/state/installer/bootstrap-failure.txt`; Shell reads that
app-local state instead of inheriting wrapper environment variables. Running
`vapor-installer` without arguments prints shim help/status for human-driven
lifecycle work; scripts are the direct source-checkout command surface.
The `installer` launch target skips headless install and opens
`vapor-installer` directly, so users can manage
install/uninstall/developer-mode state even when player-mode install is broken
or intentionally removed.

Player-mode install prepares only app-local basic runtime tooling:

- SteamCMD under `tools/steamcmd`;
- disposable app-local state, log, diagnostics run directories, and content-cache directories.

It does not install Rust/Cargo or cross-build toolchains. Development tooling is
explicit:

```text
rust-script --force <app-root>/resources/vapor/tools/production/app_setup/setup_development.rs
rust-script --force <app-root>/resources/vapor/tools/production/app_setup/teardown_development.rs
```

If launch-time install fails, the first visible Vapor Shell reports the
failure and the exact setup command. For ordinary testers, reinstalling the Steam app is the
preferred recovery because the app root is disposable and should not hold
authoritative user data.

The Steam **Play** launch target opens Vapor Studio with `--play`, which loads
the selected profile and attempts to run its composition. If no usable profile
selection exists, Studio remains open and explains what needs attention.

From Vapor Shell, `launch default` remains available as a CLI operation. It
resolves the app manifest's default packagepack seed, installs that packagepack
and its dependency closure when missing, reads the selected packagepack's
declared dependency roles, resolves the required `engine` dependency by content
ID, records any declared `game` dependency by content ID, and hands off to the
resolved engine artifact's binary for the host runtime target.

Workshop downloads use anonymous SteamCMD by default. `--account ACCOUNT` exists
for deliberately private app/content testing, not normal Play.

## Local development deploy bridge

The local bootstrap script is still only a developer bridge for placing the
first runnable binaries into a Steam app directory. It is not a product
installer and does not copy source repos, Cargo workspaces, staged package
trees, generated outputs, or user state.

A development loop now has two explicit phases:

1. Use the source-controlled bootstrap path to place current Vapor
   binaries and launch scripts into the Steam app directory.
2. Run `<app-root>/resources/vapor/tools/production/app_setup/setup_development.rs`
   only when that app root needs to build, validate, package, or publish Vapor
   projects.

Release-mode depot builds should target every shipped app platform:

```text
root publish --dry-run
```

The Windows GNU/LLVM target can be built from Linux with app-local llvm-mingw,
and the Linux GNU target can be built from Windows with the app-local Zig
cross-link wrapper model. If a target is built on another machine, preserve the same
`bin/<target>/` and `output/dev/<workspace>/<target>/debug/` relative paths
when copying artifacts back to the publishing app root.

For quick local Linux smoke, pass `--host-only`; Vapor then stages only the
host `bin/<target>/` directory plus the matching launch script. When Windows
artifacts were imported from another machine, use
`root publish --skip-build --dry-run` only to preview staging. A real
`root publish` always validates, builds, promotes, stages, and uploads the full
declared Linux+Windows matrix.

The concrete Windows build and Linux handoff checklist is documented in
[`windows-gnullvm-release-proof.md`](windows-gnullvm-release-proof.md).

## Authentication

Vapor does not expose a standalone SteamCMD login workflow. Real publication is
the authentication boundary: `root publish --account NAME --yes` starts the
installation-owned SteamCMD with inherited stdin/stdout, lets Steam own any
password or mobile-authenticator prompts, and returns to the REPL after SteamCMD
exits. Vapor never accepts a password argument and never copies SteamCMD's
`config/` into staging.

Steam authentication is session-scoped by policy. Commands that publish for real
must be typed manually in the interactive shell; scripts may dry-run but may not
authenticate, perform real uploads, create Workshop items, or delete Workshop
items.

## Preview and publish

Use `root publish --dry-run` first. It validates, builds, promotes app binaries,
builds docs, stages and smoke-tests the split runtime depots, and writes an
app-build VDF with `Preview = 1` and `SetLive = vapor-dev` plus one depot-build
VDF per staged depot; it performs no upload and does not require active
SteamCMD.

A real upload requires both a non-default branch and explicit confirmation:

```text
root publish --account NAME --branch vapor-dev --yes
```

SteamCMD runs in the foreground so progress, prompts, exit status, and failure
remain attached to the operation. `output/root/steam-build` is not cleared by
staging; it contains SteamPipe manifests and chunk cache that improve subsequent
uploads.

The VDFs map only the already-clean split staging roots. Inclusion and
credential exclusion are therefore decided before SteamPipe sees any files.

Workshop content publication is separate from app/depot publication. Use
`content publish ARTIFACT --dry-run` from a content workspace for package and
Workshop VDF previews, then perform any real content upload manually with
`content publish ARTIFACT --account ACCOUNT --yes`.
