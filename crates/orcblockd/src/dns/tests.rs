use std::collections::BTreeSet;

use orcblock_core::Domain;

use super::*;
use crate::dns::wire::tests::query;

fn blocking(domains: &[&str]) -> Blocked {
    let blocked = Blocked::default();
    blocked.replace(
        domains
            .iter()
            .map(|d| Domain::parse(d).expect("valid"))
            .collect::<BTreeSet<_>>(),
    );
    blocked
}

#[test]
fn decides_by_what_is_blocked() {
    let idle = Blocked::default();
    assert_eq!(decide("youtube.com", &idle), Answer::Refused);
    assert_eq!(
        decide("use-application-dns.net", &idle),
        Answer::Refused,
        "canaries only during a block"
    );

    let blocked = blocking(&["youtube.com"]);
    let cases = [
        ("youtube.com", Answer::Sinkhole),
        ("i.ytimg.youtube.com.", Answer::Sinkhole),
        ("YOUTUBE.COM", Answer::Sinkhole),
        ("vimeo.com", Answer::Refused),
        ("use-application-dns.net", Answer::NxDomain),
        ("mask.icloud.com.", Answer::NxDomain),
        ("mask-h2.icloud.com", Answer::NxDomain),
    ];
    for (name, want) in cases {
        assert_eq!(decide(name, &blocked), want, "{name}");
    }
}

#[test]
fn replies_end_to_end() {
    let blocked = blocking(&["youtube.com"]);
    let reply_to = |name, kind| reply(&query(3, name, kind), &blocked).expect("answers");
    assert!(reply_to("m.youtube.com", 1).ends_with(&[0, 4, 0, 0, 0, 0]));
    assert_eq!(reply_to("vimeo.com", 1)[3], Rcode::Refused as u8);
    assert_eq!(reply_to("mask.icloud.com", 1)[3], Rcode::NxDomain as u8);
    assert_eq!(reply(&[1, 2, 3], &blocked), None);
}

#[tokio::test]
async fn answers_over_udp_and_tcp() {
    let blocked = blocking(&["youtube.com"]);
    let sinkhole = Sinkhole::bind("127.0.0.1:0".parse().expect("addr"))
        .await
        .expect("binds");
    let udp_addr = sinkhole.local_addr().expect("addr");
    let tcp_addr = sinkhole.tcp.local_addr().expect("addr");
    let (stop, stopped) = watch::channel(false);
    let server = tokio::spawn(sinkhole.serve(blocked, stopped));

    let client = UdpSocket::bind("127.0.0.1:0").await.expect("binds");
    client
        .send_to(&query(5, "youtube.com", 1), udp_addr)
        .await
        .expect("sends");
    let mut buf = [0u8; 512];
    let len = client.recv(&mut buf).await.expect("receives");
    assert!(buf[..len].ends_with(&[0, 0, 0, 0]));

    let mut stream = TcpStream::connect(tcp_addr).await.expect("connects");
    let packet = query(6, "youtube.com", 28);
    stream
        .write_all(&u16::try_from(packet.len()).expect("short").to_be_bytes())
        .await
        .expect("writes");
    stream.write_all(&packet).await.expect("writes");
    let mut len = [0u8; 2];
    stream.read_exact(&mut len).await.expect("reads");
    let mut answer = vec![0u8; usize::from(u16::from_be_bytes(len))];
    stream.read_exact(&mut answer).await.expect("reads");
    assert_eq!(&answer[..2], &[0, 6]);
    assert!(answer.ends_with(&[0; 16]));

    stop.send(true).expect("running");
    server.await.expect("stops");
}

#[test]
fn dev_daemon_listens_apart_from_the_installed_helper() {
    assert_ne!(DEV_ADDR.port(), DEFAULT_ADDR.port());
}

#[tokio::test]
async fn serves_once_a_taken_port_is_free() {
    let squatter = UdpSocket::bind("127.0.0.1:0").await.expect("binds");
    let addr = squatter.local_addr().expect("addr");
    let (stop, stopped) = watch::channel(false);
    let server = tokio::spawn(Sinkhole::serve_on(
        addr,
        blocking(&["youtube.com"]),
        stopped,
        Duration::from_millis(20),
    ));
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!server.is_finished(), "a taken port is no reason to stop");
    drop(squatter);

    let client = UdpSocket::bind("127.0.0.1:0").await.expect("binds");
    let mut buf = [0u8; 512];
    let answered = timeout(Duration::from_secs(5), async {
        loop {
            client
                .send_to(&query(7, "youtube.com", 1), addr)
                .await
                .expect("sends");
            if let Ok(Ok(len)) = timeout(Duration::from_millis(50), client.recv(&mut buf)).await {
                return len;
            }
        }
    })
    .await
    .expect("answers once bound");
    assert!(buf[..answered].ends_with(&[0, 0, 0, 0]));

    stop.send(true).expect("running");
    server.await.expect("stops");
}

#[tokio::test]
async fn stops_while_waiting_for_a_taken_port() {
    let squatter = UdpSocket::bind("127.0.0.1:0").await.expect("binds");
    let (stop, stopped) = watch::channel(false);
    let server = tokio::spawn(Sinkhole::serve_on(
        squatter.local_addr().expect("addr"),
        Blocked::default(),
        stopped,
        Duration::from_secs(60),
    ));
    stop.send(true).expect("running");
    timeout(Duration::from_secs(5), server)
        .await
        .expect("stops promptly")
        .expect("no panic");
}
