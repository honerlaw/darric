import { renderHook, act } from "@testing-library/react";
import { vi, describe, it, expect, beforeEach, afterEach } from "vitest";
import { useSearch } from "./useSearch";
import { mockCommands } from "../test/tauri-helpers";

const emptyResults = { sessions: [], notes: [], tasks: [] };

describe("useSearch", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    mockCommands({ search_all: () => emptyResults });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("starts with empty state", () => {
    const { result } = renderHook(() => useSearch());
    expect(result.current.query).toBe("");
    expect(result.current.results).toBeNull();
    expect(result.current.isSearching).toBe(false);
  });

  it("whitespace-only query stays idle without searching", () => {
    const { result } = renderHook(() => useSearch());
    act(() => {
      result.current.setQuery("   ");
    });
    expect(result.current.results).toBeNull();
    expect(result.current.isSearching).toBe(false);
  });

  it("fires search after 200ms debounce", async () => {
    const searchMock = vi.fn().mockReturnValue(emptyResults);
    mockCommands({ search_all: searchMock });
    const { result } = renderHook(() => useSearch());

    act(() => {
      result.current.setQuery("hello");
    });
    expect(searchMock).not.toHaveBeenCalled();

    await act(async () => {
      vi.advanceTimersByTime(200);
      await Promise.resolve();
    });
    expect(searchMock).toHaveBeenCalledOnce();
  });

  it("rapid updates cancel earlier debounce, firing only once", async () => {
    const searchMock = vi.fn().mockReturnValue(emptyResults);
    mockCommands({ search_all: searchMock });
    const { result } = renderHook(() => useSearch());

    act(() => {
      result.current.setQuery("a");
    });
    act(() => {
      result.current.setQuery("ab");
    });

    await act(async () => {
      vi.advanceTimersByTime(200);
      await Promise.resolve();
    });
    expect(searchMock).toHaveBeenCalledOnce();
  });

  it("clear() resets all state immediately", () => {
    const { result } = renderHook(() => useSearch());
    act(() => {
      result.current.setQuery("hello");
    });
    act(() => {
      result.current.clear();
    });
    expect(result.current.query).toBe("");
    expect(result.current.results).toBeNull();
    expect(result.current.isSearching).toBe(false);
  });
});
