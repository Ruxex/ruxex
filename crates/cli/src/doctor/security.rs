#[cfg(target_os = "macos")]
use std::ffi::OsStr;
#[cfg(target_os = "macos")]
use std::process::Output;
#[cfg(target_os = "macos")]
use std::process::Stdio;
#[cfg(target_os = "macos")]
use std::time::Duration;

#[cfg(target_os = "macos")]
use tokio::process::Command;
#[cfg(target_os = "macos")]
use tokio::time::timeout;

use super::CheckStatus;
use super::DoctorCheck;
use super::DoctorIssue;

#[cfg(target_os = "macos")]
const PRODUCT_QUERY_TIMEOUT: Duration = Duration::from_secs(5);
#[cfg(target_os = "macos")]
const MAX_PRODUCT_OUTPUT_BYTES: usize = 64 * 1024;

pub(super) enum EndpointInspection {
    Complete(Vec<&'static str>),
    Partial(Vec<&'static str>),
    Unavailable,
}

pub(super) async fn check() -> DoctorCheck {
    endpoint_check(endpoint_products().await)
}

pub(super) fn endpoint_check(inspection: EndpointInspection) -> DoctorCheck {
    let (products, visibility_incomplete) = match inspection {
        EndpointInspection::Complete(products) => (products, false),
        EndpointInspection::Partial(products) => (products, true),
        EndpointInspection::Unavailable => {
            return DoctorCheck::new(
                "security.endpoint",
                "security",
                CheckStatus::Warning,
                "endpoint protection inspection unavailable",
            )
            .detail("endpoint products: unavailable");
        }
    };

    if products.is_empty() {
        return DoctorCheck::new(
            "security.endpoint",
            "security",
            CheckStatus::Ok,
            "no supported endpoint protection detected",
        )
        .detail("endpoint products: none detected");
    }

    let targets = if cfg!(target_os = "macos") {
        "signing team 2DC432GLL2 plus the installed Genex app identity, signed genex agent, and required helpers"
    } else {
        "verified Genex app, genex agent, and required helpers"
    };
    let mut check = DoctorCheck::new(
        "security.endpoint",
        "security",
        CheckStatus::Warning,
        "endpoint protection detected; Genex exclusions are unverified",
    )
    .detail(format!("endpoint products: {}", products.join(", ")))
    .detail(format!("exclusion targets: {targets}"))
    .detail("Genex exclusions: not verified")
    .remediation(
        "ask your security administrator to verify Genex exclusions and required helper allowances",
    );
    if visibility_incomplete {
        check = check.detail("additional endpoint products: unavailable");
    }

    for product in products {
        let remedy = match product {
            "CrowdStrike Falcon" => {
                "CrowdStrike Falcon: Add a certificate or IOA exclusion for Genex and monitor descendant processes."
            }
            "BeyondTrust Privilege Management" => {
                "BeyondTrust: Remove Genex from application blocking rules and allow Genex helper executables."
            }
            "Microsoft Defender" => {
                "Microsoft Defender: Add a certificate or executable-path exclusion for Genex and its helpers."
            }
            "SentinelOne" => {
                "SentinelOne: Add a signer, file-hash, or executable-path exclusion for Genex and its helpers."
            }
            "Jamf Protect" => {
                "Jamf Protect: Add an exception for Genex app and helper signing identities."
            }
            _ => {
                "Add an exclusion for Genex and its helpers. Use the endpoint product instructions."
            }
        };
        check = check.issue(
            DoctorIssue::new(
                CheckStatus::Warning,
                format!("{product} can interfere with Genex. Verify Genex exclusions."),
            )
            .measured("not verified")
            .expected("Genex application and helper exclusions")
            .remedy(remedy)
            .field("Genex exclusions"),
        );
    }

    check
}

async fn endpoint_products() -> EndpointInspection {
    #[cfg(target_os = "macos")]
    {
        const PRODUCTS: &[(&str, &str, &str)] = &[
            (
                "CrowdStrike Falcon",
                "X9E956P446",
                "com.crowdstrike.falcon.Agent",
            ),
            (
                "BeyondTrust Privilege Management",
                "2ZS8T6NYB8",
                "com.beyondtrust.endpointsecurity",
            ),
            (
                "Microsoft Defender",
                "UBF8T346G9",
                "com.microsoft.wdav.epsext",
            ),
            (
                "SentinelOne",
                "4AYE5J54KN",
                "com.sentinelone.network-monitoring",
            ),
            (
                "Jamf Protect",
                "483DWKW443",
                "com.jamf.protect.security-extension",
            ),
        ];
        let Some(output) = product_command("/usr/bin/systemextensionsctl", &["list"])
            .await
            .filter(|output| output.status.success())
        else {
            return EndpointInspection::Unavailable;
        };
        return EndpointInspection::Complete(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|line| {
                    let mut columns = line.split_whitespace();
                    if columns.next() != Some("*") || columns.next() != Some("*") {
                        return None;
                    }
                    let team = columns.next()?;
                    let bundle = columns.next()?;
                    PRODUCTS.iter().find_map(|&(name, signer, identifier)| {
                        (team == signer && bundle == identifier).then_some(name)
                    })
                })
                .collect(),
        );
    }

    EndpointInspection::Complete(Vec::new())
}

#[cfg(target_os = "macos")]
async fn product_command(program: impl AsRef<OsStr>, args: &[&str]) -> Option<Output> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    timeout(PRODUCT_QUERY_TIMEOUT, command.output())
        .await
        .ok()?
        .ok()
        .filter(|output| output.stdout.len() <= MAX_PRODUCT_OUTPUT_BYTES)
}

#[cfg(test)]
#[path = "security_tests.rs"]
mod tests;
