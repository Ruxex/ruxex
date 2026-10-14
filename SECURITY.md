# Security Policy

Ruxex is an open-source Rust library and agent runtime foundation. Security
reports are especially important for the sandbox, approval, filesystem,
network, credential, provider, and AppServer boundaries.

## Reporting a vulnerability

Please do not open a public issue for an unpatched vulnerability. Use
[GitHub's private vulnerability reporting for Ruxex](https://github.com/Ruxex/ruxex/security/advisories/new)
and include:

- the affected revision, crate, or released version;
- clear reproduction steps or a minimal proof of concept;
- the security impact and likely attack prerequisites; and
- any suggested mitigation, if available.

Remove credentials, tokens, personal data, and private endpoint details from
the report. The maintainers will coordinate the response and public disclosure
after a fix or mitigation is available.

## Scope and support

The repository currently verifies Linux and macOS. Ruxex does not distribute
V8 binaries; native artifacts are resolved as part of the consumer's build.
Reports about the Rust crates, the provider host boundary, the AppServer, the
V8 sandbox integration, and the security controls are in scope.

There is no guaranteed response or remediation time. If private reporting is
unavailable for the repository, contact the maintainers through the Ruxex
organization before disclosing the issue publicly.

## Safe handling

Do not test vulnerabilities against systems, accounts, endpoints, or data that
you do not own or have explicit permission to use. Follow the repository's
license and the applicable third-party license and attribution requirements.
