import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { type MockInstance, vi } from "vitest";

/** A command a window sent to the Rust side. */
export interface Call {
  cmd: string;
  args: unknown;
}

/** A reply to a command: a value, or a function of the command's arguments (it may throw). */
type Reply = unknown | ((args: unknown) => unknown);

/** What the window commands the pages send while they start up and fit themselves answer. */
const WINDOW_REPLIES: Record<string, Reply> = {
  "plugin:window|scale_factor": 1,
  "plugin:window|inner_size": { width: 640, height: 400 },
};

/**
 * Stands in for the Rust side of a window: every command is recorded in the returned list and
 * answered from `replies`; commands not in it answer nothing. Events sent with `emit` from
 * `@tauri-apps/api/event` reach the page's listeners, as if from the Rust side.
 */
export function mockTauri(replies: Record<string, Reply> = {}) {
  const calls: Call[] = [];
  mockWindows("main");
  mockIPC(
    (cmd, args) => {
      calls.push({ cmd, args });
      const reply = cmd in replies ? replies[cmd] : WINDOW_REPLIES[cmd];
      return typeof reply === "function" ? reply(args) : reply;
    },
    { shouldMockEvents: true },
  );
  return calls;
}

/** The arguments of each `cmd` sent so far. */
export const argsOf = (calls: Call[], cmd: string) =>
  calls.filter((call) => call.cmd === cmd).map((call) => call.args);

/**
 * Makes the pages (and `platform.ts`) loaded from now on run as on macOS ("darwin") or Linux
 * instead of Windows, until the test ends.
 */
export const onPlatform = (platform: "darwin" | "linux") => vi.stubGlobal("__PLATFORM__", platform);

/** A focused text field on the page with `value` in it and the caret at its end. */
export function textField(value = "") {
  const field = document.createElement("textarea");
  document.body.append(field);
  field.focus();
  field.value = value;
  field.setSelectionRange(value.length, value.length);
  return field;
}

/** Types `text` into `field` at the caret, as the keyboard would. */
export function type(field: HTMLTextAreaElement, text: string) {
  field.focus();
  document.execCommand("insertText", false, text);
}

/** A key press, not sent anywhere yet. */
export const keydown = (key: string, init: KeyboardEventInit = {}) =>
  new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...init });

/** Presses `key` on `target`; returns the event, to see whether it was handled. */
export function press(target: EventTarget, key: string, init: KeyboardEventInit = {}) {
  const event = keydown(key, init);
  target.dispatchEvent(event);
  return event;
}

/** Lets everything already started finish: the mocked commands answer right away. */
export const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

/** Where a page module listens on the document and the window, which outlive its page. */
let spies: [EventTarget, MockInstance<EventTarget["addEventListener"]>][] = [];

/**
 * Opens a window's page as the window would: the `<body>` of its `html` in the document, then
 * a fresh copy of its module from `load`.
 */
export async function loadPage(html: string, load: () => Promise<unknown>) {
  document.body.innerHTML = /<body>([\s\S]*)<\/body>/.exec(html)![1];
  vi.resetModules();
  spies = [document, window].map((target) => [target, vi.spyOn(target, "addEventListener")]);
  await load();
  await settle();
}

/** Closes the page opened by `loadPage`, with its listeners, so the next one is alone. */
export function closePage() {
  for (const [target, spy] of spies) {
    for (const [type, listener, options] of spy.mock.calls) {
      target.removeEventListener(type, listener, options);
    }
    spy.mockRestore();
  }
  spies = [];
  document.body.replaceChildren();
}
