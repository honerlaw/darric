import { describe, it, expect } from "vitest";
import { formatElapsed, formatTime, groupSessionsByDate } from "./utils";
import type { Session } from "../types";

function makeSession(started_at: string): Session {
  return {
    id: "test-id",
    topic: null,
    started_at,
    ended_at: null,
    created_at: started_at,
    recorded_minutes: 0,
  };
}

describe("formatElapsed", () => {
  it("formats zero seconds", () => {
    expect(formatElapsed(0)).toBe("00:00");
  });

  it("formats under a minute", () => {
    expect(formatElapsed(59)).toBe("00:59");
  });

  it("formats exactly one minute", () => {
    expect(formatElapsed(60)).toBe("01:00");
  });

  it("pads both components", () => {
    expect(formatElapsed(65)).toBe("01:05");
  });

  it("does not wrap hours", () => {
    expect(formatElapsed(3661)).toBe("61:01");
  });
});

describe("formatTime", () => {
  it("returns a time-like string for a valid ISO date", () => {
    const result = formatTime("2024-01-15T14:30:00.000Z");
    expect(result).toMatch(/\d{1,2}:\d{2}/);
  });
});

describe("groupSessionsByDate", () => {
  it("returns empty object for no sessions", () => {
    expect(groupSessionsByDate([])).toEqual({});
  });

  it("groups today's sessions under Today", () => {
    const now = new Date().toISOString();
    const result = groupSessionsByDate([makeSession(now)]);
    expect(result["Today"]).toHaveLength(1);
  });

  it("groups yesterday's sessions under Yesterday", () => {
    const yesterday = new Date(Date.now() - 86_400_000).toISOString();
    const result = groupSessionsByDate([makeSession(yesterday)]);
    expect(result["Yesterday"]).toHaveLength(1);
  });

  it("groups older sessions by date string, not Today or Yesterday", () => {
    const old = "2020-01-01T12:00:00.000Z";
    const result = groupSessionsByDate([makeSession(old)]);
    const keys = Object.keys(result);
    expect(keys).toHaveLength(1);
    expect(keys[0]).not.toBe("Today");
    expect(keys[0]).not.toBe("Yesterday");
  });

  it("groups multiple sessions from the same day together", () => {
    const now = new Date().toISOString();
    const result = groupSessionsByDate([makeSession(now), makeSession(now)]);
    expect(result["Today"]).toHaveLength(2);
  });

  it("splits sessions from different days into separate buckets", () => {
    const today = new Date().toISOString();
    const old = "2020-01-01T12:00:00.000Z";
    const result = groupSessionsByDate([makeSession(today), makeSession(old)]);
    expect(Object.keys(result)).toHaveLength(2);
    expect(result["Today"]).toHaveLength(1);
  });
});
