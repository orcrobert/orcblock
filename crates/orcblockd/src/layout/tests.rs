use super::*;

#[test]
fn mac_paths() {
    let layout = Layout::under("/");
    let cases = [
        (
            layout.helper(),
            "/Library/PrivilegedHelperTools/com.orcrobert.orcblockd",
        ),
        (
            layout.plist(),
            "/Library/LaunchDaemons/com.orcrobert.orcblockd.plist",
        ),
        (
            layout.state_file(),
            "/Library/Application Support/OrcBlock/state.json",
        ),
        (layout.socket(), "/var/run/orcblockd.sock"),
        (layout.resolver_dir(), "/etc/resolver"),
        (layout.log_file(), "/var/log/orcblockd.log"),
    ];
    for (path, want) in cases {
        assert_eq!(path, Path::new(want));
    }
    assert_eq!(layout.platform(), Platform::Mac);
}

#[test]
fn windows_paths() {
    let root = Path::new("root");
    let layout = Layout::windows_under(root);
    let cases = [
        (
            layout.helper(),
            root.join("Program Files")
                .join("OrcBlock Helper")
                .join("orcblockd.exe"),
        ),
        (layout.data_dir(), root.join("ProgramData").join("OrcBlock")),
        (
            layout.state_file(),
            root.join("ProgramData").join("OrcBlock").join("state.json"),
        ),
        (
            layout.plist(),
            root.join("ProgramData")
                .join("OrcBlock")
                .join("orcblockd-task.xml"),
        ),
        (
            layout.log_file(),
            root.join("ProgramData")
                .join("OrcBlock")
                .join("orcblockd.log"),
        ),
    ];
    for (path, want) in cases {
        assert_eq!(path, want);
    }
    assert_eq!(layout.platform(), Platform::Windows);
}

#[test]
fn system_matches_this_platform() {
    let want = if cfg!(windows) {
        Platform::Windows
    } else {
        Platform::Mac
    };
    assert_eq!(Layout::system().platform(), want);
}
