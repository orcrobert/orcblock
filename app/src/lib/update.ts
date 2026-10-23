/** A newer release the app can install. */
export interface Offer {
  version: string;
  /** `YYYY-MM-DD`, when the release manifest has one. */
  date: string | null;
}

/** What the app sends on `update_progress`. */
export type Progress = { type: "downloading"; percent: number | null } | { type: "installing" };

/** The in-app update as the top bar shows it. */
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

/** A progress event; only a running download moves on. */
export function progressed(state: UpdateState, progress: Progress): UpdateState {
  if (state.kind !== "downloading") return state;
  return progress.type === "installing"
    ? { kind: "installing", offer: state.offer }
    : { kind: "downloading", offer: state.offer, percent: progress.percent };
}

/** A failed install offers the same update again. */
export function failed(state: UpdateState): UpdateState {
  return state.kind === "none" ? state : { kind: "available", offer: state.offer };
}

/** The button text, or null when there is no button. */
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

/** What screen readers hear: the offer, then only the step, never each percent. */
export function updateSpeech(state: UpdateState): string {
  switch (state.kind) {
    case "none":
      return "";
    case "available": {
      const { version, date } = state.offer;
      return date ? `Update to ${version}, released ${date}` : `Update to ${version}`;
    }
    case "downloading":
      return `Downloading OrcBlock ${state.offer.version}`;
    case "installing":
      return `Installing OrcBlock ${state.offer.version}; it restarts when done`;
  }
}
