# OrcBlock

A website blocker for macOS that you can't talk your way out of. Pick the sites, pick a duration and an orc stands guard until the time is up. Closing the app, quitting it or rebooting does not end the block. The only way out early is a sleep potion, and you get five a month.

## How it works

- **orcblockd** is a small helper that runs as a LaunchDaemon. It owns the blocklist, the timer and the potions, and it keeps going whether or not the app is open.
- During a block, macOS sends DNS lookups for the blocked sites to the helper's local resolver, which answers with nowhere (`/etc/resolver` files, no `/etc/hosts`). A pf anchor stops browsers from going around it through DNS over HTTPS or TLS.
- The timer counts on a monotonic clock, so changing the system clock does not shorten a block. Setting the clock back does not refill potions either.
- **OrcBlock.app** (Tauri + Svelte) and the **orcblock** CLI are clients. They talk to the helper over a local socket.

The details are in [docs/architecture.md](docs/architecture.md) and [spec 0001](docs/specs/0001-rust-rewrite-macos.md).

## Using it

OrcBlock is not packaged yet; a Homebrew cask is on the [roadmap](docs/roadmap.md). To build it from source you need macOS 13+, Rust (rustup picks the pinned toolchain), Node 25 and the Xcode command line tools:

```sh
git clone https://github.com/orcrobert/orcblock && cd orcblock
npm ci --prefix app
make bundle          # target/release/bundle/macos/OrcBlock.app
```

Open the app. On first launch it asks for your password once to install the helper. The CLI drives the same helper:

```sh
orcblock add https://www.youtube.com/watch?v=1   # stored as youtube.com
orcblock start 1h30m
orcblock status
orcblock potion                                  # ends the block, uses one of five
```

## Developing

`make daemon-dev` runs the helper without root, and it never touches your system: a dry-run enforcer logs what it would do. `make dev` runs the app against it. `make check` is the gate CI runs. See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

[AGPL-3.0](LICENSE).
