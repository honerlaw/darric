import "@testing-library/jest-dom";
import { mockIPC, mockWindows, clearMocks } from "@tauri-apps/api/mocks";

beforeAll(() => {
  mockWindows("main");
});

beforeEach(() => {
  mockIPC((_cmd, _payload) => undefined, { shouldMockEvents: true });
});

afterEach(() => {
  clearMocks();
});
