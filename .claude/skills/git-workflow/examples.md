# Git workflow examples

## A feature as a commit sequence

```text
docs(specs): add 0004 extend a running block
refactor(core): extract remaining-time arithmetic from Session
feat(core): allow extending an active session
feat(ipc): add the Extend request
feat(daemon): handle Extend and persist the new remaining time
feat(ui): add a "+30 min" action to the blocking screen
```

The refactor comes first and changes no behaviour. The core rule lands before anything that calls it. Each commit passes `make test`.

## A good message

```text
fix(enforce): restore resolver files removed while a block runs

Deleting /etc/resolver/<domain> by hand ended the block until the next
start. The periodic check now compares the files on disk with the
active session and rewrites any that are missing or changed.

Refs: #18
```

## Headers: bad and fixed

| Bad | Fixed | Why |
| --- | --- | --- |
| `update stuff` | `fix(dns): answer AAAA queries with ::` | Needs a type and a specific summary |
| `feat(ui): Added dark mode.` | `feat(ui): add dark theme` | Imperative, lowercase, no period |
| `fix: fix tests and add potions` | two commits: `test(core): ...` and `feat(core): ...` | "and" means two changes |
| `feat(core): rename field and use it` | `refactor(core): rename ...` then `feat(...)` | Refactors go first, on their own |
| `chore: bump` | `build(deps): bump tokio to 1.50` | Say what changed |

## Breaking change

```text
feat(ipc)!: send remaining time in seconds instead of minutes

BREAKING CHANGE: PROTOCOL_VERSION is now 2; apps older than 0.3 ask to
update the helper.
```

## Splitting a messy working tree

```sh
git status -s
git add crates/orcblock-core/src/domain.rs crates/orcblock-core/src/domain/tests.rs
git diff --staged
git commit -m "fix(core): keep the trailing label of single-label hosts"
git add app/src/lib/SiteRow.svelte
git diff --staged
git commit -m "fix(ui): truncate long domains with an ellipsis"
```

## Fixing an unpushed commit

```sh
git add crates/orcblock-core/src/potion.rs
git commit --fixup=abc1234
GIT_SEQUENCE_EDITOR=: git rebase -i --autosquash main
```

Only before the first push. After that, add a normal commit.
