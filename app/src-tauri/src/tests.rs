use super::*;

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

#[test]
fn client_errors_map_to_the_kinds_the_ui_switches_on() {
    let unreadable = serde_json::from_str::<Status>("{").expect_err("not json");
    let cases: [(&str, ClientError, &str); 5] = [
        (
            "no socket",
            ClientError::Unavailable(std::io::ErrorKind::NotFound.into()),
            "unavailable",
        ),
        (
            "older protocol",
            ClientError::Refused {
                kind: ErrorKind::Version,
                message: "protocol 1 expected".into(),
            },
            "helper_outdated",
        ),
        (
            "answer in another shape",
            ClientError::Codec(orcblock_ipc::codec::CodecError::Json(unreadable)),
            "helper_outdated",
        ),
        (
            "ordinary refusal",
            ClientError::Refused {
                kind: ErrorKind::Locked,
                message: "locked".into(),
            },
            "locked",
        ),
        ("hang up", ClientError::Closed, "internal"),
    ];
    for (name, err, kind) in cases {
        assert_eq!(AppError::from(err).kind, kind, "{name}");
    }
}
