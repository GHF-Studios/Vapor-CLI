# Loo-Cast Steam content mount

Vapor Client is installed and updated independently through Vapor's own
release service. This checkout no longer declares a Steam app or depot release
for Vapor itself and must not be used to upload Vapor binaries, tools, docs,
source checkouts, caches, or mutable state to Steam.

The Loo-Cast Steam app is a local Packagepack mount. The common depot contains
`Packagepack.vapor.toml`; the Linux and Windows depots each contain one matching
primary Packagepack executable and only runtime libraries declared as required
by that target. The executable composes the Engine and Game; there is no
`App.vapor.toml`, separate component directory, loose Engine/Game library, or
Vapor locator.

The Loo-Cast app source/depot manifest belongs with the Loo-Cast app packaging
source, not this Vapor Client checkout. It lives at the Loo-Cast source root as
`App-Source.vapor.toml`; select that checkout after packaging the registered
Loo-Cast Packagepack. `root package` stages those Steam depots without
requiring a separate Steam-mount project. The previous workflow that ran
`root package` and `root publish`
from Vapor Client to assemble the Loo-Cast app is retired. Generic `root
package` and `root publish` remain available for an explicitly selected
application source that declares its own depot policy.

`App-Source.vapor.toml` is the allowlist for app/depot files. It takes the
composition manifest from this source checkout and product trees from
`VAPOR_PRODUCT_ROOT`. The current product layout has shared product metadata
plus target-specific ABI artifacts:

- The common depot contains `App.vapor.toml` and shared product files.
- The Linux depot contains the Linux engine library, game library, and
  Packagepack binary under their `x86_64-unknown-linux-gnu` directories.
- The Windows depot contains the Windows engine library, game library, and
  Packagepack binary plus `libunwind.dll` under their
  `x86_64-pc-windows-gnullvm` directories.

The manifest excludes `lib/` and `bin/` from the common product trees so ABI
files are not copied to every platform. Platform-scoped includes put each
artifact in the depot for its target. `root.runtime.targets` is required and
lists the ABI targets this content release supports; it does not describe Vapor
runtime binaries. Release staging rejects targets not declared there and
rejects a target whose platform depot or matching target-scoped product files
are missing.

Keep all Vapor-owned files and generated state outside the product tree. Product
source caches, compiler caches, user settings, logs, installer state, and Vapor
runtime binaries belong under Vapor home. A Steam depot include must be an
explicit product file or composition manifest; never point it at a general
Vapor-home directory.

## Review and publish

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

The depot keeps the small player runtime: `vapor-entrypoint[.exe]`,
`vapor[.exe]` for launching the bundled default composition, and
`vapor-client-service[.exe]` for the shared client bootstrap, plus the runtime
libraries those binaries need. Creator GUIs, CLI installer, and Shell are
provided by the separate Vapor installation and are excluded from Steam depots.
The target Steam configuration has two user-facing options:

- **Play Loo-Cast (default)**: native entrypoint executable with no arguments.
  This launches the default composition through Vapor Client.
- **Create with Vapor**: the same executable with `creator` as its argument.
  This starts the active standalone Vapor release under Vapor User Data, or
  opens the Homepage Downloads page when no release is installed. The bundled
  `chooser` remains a local development and recovery route.

The standalone release's entrypoint dispatches its chooser to separate
Launcher, Studio, IDE, and Installer GUI executables, all built from the shared
Vapor-GUI host. Steam's depot does not carry those creator tools. These are the
two supported Steam launch settings for both target depots.

| Option | Linux | Windows |
| --- | --- | --- |
| Play Loo-Cast (default) | `bin/x86_64-unknown-linux-gnu/vapor-entrypoint` (no arguments) | `bin\x86_64-pc-windows-gnullvm\vapor-entrypoint.exe` (no arguments) |
| Create with Vapor | `bin/x86_64-unknown-linux-gnu/vapor-entrypoint creator` | `bin\x86_64-pc-windows-gnullvm\vapor-entrypoint.exe creator` |
Running `vapor-installer` without arguments prints shim help/status for
human-driven lifecycle work; scripts are the direct source-checkout command
surface. The `installer` launch target opens the Installer GUI and falls back
to the command-line recovery workflow if that GUI is unavailable.

Player setup prepares Vapor User Data state, log, diagnostics, and mutable
content directories. SteamCMD is a developer/publisher tool in Vapor User Data.

It does not install Rust/Cargo or cross-build toolchains. Development tooling is
explicit:

```text
root package --release-targets
root publish --dry-run --release-targets
```

The preview stages only paths allowed by `App-Source.vapor.toml`, validates
required files for the selected ABI targets, and writes SteamPipe preview
files. It does not compile Vapor, package Vapor-home content, or upload.
Product builds must already have produced the target-specific files under
`VAPOR_PRODUCT_ROOT`. Steam publication never invokes Cargo or builds Vapor; it
packages only the already-installed product files selected by the manifest.

The default Steam **Play Loo-Cast** launch target starts the default playable
composition. **Create with Vapor** hands off to the standalone Vapor chooser,
which opens Launcher, Studio, IDE, Installer, or Shell. An empty composition
starts at Workshop discovery instead of trying to launch a missing root.

From Vapor Shell, `launch default` remains available as a CLI operation. It
verifies and launches the depot-owned Packagepack, Engine, and Game closure
without a Workshop download. A user-selected Workshop composition remains
mutable Library content in Vapor User Data.

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
root publish --account NAME --release-targets --yes
```

The Windows GNU/LLVM target can be built from Linux with Vapor-owned llvm-mingw,
and the Linux GNU target can be built from Windows with the Vapor-owned Zig
cross-link wrapper model. If a target is built on another machine, preserve the same
`bin/<target>/` and `output/dev/<workspace>/<target>/debug/` relative paths
when copying artifacts back to the publishing app root.

Workshop publication remains a separate workflow for community content. These
three first-party Loo-Cast products are delivered in the official Steam app and
are not published as Workshop dependencies.
