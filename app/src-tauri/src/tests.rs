use std::path::Path;

use super::*;

#[test]
fn install_script_quotes_the_path() {
    let script = install_script(Path::new(
        "/Applications/Orc \"Block\".app/Contents/MacOS/orcblockd",
    ));
    assert!(script.starts_with(
        r#"do shell script (quoted form of "/Applications/Orc \"Block\".app/Contents/MacOS/orcblockd") & " install""#
    ));
    assert!(script.ends_with("with administrator privileges"));
}

#[test]
fn errors_carry_the_daemon_kind() {
    let refused = AppError::from(ClientError::Refused {
        kind: ErrorKind::NoPotions,
        message: "none".into(),
    });
    assert_eq!(refused.kind, "no_potions");
    let unavailable = AppError::from(ClientError::Unavailable(std::io::Error::other("gone")));
    assert_eq!(unavailable.kind, "unavailable");
}
