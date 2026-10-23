// The in-app update as the top bar shows it.

/** A newer release the app can install. */
export interface Offer {
  version: string;
  /** `YYYY-MM-DD`, when the release manifest has one. */
  date: string | null;
}

export type UpdateState =
  | { kind: "none" }
  | { kind: "available"; offer: Offer }
  | { kind: "downloading"; offer: Offer; percent: number | null }
  | { kind: "installing"; offer: Offer };

export function offered(offer: Offer | null): UpdateState {
  return offer ? { kind: "available", offer } : { kind: "none" };
}

/** The click: only an available update starts downloading. */
export function started(state: UpdateState): UpdateState {
  return state.kind === "available"
    ? { kind: "downloading", offer: state.offer, percent: null }
    : state;
}

/** A progress event; the last byte means the bundle is being swapped. */
export function progressed(state: UpdateState, percent: number | null): UpdateState {
  if (state.kind !== "downloading") return state;
  return percent === 100
    ? { kind: "installing", offer: state.offer }
    : { kind: "downloading", offer: state.offer, percent };
}

/** A failed install offers the same update again. */
export function failed(state: UpdateState): UpdateState {
  return state.kind === "none" ? state : { kind: "available", offer: state.offer };
}

export function updateLabel(state: UpdateState): string | null {
  switch (state.kind) {
    case "none":
      return null;
    case "available":
      return `UPDATE → ${state.offer.version}`;
    case "downloading":
      return state.percent === null ? "DOWNLOADING" : `DOWNLOADING ${state.percent}%`;
    case "installing":
      return "INSTALLING";
  }
}
