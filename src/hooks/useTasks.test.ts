import { renderHook, act, waitFor } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import { useTasks } from "./useTasks";
import { mockCommands } from "../test/tauri-helpers";
import type { Task } from "../types";

function makeTask(overrides: Partial<Task> = {}): Task {
  return {
    id: "task-1",
    title: "Test task",
    col: "todo",
    position: 0,
    created_at: "2024-01-01T00:00:00Z",
    updated_at: "2024-01-01T00:00:00Z",
    tags: [],
    ...overrides,
  };
}

describe("useTasks", () => {
  it("loads tasks on mount", async () => {
    const task = makeTask();
    mockCommands({ list_tasks: () => [task] });

    const { result } = renderHook(() => useTasks());
    await waitFor(() => {
      expect(result.current.tasks).toHaveLength(1);
    });
    expect(result.current.tasks[0]?.id).toBe("task-1");
  });

  it("create() adds a task and triggers refresh", async () => {
    const created = makeTask({ id: "task-2", title: "New task" });
    let tasks: Task[] = [];
    mockCommands({
      list_tasks: () => tasks,
      create_task: () => {
        tasks = [created];
        return created;
      },
    });

    const { result } = renderHook(() => useTasks());
    await waitFor(() => {
      expect(result.current.tasks).toHaveLength(0);
    });

    await act(async () => {
      await result.current.create("New task", "todo");
    });
    await waitFor(() => {
      expect(result.current.tasks).toHaveLength(1);
    });
    expect(result.current.tasks[0]?.title).toBe("New task");
  });

  it("remove() deletes a task and triggers refresh", async () => {
    const task = makeTask();
    let tasks: Task[] = [task];
    mockCommands({
      list_tasks: () => tasks,
      delete_task: () => {
        tasks = [];
      },
    });

    const { result } = renderHook(() => useTasks());
    await waitFor(() => {
      expect(result.current.tasks).toHaveLength(1);
    });

    await act(async () => {
      await result.current.remove("task-1");
    });
    await waitFor(() => {
      expect(result.current.tasks).toHaveLength(0);
    });
  });

  it("rename() updates title and triggers refresh", async () => {
    const task = makeTask();
    const renamed = makeTask({ title: "Renamed task" });
    let tasks: Task[] = [task];
    mockCommands({
      list_tasks: () => tasks,
      update_task: () => {
        tasks = [renamed];
        return renamed;
      },
    });

    const { result } = renderHook(() => useTasks());
    await waitFor(() => {
      expect(result.current.tasks).toHaveLength(1);
    });

    await act(async () => {
      await result.current.rename("task-1", "Renamed task");
    });
    await waitFor(() => {
      expect(result.current.tasks[0]?.title).toBe("Renamed task");
    });
  });

  it("move() updates col and position then triggers refresh", async () => {
    const task = makeTask();
    const moved = makeTask({ col: "doing", position: 0 });
    let tasks: Task[] = [task];
    mockCommands({
      list_tasks: () => tasks,
      update_task: () => {
        tasks = [moved];
        return moved;
      },
    });

    const { result } = renderHook(() => useTasks());
    await waitFor(() => {
      expect(result.current.tasks).toHaveLength(1);
    });

    await act(async () => {
      await result.current.move("task-1", "doing", 0);
    });
    await waitFor(() => {
      expect(result.current.tasks[0]?.col).toBe("doing");
    });
  });
});
