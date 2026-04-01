use rust_ssh::connection_input::{is_direct_candidate, parse_direct_input};

#[test]
fn parse_user_host_port() {
    let p = parse_direct_input("u@example.com:2222").unwrap();
    assert_eq!(p.user.as_deref(), Some("u"));
    assert_eq!(p.host, "example.com");
    assert_eq!(p.port, Some(2222));
}

#[test]
fn parse_host_port_without_user() {
    let p = parse_direct_input("example.com:22").unwrap();
    assert_eq!(p.user, None);
    assert_eq!(p.host, "example.com");
    assert_eq!(p.port, Some(22));
}

#[test]
fn parse_ipv6_bracketed() {
    let p = parse_direct_input("u@[fe80::1]:2200").unwrap();
    assert_eq!(p.user.as_deref(), Some("u"));
    assert_eq!(p.host, "fe80::1");
    assert_eq!(p.port, Some(2200));
}

#[test]
fn ipv6_without_brackets_is_not_direct() {
    // Ambiguous: should not be treated as direct input.
    assert!(!is_direct_candidate("fe80::1"));
    assert!(parse_direct_input("fe80::1").is_some() == false);
}

