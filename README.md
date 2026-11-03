# Vapor-CLI

Vapor-CLI is the terminal facade for Vapor. It owns the interactive shell,
command grammar, and terminal output. Reusable operations live in Vapor; SDK
authoring commands are available through `vapor sdk` in the same process.

The shipped terminal binary remains `vapor`.

## Temporary Platform SSH access

The client can submit a temporary SSH request for the current machine. An
operator must still approve the request in the Platform dashboard; this
command never grants access by itself.

1. In Identity, create an automation request credential with only the
   `platform:ssh-access:request` scope.
2. Enter it once through the hidden prompt:

   ```text
   vapor platform ssh credential set
   ```

3. Submit a request:

   ```text
   vapor platform ssh request
   ```

The CLI generates a dedicated Ed25519 key under the current user's Vapor data
directory. It sends only the public key; the private key remains local. The
default request is eight hours with deployment and service-control capabilities.
The `host-admin` capability must be requested alone and requires a separate
operator confirmation. Use `--duration-hours` and repeat `--capability` to
adjust the request. The CLI prints the request ID, key fingerprint, key path,
and dashboard link. Clear the request credential with
`vapor platform ssh credential clear`, or remove the local key with
`vapor platform ssh remove-key`.

The request credential is stored under the operating system user's Vapor data
directory with restrictive Unix permissions (and the inherited user profile
ACL on Windows). It is a request-only credential, not an SSH login key or an
admin credential. Set `VAPOR_PLATFORM_URL` only for a trusted HTTPS deployment
or a loopback development server.
