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

beforeAll(() => {
  mockWindows("main");
  stubMatchMedia();
});

beforeEach(() => {
  mockIPC((_cmd, _payload) => undefined, { shouldMockEvents: true });
});

afterEach(() => {
  clearMocks();
});
