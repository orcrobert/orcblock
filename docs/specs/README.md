# Specs

Every feature starts as a spec here, written by `/feature` and implemented by `/build`.

- File name: `NNNN-kebab-slug.md`, the next free number. `0000-template.md` is the template.
- Status moves `draft` → `approved` → `done`. Only the maintainer sets `approved`.
- Acceptance criteria are testable checkboxes. `/build` ticks them in the commit that completes each slice.
- Slices are shippable on their own and under ~300 changed lines each.
- Unknowns go under Open questions, never guessed.
- A spec that proves wrong during `/build` is changed through a proposal, not silently.
