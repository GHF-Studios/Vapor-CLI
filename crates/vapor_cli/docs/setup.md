# Vapor installation and local setup

Normal closed-alpha testers should not run manual setup commands before first
launch. The Steam app starts through the platform entrypoint and uses the
packaged default App. First launch records Player readiness in Vapor User Data;
it does not fetch Rust, SteamCMD, or source.

## Ownership boundaries

- Vapor home owns Vapor executables, tools, source checkouts, caches, logs,
  configuration, generated output, and writable state.
- The Loo-Cast Steam app owns only `Packagepack.vapor.toml` in its common
  depot and one Packagepack executable with only declared required runtime
  libraries in the matching Linux or Windows depot. It does not install Vapor
  or a launcher shim.
- Mad Max remains a player-owned Steam installation. Vapor manages modding
  tools and packages around that local game; it does not distribute the base
  game.

Installing or removing either Steam game must not move, update, or uninstall
Vapor home. Installing or removing Vapor must not modify either game's owned
content.

## Installation roles

Vapor has two local roles:

- **Player** installs the runtime needed to discover and launch available
  content.
- **Developer** installs the runtime plus the required authoring tools,
  cross-target Rust toolchains, cross-linkers, and diagnostics tooling.

Developer setup is complete by definition; the user cannot choose a partial
Developer toolset. Ecosystem-level developer permissions are separate server
authorization and do not change the local installation role.

## Product mounting

Vapor treats Steam-installed products as external content mounts. Loo Cast's
mount has `Packagepack.vapor.toml` beside its executable. Vapor binaries,
tools, source checkouts, caches, docs, and mutable state never live in Steam.

`clone.rs` clones sources into `SuperWorkspace/sources/`.

## Installer shim

`vapor-installer` remains an app binary wrapper for launch scripts and old
command habits. It does not make the SuperWorkspace own tools.

The app root is disposable. Script-managed tools, caches, logs, receipts,
content Library state, and build output live in Vapor User Data. The app root
contains packaged binaries, launch/bootstrap scripts, documentation, and the
default App payload.
