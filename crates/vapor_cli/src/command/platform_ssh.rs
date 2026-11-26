//! Request short-lived Platform SSH access through a one-time Identity browser authorization.

use super::{PlatformCommand, PlatformSshCapability, PlatformSshCommand};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, Instant},
};
use vapor_core::VaporInstallation;

const DEFAULT_PLATFORM_ORIGIN: &str = "https://vapor.ghf-studios.site";
const AUTHORIZATION_TIMEOUT: Duration = Duration::from_secs(10 * 60);

#[derive(Serialize)]
struct CreateRequest<'a> {
    public_key: &'a str,
    purpose: &'a str,
    requested_capabilities: Vec<&'static str>,
    requested_duration_seconds: i64,
}

#[derive(Serialize)]
struct StartAuthorization<'a> {
    ssh_fingerprint: &'a str,
    purpose: &'a str,
    requested_capabilities: Vec<&'static str>,
    requested_duration_seconds: i64,
}

#[derive(Deserialize)]
struct StartedAuthorization {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in_seconds: u64,
    poll_interval_seconds: u64,
}

#[derive(Serialize)]
struct PollAuthorization<'a> {
    device_code: &'a str,
}

#[derive(Deserialize)]
struct PolledAuthorization {
    status: String,
    credential_token: Option<String>,
    poll_interval_seconds: u64,
}

#[derive(Deserialize)]
struct CreatedRequest {
    id: String,
    fingerprint: String,
    status: String,
}

/// Execute a Platform SSH operation.
pub(crate) fn execute(command: PlatformCommand) -> Result<(), String> {
    let PlatformCommand::Ssh { command } = command;
    let installation = VaporInstallation::discover()
        .map_err(|error| format!("could not locate the Vapor installation: {error}"))?;
    let store = LocalStore::new(installation.user_data_root());

    match command {
        PlatformSshCommand::Request {
            purpose,
            duration_hours,
            capability,
        } => store.request(purpose, duration_hours, capability),
        PlatformSshCommand::RemoveKey => store.remove_key(),
    }
}

struct LocalStore {
    root: PathBuf,
}

impl LocalStore {
    fn new(user_data_root: PathBuf) -> Self {
        Self {
            root: user_data_root.join("platform/ssh-access"),
        }
    }

    fn key_path(&self) -> PathBuf {
        self.root.join("id_ed25519")
    }

    fn ensure_private_directory(&self) -> Result<(), String> {
        fs::create_dir_all(&self.root)
            .map_err(|error| format!("could not create {}: {error}", self.root.display()))?;
        set_private_directory_permissions(&self.root)?;
        Ok(())
    }

    fn remove_key(&self) -> Result<(), String> {
        let private = self.key_path();
        for path in [&private, &public_key_path(&private)] {
            match fs::remove_file(path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(format!("could not remove {}: {error}", path.display()));
                }
            }
        }
        println!("Local Platform SSH key removed. Any approved server grant remains until its approved expiry or operator revocation.");
        Ok(())
    }

    fn request(
        &self,
        purpose: String,
        duration_hours: u8,
        capability: Vec<PlatformSshCapability>,
    ) -> Result<(), String> {
        if purpose.trim().is_empty() {
            return Err("the SSH request purpose cannot be empty".to_owned());
        }
        if capability.is_empty() {
            return Err("select at least one SSH capability".to_owned());
        }
        if capability.contains(&PlatformSshCapability::HostAdmin) && capability.len() != 1 {
            return Err("`host-admin` must be requested by itself; the operator requires a separate confirmation for it".to_owned());
        }

        self.ensure_private_directory()?;
        let private_key = self.ensure_key_pair()?;
        let public_path = public_key_path(&private_key);
        let public_key = fs::read_to_string(&public_path)
            .map_err(|error| format!("could not read the SSH public key: {error}"))?;
        let fingerprint = ssh_fingerprint(&public_path)?;
        let origin = platform_origin()?;
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(20))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| format!("could not prepare a secure Vapor connection: {error}"))?;
        let requested_capabilities = capability
            .into_iter()
            .map(PlatformSshCapability::as_api_value)
            .collect::<Vec<_>>();
        let duration_seconds = i64::from(duration_hours) * 60 * 60;

        let authorization = client
            .post(format!(
                "{origin}/api/identity/v1/cli/ssh-authorization/start"
            ))
            .json(&StartAuthorization {
                ssh_fingerprint: &fingerprint,
                purpose: purpose.trim(),
                requested_capabilities: requested_capabilities.clone(),
                requested_duration_seconds: duration_seconds,
            })
            .send()
            .map_err(|error| {
                format!("could not start browser authorization with Vapor Identity: {error}")
            })?;
        let authorization_status = authorization.status();
        if authorization_status == reqwest::StatusCode::NOT_FOUND
            || authorization_status == reqwest::StatusCode::METHOD_NOT_ALLOWED
        {
            return Err("the contacted Vapor Identity service does not expose the CLI SSH authorization flow yet; no SSH request was created. The local key is retained for retry. Update the Platform and Identity services, then retry".to_owned());
        }
        if !authorization_status.is_success() {
            return Err(format!(
                "Vapor Identity could not start SSH request authorization ({}): {}",
                authorization_status,
                limited_body(authorization)
            ));
        }
        let started: StartedAuthorization = authorization.json().map_err(|error| {
            format!("Vapor Identity returned an unreadable authorization response: {error}")
        })?;

        println!("Authorize this exact SSH request in your browser:");
        println!("  Key: {fingerprint}");
        println!("  Purpose: {}", purpose.trim());
        println!("  Duration: {duration_hours} hour(s)");
        println!("  Approve request: {}", started.verification_uri);
        println!("  Authorization code: {}", started.user_code);
        if let Err(error) = open_browser(&started.verification_uri) {
            eprintln!("Could not open a browser automatically: {error}");
        }

        let token = self.wait_for_authorization(&client, &origin, &started)?;
        let request = CreateRequest {
            public_key: public_key.trim(),
            purpose: purpose.trim(),
            requested_capabilities,
            requested_duration_seconds: duration_seconds,
        };
        let response = client
            .post(format!("{origin}/api/ssh-access/requests"))
            .bearer_auth(&token)
            .json(&request)
            .send()
            .map_err(|error| {
                format!("could not submit the SSH request to Vapor Platform: {error}")
            })?;
        if !response.status().is_success() {
            let status = response.status();
            let detail = limited_body(response);
            return Err(format!(
                "Platform rejected the SSH request ({status}): {detail}. Run the request again to authorize a fresh one-time request."
            ));
        }
        let created: CreatedRequest = response.json().map_err(|error| {
            format!("Platform accepted the request but returned an unreadable response: {error}")
        })?;

        println!("Temporary SSH request submitted for operator approval.");
        println!("  Request: {}", created.id);
        println!("  Status: {}", created.status);
        println!("  Key fingerprint: {}", created.fingerprint);
        println!("  Local private key: {}", private_key.display());
        println!("  Review and approve: {origin}/admin/operations#ssh-access");
        println!("  Requested duration: {duration_hours} hour(s)");
        println!(
            "  Requested capabilities: {}",
            request.requested_capabilities.join(", ")
        );
        Ok(())
    }

    fn wait_for_authorization(
        &self,
        client: &reqwest::blocking::Client,
        origin: &str,
        started: &StartedAuthorization,
    ) -> Result<String, String> {
        let deadline = Instant::now()
            + AUTHORIZATION_TIMEOUT.min(Duration::from_secs(started.expires_in_seconds));
        let mut poll_interval = Duration::from_secs(started.poll_interval_seconds.max(1));
        println!("Waiting for the one-time browser authorization...");
        loop {
            if Instant::now() >= deadline {
                return Err("browser authorization expired; run the SSH request again".to_owned());
            }
            thread::sleep(poll_interval);
            let response = client
                .post(format!(
                    "{origin}/api/identity/v1/cli/ssh-authorization/poll"
                ))
                .json(&PollAuthorization {
                    device_code: &started.device_code,
                })
                .send()
                .map_err(|error| format!("could not check the browser authorization: {error}"))?;
            if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                continue;
            }
            if !response.status().is_success() {
                return Err(format!(
                    "Vapor Identity authorization polling failed ({}): {}",
                    response.status(),
                    limited_body(response)
                ));
            }
            let result: PolledAuthorization = response.json().map_err(|error| {
                format!("Vapor Identity returned an unreadable authorization status: {error}")
            })?;
            poll_interval = Duration::from_secs(result.poll_interval_seconds.max(1));
            match result.status.as_str() {
                "pending" => continue,
                "denied" => return Err("SSH request authorization was denied".to_owned()),
                "approved" => {
                    return result.credential_token.ok_or_else(|| {
                        "authorization completed without its one-time request token".to_owned()
                    });
                }
                _ => {
                    return Err("Vapor Identity returned an unknown authorization status".to_owned())
                }
            }
        }
    }

    fn ensure_key_pair(&self) -> Result<PathBuf, String> {
        let private = self.key_path();
        let public = public_key_path(&private);
        if private.is_file() && public.is_file() {
            validate_public_key(&public)?;
            return Ok(private);
        }
        if private.exists() && !public.exists() {
            let output = Command::new("ssh-keygen")
                .arg("-y")
                .arg("-f")
                .arg(&private)
                .output()
                .map_err(|error| {
                    format!("could not run OpenSSH `ssh-keygen` to restore the public key: {error}")
                })?;
            if !output.status.success() {
                return Err("the stored SSH private key could not be read by OpenSSH; remove it with `vapor platform ssh remove-key` and retry".to_owned());
            }
            write_private_file(&public, &output.stdout)?;
            validate_public_key(&public)?;
            return Ok(private);
        }
        if private.exists() || public.exists() {
            return Err("the local SSH key files are incomplete; remove them with `vapor platform ssh remove-key` and retry".to_owned());
        }
        let output = Command::new("ssh-keygen")
            .args([
                "-q",
                "-t",
                "ed25519",
                "-N",
                "",
                "-C",
                "Vapor temporary Platform access",
                "-f",
            ])
            .arg(&private)
            .output()
            .map_err(|error| {
                format!(
                    "OpenSSH `ssh-keygen` is required to create the dedicated access key: {error}"
                )
            })?;
        if !output.status.success() {
            let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            return Err(format!(
                "could not create the dedicated SSH key{}",
                if detail.is_empty() {
                    String::new()
                } else {
                    format!(": {detail}")
                }
            ));
        }
        set_private_file_permissions(&private)?;
        validate_public_key(&public)?;
        Ok(private)
    }
}

fn platform_origin() -> Result<String, String> {
    let configured =
        std::env::var("VAPOR_PLATFORM_URL").unwrap_or_else(|_| DEFAULT_PLATFORM_ORIGIN.to_owned());
    let url = reqwest::Url::parse(configured.trim_end_matches('/'))
        .map_err(|_| "VAPOR_PLATFORM_URL must be a valid origin URL".to_owned())?;
    let local_host = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "::1"));
    let is_local_http = url.scheme() == "http" && local_host;
    if (url.scheme() != "https" && !is_local_http)
        || !url.username().is_empty()
        || url.password().is_some()
        || !matches!(url.path(), "" | "/")
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("VAPOR_PLATFORM_URL must be a plain HTTPS origin (HTTP is allowed only for loopback development)".to_owned());
    }
    Ok(url.origin().ascii_serialization())
}

fn ssh_fingerprint(public_key: &Path) -> Result<String, String> {
    let output = Command::new("ssh-keygen")
        .args(["-l", "-E", "sha256", "-f"])
        .arg(public_key)
        .output()
        .map_err(|error| format!("could not inspect the SSH public key with OpenSSH: {error}"))?;
    if !output.status.success() {
        return Err("OpenSSH could not calculate the SSH key fingerprint".to_owned());
    }
    let output = String::from_utf8_lossy(&output.stdout);
    output
        .split_whitespace()
        .find(|part| part.starts_with("SHA256:"))
        .map(str::to_owned)
        .ok_or_else(|| "OpenSSH returned no SHA256 key fingerprint".to_owned())
}

fn open_browser(url: &str) -> Result<(), String> {
    #[cfg(windows)]
    let result = Command::new("rundll32.exe")
        .args(["url.dll,FileProtocolHandler", url])
        .spawn();
    #[cfg(target_os = "macos")]
    let result = Command::new("open").arg(url).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let result = Command::new("xdg-open").arg(url).spawn();
    result.map(|_| ()).map_err(|error| error.to_string())
}

fn limited_body(response: reqwest::blocking::Response) -> String {
    response
        .text()
        .unwrap_or_default()
        .trim()
        .chars()
        .take(300)
        .collect()
}

fn public_key_path(private_key: &Path) -> PathBuf {
    PathBuf::from(format!("{}.pub", private_key.display()))
}

fn validate_public_key(path: &Path) -> Result<(), String> {
    let key = fs::read_to_string(path)
        .map_err(|error| format!("could not read SSH public key {}: {error}", path.display()))?;
    if !key.trim_start().starts_with("ssh-ed25519 ") {
        return Err(format!("{} is not an ssh-ed25519 key", path.display()));
    }
    Ok(())
}

fn write_private_file(path: &Path, contents: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary).map_err(|error| {
        format!(
            "could not create protected file {}: {error}",
            temporary.display()
        )
    })?;
    if let Err(error) = file.write_all(contents).and_then(|()| file.sync_all()) {
        let _ = fs::remove_file(&temporary);
        return Err(format!(
            "could not securely write {}: {error}",
            path.display()
        ));
    }
    set_private_file_permissions(&temporary)?;
    if path.exists() {
        fs::remove_file(path)
            .map_err(|error| format!("could not replace {}: {error}", path.display()))?;
    }
    fs::rename(&temporary, path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        format!("could not store {}: {error}", path.display())
    })
}

fn set_private_directory_permissions(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|error| format!("could not protect {}: {error}", path.display()))?;
    }
    // On Windows this lives below LocalAppData, whose inherited ACL is scoped
    // to the current user and the local system.
    #[cfg(windows)]
    let _ = path;
    Ok(())
}

fn set_private_file_permissions(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("could not protect {}: {error}", path.display()))?;
    }
    // The key inherits the user-only LocalAppData ACL on Windows.
    #[cfg(windows)]
    let _ = path;
    Ok(())
}
