import "@testing-library/jest-dom";
import { mockIPC, mockWindows, clearMocks } from "@tauri-apps/api/mocks";

// jsdom ships no matchMedia, and App reads it on mount to follow the OS colour
// scheme — without this every test that renders App throws before it gets to
// what it is actually asserting.
function stubMatchMedia(): void {
  window.matchMedia = (query: string): MediaQueryList => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => undefined,
    removeListener: () => undefined,
    addEventListener: () => undefined,
    removeEventListener: () => undefined,
    dispatchEvent: () => false,
  });
}

// jsdom implements no scrolling, and RecorderPane scrolls the transcript to the
// newest line on every append — so any test rendering a non-empty transcript
// throws before reaching its assertions.
function stubScrollIntoView(): void {
  Element.prototype.scrollIntoView = (): void => undefined;
}

beforeAll(() => {
  mockWindows("main");
  stubMatchMedia();
  stubScrollIntoView();
});

beforeEach(() => {
  mockIPC((_cmd, _payload) => undefined, { shouldMockEvents: true });
});

afterEach(() => {
  clearMocks();
});
