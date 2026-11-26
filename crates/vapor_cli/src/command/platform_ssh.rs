//! Temporary Platform SSH request flow.
//!
//! The private identity remains local. Identity confirms the exact key and
//! request, then issues a short-lived single-use credential that Platform
//! binds to the submitted request tuple.

use super::ShellState;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const DEFAULT_PLATFORM_ORIGIN: &str = "https://vapor.ghf-studios.site";
const ALLOWED_CAPABILITIES: &[&str] = &["vapor_deploy", "vapor_service_control", "host_admin"];

#[derive(Serialize)]
struct StartRequest<'a> {
    ssh_fingerprint: &'a str,
    purpose: &'a str,
    requested_capabilities: &'a [String],
    requested_duration_seconds: u32,
}

#[derive(Deserialize)]
struct StartResponse {
    device_code: String,
    verification_uri: String,
    expires_in_seconds: u64,
    poll_interval_seconds: u64,
}

#[derive(Serialize)]
struct PollRequest<'a> {
    device_code: &'a str,
}

#[derive(Deserialize)]
struct PollResponse {
    status: String,
    credential_token: Option<String>,
}

#[derive(Serialize)]
struct PlatformRequest<'a> {
    public_key: &'a str,
    purpose: &'a str,
    requested_capabilities: &'a [String],
    requested_duration_seconds: u32,
}

#[derive(Deserialize)]
struct CreatedRequest {
    id: String,
    status: String,
}

/// Run the Identity-confirmed SSH access request flow.
pub(crate) fn execute_platform_ssh_request(
    purpose: &str,
    capabilities: &[String],
    duration_hours: u8,
    identity_file: Option<&Path>,
    state: &ShellState,
) -> Result<(), String> {
    validate_request(purpose, capabilities, duration_hours)?;
    let identity = select_identity(identity_file, &state.installation().user_data_root())?;
    let (public_key, fingerprint) =
        read_ed25519_public_key(&identity.private_key, &identity.public_key)?;
    let origin = platform_origin()?;
    let client = reqwest::blocking::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|_| "could not initialize secure Platform connection".to_owned())?;
    let seconds = u32::from(duration_hours) * 3600;
    let start: StartResponse = post_json(
        &client,
        &format!("{origin}/api/identity/v1/cli/ssh-authorization/start"),
        &StartRequest {
            ssh_fingerprint: &fingerprint,
            purpose: purpose.trim(),
            requested_capabilities: capabilities,
            requested_duration_seconds: seconds,
        },
        reqwest::StatusCode::CREATED,
    )?;
    if start.device_code.is_empty()
        || start.expires_in_seconds == 0
        || start.expires_in_seconds > 10 * 60
        || start.poll_interval_seconds == 0
        || !valid_verification_uri(&start.verification_uri, &origin)
    {
        return Err("Identity returned an invalid authorization response".to_owned());
    }

    println!("SSH identity: {}", identity.private_key.display());
    println!("Fingerprint: {fingerprint}");
    println!(
        "Confirm this exact request in Vapor Identity: {}",
        start.verification_uri
    );
    if let Err(error) = open_verification_page(&start.verification_uri) {
        eprintln!("Warning: {error}");
    }

    let deadline = Instant::now() + Duration::from_secs(start.expires_in_seconds);
    let credential = loop {
        if Instant::now() >= deadline {
            return Err("Identity authorization expired before confirmation".to_owned());
        }
        thread::sleep(Duration::from_secs(start.poll_interval_seconds.min(30) + 1));
        let response: PollResponse = post_json(
            &client,
            &format!("{origin}/api/identity/v1/cli/ssh-authorization/poll"),
            &PollRequest {
                device_code: &start.device_code,
            },
            reqwest::StatusCode::OK,
        )?;
        match poll_decision(response)? {
            PollDecision::Pending => continue,
            PollDecision::Denied => {
                return Err("SSH request was denied in Vapor Identity".to_owned());
            }
            PollDecision::Approved(token) => break token,
        }
    };

    let response = client
        .post(format!("{origin}/api/ssh-access/requests"))
        .bearer_auth(&credential)
        .json(&PlatformRequest {
            public_key: &public_key,
            purpose: purpose.trim(),
            requested_capabilities: capabilities,
            requested_duration_seconds: seconds,
        })
        .send()
        .map_err(|_| {
            "Platform request submission failed; the credential was not displayed".to_owned()
        })?;
    if response.status() != reqwest::StatusCode::ACCEPTED {
        return Err(format!(
            "Platform rejected the authorized request (HTTP {})",
            response.status().as_u16()
        ));
    }
    let created: CreatedRequest = response
        .json()
        .map_err(|_| "Platform returned an invalid request response".to_owned())?;
    if created.id.is_empty() || created.status != "pending" {
        return Err("Platform returned an invalid request status".to_owned());
    }
    println!("SSH request {} is pending root approval.", created.id);
    println!("Review it at {origin}/admin/approvals#pending-ssh-approvals");
    Ok(())
}

struct IdentityPaths {
    private_key: PathBuf,
    public_key: PathBuf,
}

fn select_identity(
    identity_file: Option<&Path>,
    user_data_root: &Path,
) -> Result<IdentityPaths, String> {
    let private_key = identity_file
        .map(Path::to_path_buf)
        .unwrap_or_else(|| user_data_root.join("ssh").join("platform_ed25519"));
    let public_key = PathBuf::from(format!("{}.pub", private_key.display()));
    if identity_file.is_some() {
        if !private_key.is_file() || !public_key.is_file() {
            return Err("the selected identity and its .pub file must already exist".to_owned());
        }
    } else {
        match classify_identity_pair(private_key.is_file(), public_key.is_file()) {
            IdentityPairState::Complete => {}
            IdentityPairState::Incomplete => {
                return Err(
                    "Vapor SSH identity files are incomplete; refusing to overwrite either file"
                        .to_owned(),
                );
            }
            IdentityPairState::Missing => {
                let parent = private_key
                    .parent()
                    .ok_or_else(|| "invalid identity path".to_owned())?;
                fs::create_dir_all(parent)
                    .map_err(|_| "could not create Vapor SSH identity directory".to_owned())?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
                        .map_err(|_| "could not secure Vapor SSH identity directory".to_owned())?;
                }
                let output = Command::new("ssh-keygen")
                    .args([
                        "-q",
                        "-t",
                        "ed25519",
                        "-N",
                        "",
                        "-C",
                        "vapor-platform-request",
                        "-f",
                    ])
                    .arg(&private_key)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .map_err(|_| {
                        "OpenSSH ssh-keygen is required to create the Vapor-managed identity"
                            .to_owned()
                    })?;
                if !output.success() {
                    return Err("ssh-keygen could not create a Vapor-managed identity".to_owned());
                }
            }
        }
    }
    Ok(IdentityPaths {
        private_key,
        public_key,
    })
}

fn read_ed25519_public_key(
    private_path: &Path,
    public_path: &Path,
) -> Result<(String, String), String> {
    let contents =
        fs::read_to_string(public_path).map_err(|_| "could not read SSH public key".to_owned())?;
    let key = validate_public_key_line(&contents)?;
    let key = key.as_str();
    let output = Command::new("ssh-keygen")
        .args(["-lf"])
        .arg(public_path)
        .args(["-E", "sha256"])
        .output()
        .map_err(|_| "OpenSSH ssh-keygen is required to validate the identity".to_owned())?;
    if !output.status.success() {
        return Err("ssh-keygen rejected the selected public key".to_owned());
    }
    let details = String::from_utf8_lossy(&output.stdout);
    let fingerprint = parse_ssh_key_fingerprint(&details)?;
    let private = Command::new("ssh-keygen")
        .args(["-y", "-f"])
        .arg(private_path)
        .stdin(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .map_err(|_| {
            "OpenSSH ssh-keygen is required to validate the private identity".to_owned()
        })?;
    if !private.status.success() {
        return Err(
            "could not unlock the selected private key; use a valid OpenSSH identity".to_owned(),
        );
    }
    let derived = String::from_utf8_lossy(&private.stdout);
    let expected_fields: Vec<_> = key.split_ascii_whitespace().take(2).collect();
    let derived_fields: Vec<_> = derived.split_ascii_whitespace().take(2).collect();
    if expected_fields != derived_fields {
        return Err("the selected private and public SSH keys do not match".to_owned());
    }
    Ok((key.to_owned(), fingerprint))
}

#[derive(Debug, PartialEq, Eq)]
enum IdentityPairState {
    Missing,
    Complete,
    Incomplete,
}

fn classify_identity_pair(private_exists: bool, public_exists: bool) -> IdentityPairState {
    match (private_exists, public_exists) {
        (false, false) => IdentityPairState::Missing,
        (true, true) => IdentityPairState::Complete,
        _ => IdentityPairState::Incomplete,
    }
}

fn parse_ssh_key_fingerprint(details: &str) -> Result<String, String> {
    let values: Vec<_> = details.split_ascii_whitespace().collect();
    if values.len() < 4
        || values[0] != "256"
        || !values[1].starts_with("SHA256:")
        || values.last() != Some(&"(ED25519)")
    {
        return Err("selected SSH identity is not a supported Ed25519 key".to_owned());
    }
    Ok(values[1].to_owned())
}

fn validate_public_key_line(contents: &str) -> Result<String, String> {
    let key = contents.trim();
    let fields: Vec<_> = key.split_ascii_whitespace().collect();
    if fields.len() < 2 || fields[0] != "ssh-ed25519" || key.contains('\n') || key.contains('\r') {
        return Err("SSH public key must contain one OpenSSH Ed25519 key".to_owned());
    }
    Ok(key.to_owned())
}

fn validate_request(
    purpose: &str,
    capabilities: &[String],
    duration_hours: u8,
) -> Result<(), String> {
    let purpose = purpose.trim();
    if purpose.is_empty() || purpose.len() > 500 || purpose.chars().any(char::is_control) {
        return Err("purpose must be plain text of 1–500 bytes".to_owned());
    }
    if !(1..=8).contains(&duration_hours) {
        return Err("requested duration must be from 1 to 8 hours".to_owned());
    }
    if capabilities.is_empty()
        || capabilities.len() > ALLOWED_CAPABILITIES.len()
        || capabilities
            .iter()
            .any(|cap| !ALLOWED_CAPABILITIES.contains(&cap.as_str()))
        || capabilities
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len()
            != capabilities.len()
        || (capabilities.iter().any(|cap| cap == "host_admin") && capabilities.len() != 1)
    {
        return Err("capabilities are unsupported, duplicated, or conflicting".to_owned());
    }
    Ok(())
}

fn platform_origin() -> Result<String, String> {
    let raw = std::env::var("VAPOR_PLATFORM_BASE_URL")
        .unwrap_or_else(|_| DEFAULT_PLATFORM_ORIGIN.to_owned());
    let url = reqwest::Url::parse(&raw)
        .map_err(|_| "VAPOR_PLATFORM_BASE_URL must be an HTTPS origin".to_owned())?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != "/"
    {
        return Err(
            "VAPOR_PLATFORM_BASE_URL must be an HTTPS origin without a path, query, or fragment"
                .to_owned(),
        );
    }
    Ok(url.origin().ascii_serialization())
}

fn valid_verification_uri(uri: &str, expected_origin: &str) -> bool {
    reqwest::Url::parse(uri).is_ok_and(|url| {
        url.scheme() == "https"
            && url.origin().ascii_serialization() == expected_origin
            && url.path() == "/cli/ssh/authorize"
            && url
                .query_pairs()
                .any(|(key, value)| key == "user_code" && !value.is_empty())
            && url.fragment().is_none()
    })
}

fn post_json<T: Serialize, R: for<'de> Deserialize<'de>>(
    client: &reqwest::blocking::Client,
    url: &str,
    payload: &T,
    expected_status: reqwest::StatusCode,
) -> Result<R, String> {
    let response = client
        .post(url)
        .json(payload)
        .send()
        .map_err(|_| "secure Identity request failed".to_owned())?;
    validate_http_status(response.status(), expected_status)?;
    response
        .json()
        .map_err(|_| "Identity returned an invalid response".to_owned())
}

fn validate_http_status(
    status: reqwest::StatusCode,
    expected: reqwest::StatusCode,
) -> Result<(), String> {
    if status == expected {
        Ok(())
    } else if status == reqwest::StatusCode::GONE {
        Err("Identity authorization expired before confirmation".to_owned())
    } else if status.is_redirection() {
        Err("Identity redirected a request; refusing to follow it".to_owned())
    } else {
        Err(format!(
            "Identity request failed (HTTP {})",
            status.as_u16()
        ))
    }
}

enum PollDecision {
    Pending,
    Denied,
    Approved(String),
}

fn poll_decision(response: PollResponse) -> Result<PollDecision, String> {
    match response.status.as_str() {
        "pending" if response.credential_token.is_none() => Ok(PollDecision::Pending),
        "denied" if response.credential_token.is_none() => Ok(PollDecision::Denied),
        "approved" => response
            .credential_token
            .filter(|token| !token.is_empty())
            .map(PollDecision::Approved)
            .ok_or_else(|| {
                "Identity approved the request but returned no one-time credential".to_owned()
            }),
        _ => Err("Identity returned an invalid authorization status".to_owned()),
    }
}

fn open_verification_page(uri: &str) -> Result<(), String> {
    let mut command = if cfg!(target_os = "windows") {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", ""]);
        command
    } else if cfg!(target_os = "macos") {
        Command::new("open")
    } else {
        Command::new("xdg-open")
    };
    command
        .arg(uri)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| {
            "could not open the Identity confirmation page; open the displayed URL manually"
                .to_owned()
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        IdentityPairState, PlatformRequest, PollDecision, PollResponse, StartRequest,
        classify_identity_pair, parse_ssh_key_fingerprint, poll_decision, valid_verification_uri,
        validate_http_status, validate_public_key_line, validate_request,
    };
    use serde_json::json;

    #[test]
    fn verification_uri_must_be_identity_confirmation_on_same_https_origin() {
        assert!(valid_verification_uri(
            "https://vapor.ghf-studios.site/cli/ssh/authorize?user_code=ABC",
            "https://vapor.ghf-studios.site"
        ));
        assert!(!valid_verification_uri(
            "http://vapor.ghf-studios.site/cli/ssh/authorize?user_code=ABC",
            "https://vapor.ghf-studios.site"
        ));
        assert!(!valid_verification_uri(
            "https://evil.example/cli/ssh/authorize?user_code=ABC",
            "https://vapor.ghf-studios.site"
        ));
        assert!(!valid_verification_uri(
            "https://vapor.ghf-studios.site/other?user_code=ABC",
            "https://vapor.ghf-studios.site"
        ));
    }

    #[test]
    fn authorization_results_gate_request_submission() {
        assert!(matches!(
            poll_decision(PollResponse {
                status: "pending".into(),
                credential_token: None
            })
            .unwrap(),
            PollDecision::Pending
        ));
        assert!(matches!(
            poll_decision(PollResponse {
                status: "denied".into(),
                credential_token: None
            })
            .unwrap(),
            PollDecision::Denied
        ));
        assert!(
            matches!(poll_decision(PollResponse { status: "approved".into(), credential_token: Some("one-time".into()) }).unwrap(), PollDecision::Approved(token) if token == "one-time")
        );
        assert!(
            poll_decision(PollResponse {
                status: "approved".into(),
                credential_token: None
            })
            .is_err()
        );
        assert!(
            poll_decision(PollResponse {
                status: "expired".into(),
                credential_token: None
            })
            .is_err()
        );
    }

    #[test]
    fn expired_and_redirected_identity_responses_fail_closed() {
        assert!(
            validate_http_status(reqwest::StatusCode::GONE, reqwest::StatusCode::OK)
                .unwrap_err()
                .contains("expired")
        );
        assert!(
            validate_http_status(reqwest::StatusCode::FOUND, reqwest::StatusCode::OK)
                .unwrap_err()
                .contains("redirected")
        );
        assert!(
            validate_http_status(reqwest::StatusCode::OK, reqwest::StatusCode::CREATED).is_err()
        );
    }

    #[test]
    fn identity_and_platform_receive_the_same_bound_request_fields() {
        let purpose = "inspect release config";
        let capabilities = vec!["host_admin".to_owned()];
        let start = StartRequest {
            ssh_fingerprint: "SHA256:example",
            purpose,
            requested_capabilities: &capabilities,
            requested_duration_seconds: 3600,
        };
        let submit = PlatformRequest {
            public_key: "ssh-ed25519 AAAA...",
            purpose,
            requested_capabilities: &capabilities,
            requested_duration_seconds: 3600,
        };
        let start_json = serde_json::to_value(start).unwrap();
        let submit_json = serde_json::to_value(submit).unwrap();
        assert_eq!(
            start_json,
            json!({ "ssh_fingerprint": "SHA256:example", "purpose": purpose, "requested_capabilities": capabilities, "requested_duration_seconds": 3600 })
        );
        assert_eq!(
            submit_json,
            json!({ "public_key": "ssh-ed25519 AAAA...", "purpose": purpose, "requested_capabilities": capabilities, "requested_duration_seconds": 3600 })
        );
    }

    #[test]
    fn request_validation_rejects_unsupported_or_conflicting_capabilities() {
        assert!(validate_request("inspect", &["unknown".into()], 1).is_err());
        assert!(
            validate_request("inspect", &["host_admin".into(), "vapor_deploy".into()], 1).is_err()
        );
        assert!(validate_request("inspect", &["host_admin".into()], 1).is_ok());
    }

    #[test]
    fn public_key_preflight_rejects_unsupported_and_multiline_keys() {
        assert!(validate_public_key_line("ssh-rsa AAAA comment").is_err());
        assert!(validate_public_key_line("ssh-ed25519 AAAA\nssh-ed25519 BBBB").is_err());
    }

    #[test]
    fn ssh_keygen_fingerprint_allows_a_comment_before_key_type() {
        assert_eq!(
            parse_ssh_key_fingerprint("256 SHA256:abc vapor-platform-request (ED25519)").unwrap(),
            "SHA256:abc"
        );
    }

    #[test]
    fn default_identity_reuses_complete_pair_and_rejects_partial_pair() {
        assert_eq!(
            classify_identity_pair(false, false),
            IdentityPairState::Missing
        );
        assert_eq!(
            classify_identity_pair(true, true),
            IdentityPairState::Complete
        );
        assert_eq!(
            classify_identity_pair(true, false),
            IdentityPairState::Incomplete
        );
        assert_eq!(
            classify_identity_pair(false, true),
            IdentityPairState::Incomplete
        );
    }
}
