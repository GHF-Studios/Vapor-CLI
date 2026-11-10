# Vapor home and product discovery

Vapor has three distinct path domains:

1. **Vapor home** owns the installed runtime, tools, writable data, caches,
   sources, and documentation.
2. **Product root** is an external game or app installation which Vapor has
   been explicitly asked to manage. For Loo-Cast this is the Steam content
   mount; it remains Steam-owned and ABI-specific.
3. **Source workspace** is a separately selected authoring checkout.

These roots are never interchangeable. In particular, `App.vapor.toml` in a
Steam product root describes that product and cannot identify Vapor home.

## Vapor home

The default home is `%USERPROFILE%\\Vapor` on Windows and `$HOME/Vapor` on
Linux. `VAPOR_HOME` selects another home explicitly. Every valid home contains
`Vapor-Home.toml` with the Vapor identity; discovery does not fall back to a
Steam installation.

```text
<Vapor home>/
├── Vapor-Home.toml
├── bin/<target>/                 installed Vapor applications and service
├── cache/build/<target>/         target-specific build cache
├── config/                       user configuration
├── content/                      installed Vapor-managed content
├── data/state/                   indexes, receipts, logs, and preferences
├── docs/                         installed documentation
├── examples/
├── metadata/toolchain.toml       pinned toolchain identity
├── output/                       generated output
├── resources/                    scripts and managed helpers
├── sources/                      developer source workspaces
└── tools/
    ├── cargo/{bin,home}/
    └── rustup/{bin,home}/
```

`Vapor-Home.toml` is the installation marker. A product's `App.vapor.toml`
belongs to the product root and is not copied into Vapor home as an identity
marker.

## Product root

Vapor currently receives explicit product context through
`VAPOR_PRODUCT_ROOT`. The root must contain an `App.vapor.toml` with a `[root]`
table. Automatic Steam product discovery and direct Steam launch-option
handoff are integration work still to be completed.
For Loo-Cast, the Steam app physically ships the Spacetime Engine, Loo-Cast
game, and Packagepack in its depots. Vapor reads those local files in place;
they are not downloaded from Workshop or the Vapor Artifact Server at runtime.
Steam remains their installer, updater, and uninstaller.

Mad Max follows a different ownership rule: Steam installs the publisher-owned
game, and Vapor may discover or accept its local directory. Vapor must not
package or distribute the base game.

## Source root selection

Starting Vapor discovers its home from `VAPOR_HOME` or the running executable's
location. Authoring source is selected independently:

1. `VAPOR_WORKSPACE`, when set;
2. the remembered path in `<Vapor home>/data/state/source-workspace`;
3. no source, leaving source-bound work unavailable until `source open PATH`.

`source open PATH` and `source add PATH` canonicalize the selected directory,
walk its ancestors, and choose the highest `[root]` or `[workspace]` marker.
`App-Source.vapor.toml` identifies a source/depot root; ordinary workspaces use
`Workspace.vapor.toml`. A standalone content manifest is not a source root.

Source workspaces must be disjoint from Vapor home. Product roots may be outside
both and are accessed only through explicit product context.

## Steam content mount

The Loo-Cast Steam app is a content-only mount. Its depots contain the
composition manifest and the three first-party products, split by ABI target.
No Vapor executable, launcher, tool, or handoff binary is part of the Steam
app root. Vapor independently discovers and mounts this content while owning
its own lifecycle and writable state. Removing Loo-Cast through Steam leaves
Vapor home and its user data intact.

Runtime executables, GUI applications, the service, toolchains, docs, sources,
cache, and writable state belong to Vapor home, never to the Loo-Cast Steam
depot. A direct Steam launch option into Vapor's tools selector still needs an
external integration mechanism; it must not be implemented by placing Vapor
files in the product content mount.

## User-facing commands

Run source-bound commands after selecting a source with `source open PATH`.
Vapor itself may be started from any working directory; its home is derived
from the installation marker and executable path, not the current directory.
