## Install on a Mac

1. Download **OrcBlock.dmg**, open it and drag OrcBlock into Applications.
2. Open OrcBlock. macOS says it cannot check the app for malicious software, because OrcBlock is not signed with an Apple developer certificate. Choose **Done**, then open **System Settings → Privacy & Security**, scroll down and click **Open Anyway** next to OrcBlock. You only do this once.
3. Click **Install helper** and enter your password. The helper keeps blocks running when the app is closed.

## Install on Windows

1. Download **OrcBlock-setup.exe** and run it. Windows SmartScreen may say it protected your PC, because OrcBlock is not signed with a code signing certificate. Choose **More info**, then **Run anyway**.
2. Allow the installer to make changes (administrator rights). It installs the app and the helper that keeps blocks running when the app is closed.

To check the download, compare it with `SHA256SUMS`, or verify where it was built with `gh attestation verify OrcBlock.dmg -R orcrobert/orcblock` (or `OrcBlock-setup.exe`).
