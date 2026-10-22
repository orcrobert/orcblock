# 0005: Named lists of sites and apps

Status: approved (maintainer, 2026-10-22)
Issue: none (maintainer request, 2026-10-22)

## Problem
OrcBlock has one list. Everything on it is blocked every time. Someone who wants "social media" blocked while writing but "social media + email + Slack" during deep work has to add and remove targets before each block, and removal is the slow, error-prone part. New users also start from an empty list and must think of every distraction themselves.

## Behaviour
1. **Lists.** Sites and apps live in named lists ("Social", "Deep work"). A list holds sites and apps, the same rules as today's single list. There is always at least one list.
2. **Upgrade.** Whatever was on the single list moves into a list named "My list". A block running during the upgrade keeps running untouched.
3. **Managing lists.** The TARGETS panel shows one list at a time. Its header is a list switcher: the current list's name, its count, and a menu with New list, Rename and Delete. New list asks for a name and opens the empty list. Delete asks for confirmation ("Delete Social and its 7 targets?").
4. **Choosing what to block.** Each list has a checkbox in the switcher ("in next block"). Start blocks the union of the checked lists. The left panel shows what will be blocked: "SOCIAL + DEEP WORK // 12 TARGETS". The app remembers the checked lists between launches. Start is disabled while the checked lists are empty, with "CHECK A LIST WITH TARGETS".
5. **During a block.** Lists in the block show a lock. Their sites and apps cannot be removed, the list cannot be deleted, and anything added to them is blocked at once (as today). Lists not in the block can be edited, renamed and deleted freely; adding to them does not change the running block. Any list can be renamed.
6. **Duplicates.** The same site or app may sit in several lists. Within one list the existing rule holds: `m.youtube.com` is refused when `youtube.com` is there.
7. **Limits.** At most 32 lists. A name is 1 to 40 characters after trimming, unique ignoring case. Errors are shown verbatim: "There is already a list named Social".
8. **CLI.** `orcblock status` shows every list, `orcblock list create|rename|delete`, and `--list <name>` on `add`, `remove`, `add-app`, `remove-app` and `start` (default: the only list, or an error naming the lists when there are several; `start` defaults to all lists, as today).
9. **Helper too old.** The app already shows "update the helper" on a version mismatch; nothing new.

### UI sketch (right panel)
```
┌──────────────────────────────────────────────┐
│ [x] SOCIAL ▾                       07   [≡]  │  switcher; ≡ = New / Rename / Delete
├──────────────────────────────────────────────┤
│ > YOUTUBE.COM                  ADD +  + APP  │
│ PASTE A LINK, TYPE A NAME OR DROP AN APP     │
├──────────────────────────────────────────────┤
│ 01  youtube.com                          ×   │
│ ...                                          │
```
The ▾ menu lists every list with its checkbox, count and lock. Final layout goes through the ui-reviewer.

## Acceptance criteria
- [x] `ListName` accepts 1 to 40 characters after trimming, no control characters, and compares ignoring case. Table tests and proptests.
- [x] A `state.json` from 0.2.0 (single blocklist and app list, with and without a running session) loads into one list named "My list", and a running session keeps its sites, apps and remaining time.
- [x] Creating a 33rd list, a duplicate name or an empty name is refused without changing state.
- [x] `start_block` blocks the union of the given lists; it refuses unknown list IDs and an empty union with `EmptyBlocklist`.
- [x] During a block, removing a site or app from a list in the block, or deleting such a list, is refused with `Locked`; the same actions on a list outside the block succeed.
- [x] Adding to a list in the block enforces the new target at once; adding to a list outside the block leaves `blocked_domains()` and `blocked_apps()` unchanged.
- [x] Removing a site from list B never unblocks it while list A, which is in the block, still lists it (the session's frozen set is the authority).
- [x] Deleting the last list is refused.
- [x] `PROTOCOL_VERSION` is 3; a version-2 client gets `Version`, not a misread status.
- [x] The app switches lists, creates, renames and deletes them, and starts a block over the checked lists. Checked lists survive an app restart. vitest covers the selection logic.

## Design

### orcblock-core
- **New module `list.rs`:**
  - `ListId(u32)`, assigned by `State` from a counter that never reuses an ID, so a stale client cannot hit a recreated list.
  - `ListName` newtype with `ListNameError`.
  - `TargetList { id, name, sites: Blocklist, apps: AppList }`. `Blocklist` and `AppList` keep their invariants unchanged.
- **`State`:**
  - Replaces `blocklist` and `apps` with `lists: Vec<TargetList>` and `next_list_id`.
  - Loads the old shape through a private `StateRepr` enum (`#[serde(untagged)]` or an explicit version field), converting it into one list. This must never fail on a file 0.2.0 wrote (see the open wiki issue about unreadable state files ending blocks).
  - Commands: `create_list(name)`, `rename_list(id, name)`, `delete_list(id)`, and `add_site`, `remove_site`, `add_app`, `remove_app` gain a `ListId`. `start_block(lists: &[ListId], duration, now)`.
  - New `CommandError` variants: `ListNotFound`, `ListNameTaken(ListName)`, `TooManyLists`, `LastList`, `ListLocked(ListName)`.
- **`Session`** gains `lists: BTreeSet<ListId>` (`#[serde(default)]`; a migrated session gets the "My list" ID). Its `domains` and `apps` stay the frozen union and remain the only thing enforcement reads. The union is normalised so a parent domain from one list drops a covered subdomain from another.

### orcblock-ipc
- **Requests:** `create_list {name}`, `rename_list {list, name}`, `delete_list {list}`. `add_site`, `remove_site`, `add_app`, `remove_app` gain `list`. `start_block` gains `lists`.
- **`Status`:** `lists: Vec<List {id, name, sites, apps}>` replaces `sites` and `apps`. `Block.lists` names the lists in the block.
- **Error kinds:** `InvalidList` (bad name), `ListNotFound`, `TooManyLists`. `ListNameTaken` maps to `AlreadyListed`, `ListLocked` and `LastList` to `Locked`.
- **`PROTOCOL_VERSION` = 3.** Golden JSON strings for each new request.

### orcblockd
Match arms in `service.rs` per the add-ipc-command skill. Enforcement is unchanged: it still reads `blocked_domains()` and `blocked_apps()`. No new files; lists live in `state.json`.

### orcblock-cli
The subcommands in Behaviour 8. `render.rs` groups targets by list and marks lists in the running block.

### app/src-tauri
Tauri commands for the three new requests; the four target commands and `start_block` pass the list through. No other change.

### app/src
- `lib/lists.ts`: the current list, the checked set (in `localStorage`, pruned of IDs the daemon no longer reports, defaulting to all lists) and the union count. Pure, vitest-tested.
- `components/ListSwitcher.svelte`: the header and menu.
- `Home.svelte`: the switcher above the prompt; rows come from the current list; the left panel names the checked lists.
- New, Rename and Delete reuse `Dialog.svelte`. theme.css tokens only, light and dark.

### Risks
- **Migration** is the riskiest part: a bad conversion of `state.json` would either lose the list or, worse, end a running block. Fixture tests from real 0.2.0 files cover it.
- **Home layout** is already tight at 900 × 520; the switcher must fit in the existing 28 px header row.

## Bypass and safety
- **No new way out.** The session's frozen `domains` and `apps` stay the only input to enforcement. Lists only decide what goes into a new session and where an addition during a block lands.
- **Deleting or emptying a list during a block** is refused for lists in the block. For lists outside it, the session is untouched even when it holds the same site (criterion on list B above).
- **Renaming** never changes what is blocked.
- **Migration** must keep a running session byte-for-byte in its domains, apps and remaining time; a failed parse would hit the existing "unreadable state file" issue, so the old shape gets dedicated fixtures.
- **Untrusted socket input:** list IDs are plain integers looked up in `State`; names are validated `ListName`s and never touch the filesystem or a command.
- **Resource use:** at most 32 lists, each with today's per-list limits, bounds the state file and the status message (already capped at `MAX_MESSAGE_LEN`).
- Needs a security-reviewer pass because it changes `start_block`, removal rules and the state file format.

## Slices
**PR 1: lists**
1. **Core types:** `ListId`, `ListName` (table tests, proptests), `TargetList`.
2. **Core state:** lists in `State`, the list commands, list-aware add/remove/start, lock rules, `Session.lists`, migration with 0.2.0 fixtures.
3. **Wire and daemon:** requests, `Status.lists`, error kinds, `PROTOCOL_VERSION = 3`, service arms and integration tests.
4. **CLI:** subcommands, `--list`, grouped rendering.
5. **App:** Tauri commands, `lists.ts` with vitest, `ListSwitcher.svelte`, Home wiring, dialogs.
6. **Wrap-up:** architecture.md, README, website line if the copy mentions "your list", spec ticked.

**PR 2: a starter list** (separate branch, after PR 1 merges)
7. **Preset data:** `app/src/lib/presets.ts` with one preset, its source and the date it was checked (see "Starter list" below). vitest checks every domain parses the way the daemon will clean it.
8. **Creating from a preset:** `create_list` gains optional `sites` and `apps`, validated all-or-nothing so a preset never lands half-added. New list offers "Start from: Social media & video". The empty state on first run offers the same in one click.
9. **Credit:** the preset shows its source under the list name ("BASED ON PEW RESEARCH CENTER, SOCIAL MEDIA FACT SHEET, 2026"), and the README gets a Credits section.

### Starter list
One preset, not several. The research found no peer-reviewed ranking of "distracting websites"; the only well-sourced, regularly updated data on which platforms people actually spend time on is the Pew Research Center's social media fact sheet (survey of U.S. adults, Jan–Jun 2026, published 2026-10-19). Vendor blogs (Freedom, DeskTime, RescueTime) and surveys reported second-hand were not used. Community blocklists such as StevenBlack/hosts' social extension (MIT, ~3,800 domains) are built for ad and tracker blocking, list CDN and API hosts that would break embeds across the web, and are far longer than a list anyone would read; also not used.

**Social media & video**, the platforms Pew measures, as their main domains (subdomains are covered automatically):
youtube.com, facebook.com, instagram.com, tiktok.com, x.com, twitter.com, reddit.com, pinterest.com, snapchat.com, threads.com, threads.net, bsky.app

twitter.com and threads.net are the old names. They still redirect to the new ones, and they're listed so that old links and bookmarks are blocked directly instead of depending on the redirect.

Left out on purpose: LinkedIn and WhatsApp (work and messaging for many people), Nextdoor, Truth Social and BeReal (niche, or phone-only). No apps: none of these platforms ships a common Mac app besides WhatsApp, which is left out.

## Test plan
- **Core:** table tests for every list command and error, proptests for `ListName`, the list-B criterion, union normalisation, and 0.2.0 `state.json` fixtures in `crates/orcblock-core/testdata/` (idle and mid-block).
- **ipc:** round-trip and golden JSON per request; version-2 request refused.
- **Daemon:** service tests with the dry-run enforcer: start over two lists, add during a block to a list in and out of it, delete refused.
- **App:** vitest for `lists.ts` and the preset domains; `make site-check` after touching app/src/lib.
- **Manual (QA agent, dev daemon only, 1-minute blocks):** upgrade a dev `state.json` from 0.2.0 mid-block; create, rename, delete; block two of three lists and check only their sites resolve to 0.0.0.0 in the dry-run log; light and dark. Teardown as in spec 0004.

## Out of scope
- Schedules per list (roadmap: Schedules).
- Sharing, importing or exporting lists.
- Syncing presets from the internet; presets ship with the app and are updated by hand in a release.
- More presets (news, shopping, streaming). Easy to add later if a source as solid as Pew turns up.

## Open questions
None. Resolved with the maintainer (2026-10-22):
1. The starter list is offered (one click in the empty state and in New list), never created automatically.
2. The migrated list is named "My list".
3. The checked lists live in the app (`localStorage`); the CLI's `start` takes `--list` and defaults to all lists.
