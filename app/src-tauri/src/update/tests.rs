use super::*;

#[test]
fn download_reports_whole_percent() {
    let cases = [
        ("no length", vec![10], None, None),
        ("zero length", vec![10], Some(0), None),
        ("first chunk", vec![1], Some(3), Some(33)),
        ("several chunks", vec![250, 250], Some(1000), Some(50)),
        ("complete", vec![500, 500], Some(1000), Some(100)),
        ("more than announced", vec![900, 900], Some(1000), Some(100)),
    ];
    for (name, chunks, total, expected) in cases {
        let mut download = Download::default();
        let last = chunks
            .into_iter()
            .map(|len| download.add(len, total))
            .last()
            .expect("every case has a chunk");
        assert_eq!(last.percent, expected, "{name}");
    }
}

#[test]
fn pending_is_empty_until_a_check_finds_an_update() {
    let pending = Pending::default();
    assert_eq!(pending.set(None), None);
    assert!(pending.take().is_none());
}
