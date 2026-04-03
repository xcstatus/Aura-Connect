#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectInputParts {
    pub user: Option<String>,
    pub host: String,
    pub port: Option<u16>,
}

/// Try to parse direct-connect input.
///
/// Supported:
/// - `user@host`
/// - `user@host:port`
/// - `host:port`
/// - `host`
/// - `user@[ipv6]:port`
/// - `[ipv6]:port`
pub fn parse_direct_input(input: &str) -> Option<DirectInputParts> {
    let s = input.trim();
    if s.is_empty() {
        return None;
    }

    let (user_opt, rest) = if let Some((u, r)) = s.split_once('@') {
        let u = u.trim();
        let r = r.trim();
        let user = (!u.is_empty()).then(|| u.to_string());
        (user, r)
    } else {
        (None, s)
    };

    if rest.is_empty() {
        return None;
    }

    // Enforce bracketed IPv6 to avoid ambiguous parsing like `fe80::1`.
    if !rest.starts_with('[') && rest.matches(':').count() >= 2 {
        return None;
    }

    // IPv6 in brackets: [fe80::1]:2222
    if let Some(r) = rest.strip_prefix('[') {
        let (inside, after) = r.split_once(']')?;
        let host = inside.trim();
        if host.is_empty() {
            return None;
        }
        let after = after.trim();
        if after.is_empty() {
            return Some(DirectInputParts {
                user: user_opt,
                host: host.to_string(),
                port: None,
            });
        }
        let after = after.strip_prefix(':')?;
        let p = after.trim();
        let port = p.parse::<u16>().ok();
        return Some(DirectInputParts {
            user: user_opt,
            host: host.to_string(),
            port,
        });
    }

    // host[:port] (avoid mis-parsing IPv6 without brackets)
    let (host, port) = if let Some((h, p)) = rest.rsplit_once(':') {
        // If host part still contains ':' it's likely IPv6 without brackets — reject.
        if h.contains(':') {
            (rest, None)
        } else {
            let port = p.trim().parse::<u16>().ok();
            (h.trim(), port)
        }
    } else {
        (rest.trim(), None)
    };

    if host.is_empty() {
        return None;
    }

    Some(DirectInputParts {
        user: user_opt,
        host: host.to_string(),
        port,
    })
}

/// Heuristic: whether input should be treated as a direct-connect candidate.
pub fn is_direct_candidate(input: &str) -> bool {
    let s = input.trim();
    if s.is_empty() {
        return false;
    }
    if s.contains('@') {
        return true;
    }
    // Has explicit port or looks like a host (domain/ip)
    if s.contains(':') || s.contains('.') {
        return parse_direct_input(s).is_some();
    }
    false
}
