import { useCallback, useEffect, useState } from "react";
import { createTask, deleteTask, listTasks, updateTask } from "../lib/tauri";
import type { Task, TaskColumn } from "../types";

interface UseTasksReturn {
  tasks: Task[];
  create: (title: string, col: TaskColumn) => Promise<Task>;
  move: (id: string, col: TaskColumn, position: number) => Promise<void>;
  rename: (id: string, title: string) => Promise<void>;
  remove: (id: string) => Promise<void>;
  refresh: () => Promise<void>;
}

export function useTasks(): UseTasksReturn {
  const [tasks, setTasks] = useState<Task[]>([]);

  const refresh = useCallback(async (): Promise<void> => {
    const list = await listTasks();
    setTasks(list);
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const create = useCallback(
    async (title: string, col: TaskColumn): Promise<Task> => {
      const task = await createTask(title, col);
      await refresh();
      return task;
    },
    [refresh],
  );

  const move = useCallback(
    async (id: string, col: TaskColumn, position: number): Promise<void> => {
      await updateTask(id, { col, position });
      await refresh();
    },
    [refresh],
  );

  const rename = useCallback(
    async (id: string, title: string): Promise<void> => {
      await updateTask(id, { title });
      await refresh();
    },
    [refresh],
  );

  const remove = useCallback(
    async (id: string): Promise<void> => {
      await deleteTask(id);
      await refresh();
    },
    [refresh],
  );

  return { tasks, create, move, rename, remove, refresh };
}
