# Vapor installation and local setup

Vapor is an independently installed application. Its installer owns the
Vapor home, update, repair, and uninstall lifecycle. Steam does not install or
remove Vapor.

## Ownership boundaries

- Vapor home owns Vapor executables, tools, source checkouts, caches, logs,
  configuration, generated output, and writable state.
- The Loo-Cast Steam app owns only its composition manifest and the three
  first-party content products: Spacetime Engine, Loo-Cast game, and Loo-Cast
  Packagepack. The ABI-specific product files are delivered from matching
  Linux and Windows depots.
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

Vapor treats Steam-installed products as external content mounts. The current
Loo-Cast product context is supplied with `VAPOR_PRODUCT_ROOT` and must point
to a directory containing `App.vapor.toml`. Product discovery and any direct
Steam launch-option handoff into Vapor remain separate integration work; no
Vapor binary or launch adapter may be added to the Loo-Cast Steam depots to
implement them.

Use Vapor's standalone Installer to select or repair the installation role.
Use Vapor Launcher to choose an installed product composition. Do not run
legacy Steam-root setup scripts or place Vapor-owned files in a game's install
directory.
