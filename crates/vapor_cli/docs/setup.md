# Vapor installation and local setup

Vapor is an independently installed application. Its installer owns the
Vapor home, update, repair, and uninstall lifecycle. Steam does not install or
remove Vapor.

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

Use Vapor's standalone Installer to select or repair the installation role.
Use Vapor Launcher to choose an installed product composition. Do not run
legacy Steam-root setup scripts or place Vapor-owned files in a game's install
directory.
