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

The current Loo-Cast allowlist reads the Packagepack output from Vapor-home
`output/content/packages/` and stages its manifest in the common depot and its
target executable plus any declared runtime libraries in the matching OS
depot. Never include a general Vapor-home directory or a broad source/install
tree. Review a dry-run stage before real publication.
