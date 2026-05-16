import { mockIPC } from "@tauri-apps/api/mocks";
import type { InvokeArgs } from "@tauri-apps/api/core";

export type CommandMockMap = Record<string, (payload?: InvokeArgs) => unknown>;

export function mockCommands(commands: CommandMockMap): void {
  mockIPC((cmd, payload) => commands[cmd]?.(payload), { shouldMockEvents: true });
}
