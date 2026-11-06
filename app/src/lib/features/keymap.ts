// Keyboard map of the main window (SPEC-0300-0001 §3). `resolveKey` only decides what a key
// press means; +page.svelte carries the action out. Keeping this pure lets the whole
// key table be tested without a DOM.

export type Focus = "side" | "list" | "detail";

export type KeyAction =
  | { type: "addBook" }
  | { type: "importCsv" }
  | { type: "openDb" }
  | { type: "focusSearch" }
  | { type: "leaveInput" }
  | { type: "searchToList" }
  | { type: "toggleFocus" }
  | { type: "sideMove"; delta: 1 | -1 }
  | { type: "sideToList" }
  | { type: "move"; delta: 1 | -1 }
  | { type: "page"; delta: 1 | -1 }
  | { type: "first" }
  | { type: "last" }
  | { type: "toSide" }
  | { type: "rate"; value: number }
  | { type: "editNotes" }
  | { type: "openLink" }
  | { type: "lookup" }
  | { type: "cycleSort" }
  | { type: "reverseSort" }
  | { type: "deleteBook" };

export interface KeyInput {
  key: string;
  ctrl: boolean;
  composing: boolean;
  /** A dialog is open: the main window ignores keys. */
  dialogOpen: boolean;
  /** Focus is in an input / textarea / select / contenteditable. */
  typing: boolean;
  /** Focus is in the search box. */
  inSearch: boolean;
  focus: Focus;
}

export interface KeyResult {
  action: KeyAction;
  /** Whether the browser default (scrolling, typing the character, …) is suppressed. */
  preventDefault: boolean;
}

const CTRL_KEYS: Record<string, KeyAction> = {
  n: { type: "addBook" },
  i: { type: "importCsv" },
  o: { type: "openDb" },
  f: { type: "focusSearch" },
};

const SIDE_KEYS: Record<string, KeyAction> = {
  j: { type: "sideMove", delta: 1 },
  ArrowDown: { type: "sideMove", delta: 1 },
  k: { type: "sideMove", delta: -1 },
  ArrowUp: { type: "sideMove", delta: -1 },
  Enter: { type: "sideToList" },
  l: { type: "sideToList" },
  ArrowRight: { type: "sideToList" },
};

/** List keys; the boolean is whether the browser default is suppressed. */
const LIST_KEYS: Record<string, [KeyAction, boolean]> = {
  j: [{ type: "move", delta: 1 }, true],
  ArrowDown: [{ type: "move", delta: 1 }, true],
  k: [{ type: "move", delta: -1 }, true],
  ArrowUp: [{ type: "move", delta: -1 }, true],
  PageDown: [{ type: "page", delta: 1 }, true],
  PageUp: [{ type: "page", delta: -1 }, true],
  g: [{ type: "first" }, true],
  Home: [{ type: "first" }, true],
  G: [{ type: "last" }, true],
  End: [{ type: "last" }, true],
  h: [{ type: "toSide" }, true],
  ArrowLeft: [{ type: "toSide" }, true],
  e: [{ type: "editNotes" }, true],
  o: [{ type: "openLink" }, false],
  Enter: [{ type: "openLink" }, false],
  u: [{ type: "lookup" }, false],
  s: [{ type: "cycleSort" }, false],
  r: [{ type: "reverseSort" }, false],
  Delete: [{ type: "deleteBook" }, false],
};

const prevent = (action: KeyAction): KeyResult => ({ action, preventDefault: true });

export function resolveKey(k: KeyInput): KeyResult | null {
  if (k.dialogOpen || k.composing) return null;

  if (k.ctrl) {
    const a = CTRL_KEYS[k.key.toLowerCase()];
    if (a) return prevent(a);
  }

  if (k.typing) {
    if (k.key === "Escape") return { action: { type: "leaveInput" }, preventDefault: false };
    if (k.inSearch && (k.key === "Enter" || k.key === "ArrowDown")) return prevent({ type: "searchToList" });
    return null;
  }
  if (k.ctrl) return null;

  if (k.key === "Tab") return prevent({ type: "toggleFocus" });
  if (k.key === "/") return prevent({ type: "focusSearch" });

  if (k.focus === "side") {
    const a = SIDE_KEYS[k.key];
    return a ? prevent(a) : null;
  }

  if (/^[0-5]$/.test(k.key)) return { action: { type: "rate", value: Number(k.key) }, preventDefault: false };
  const hit = LIST_KEYS[k.key];
  return hit ? { action: hit[0], preventDefault: hit[1] } : null;
}

export function isTypingTarget(t: EventTarget | null): boolean {
  const el = t as HTMLElement | null;
  return !!el && (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.tagName === "SELECT" || el.isContentEditable);
}
