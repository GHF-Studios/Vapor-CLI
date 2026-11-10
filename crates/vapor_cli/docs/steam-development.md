# Loo-Cast Steam content release

The Loo-Cast Steam app delivers one Vapor composition: the Spacetime Engine,
Loo-Cast game, Loo-Cast Packagepack, and `App.vapor.toml` composition metadata.
Vapor is installed and updated independently. Steam does not distribute Vapor
executables, launchers, tools, source checkouts, documentation, caches, or
mutable state.

Steam is the content mount and delivery channel. Vapor home is the runtime and
management layer. Vapor reads the mounted composition, keeps its own caches and
state in its own directories, and launches the selected composition. Steam
uninstalling Loo-Cast does not uninstall Vapor.

## Manifest-owned payload

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

Run the package preview from the Vapor Client source workspace:

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

After reviewing the staged files and SteamPipe preview, publish explicitly:

```text
root publish --account NAME --release-targets --yes
```

SteamCMD prompts for authentication when needed. The service re-stages the
manifest-defined payload and compares it with the reviewed fingerprint before
upload. A changed or missing required product file invalidates the review.

Workshop publication remains a separate workflow for community content. These
three first-party Loo-Cast products are delivered in the official Steam app and
are not published as Workshop dependencies.
