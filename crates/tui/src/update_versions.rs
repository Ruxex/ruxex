/// Whether an official stable TUI release is newer than the connected app server.
pub(crate) fn is_official_server_older(client: &str, server: &str) -> bool {
    fn stable_version(version: &str) -> Option<(u64, u64, u64)> {
        fn component(value: Option<&str>) -> Option<u64> {
            let value = value?;
            if value.is_empty()
                || !value.bytes().all(|byte| byte.is_ascii_digit())
                || (value.len() > 1 && value.starts_with('0'))
            {
                return None;
            }
            value.parse().ok()
        }

        let mut parts = version.split('.');
        let version = (
            component(parts.next())?,
            component(parts.next())?,
            component(parts.next())?,
        );
        (parts.next().is_none() && version != (0, 0, 0)).then_some(version)
    }

    matches!((stable_version(client), stable_version(server)), (Some(client), Some(server)) if client > server)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_server_version_comparison() {
        assert!(is_official_server_older("0.152.1", "0.152.0"));
        assert!(is_official_server_older("0.153.0", "0.152.1"));
        assert!(!is_official_server_older("0.152.0", "0.152.0"));
        assert!(!is_official_server_older("0.152.0", "0.153.0"));
        for version in [
            "0.0.0",
            "0.0.0.0",
            "0.153.0-alpha.1",
            "unknown",
            "0.153",
            "0.153.0.1",
            " 0.153.0",
            "+0.153.0",
            "0.0153.0",
        ] {
            assert!(!is_official_server_older(version, "0.152.0"));
            assert!(!is_official_server_older("0.153.0", version));
        }
    }
}
