import {
  DndContext,
  DragOverlay,
  PointerSensor,
  useDroppable,
  useSensor,
  useSensors,
  type DragEndEvent,
  type DragStartEvent,
} from "@dnd-kit/core";
import { SortableContext, useSortable, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import type React from "react";
import { useEffect, useRef, useState } from "react";
import { TagInput } from "../components/tags/TagInput";
import { getSetting, saveSetting } from "../lib/tauri";
import type { Tag, Task, TaskColumn } from "../types";

const COLUMNS: { id: TaskColumn; label: string }[] = [
  { id: "todo", label: "Todo" },
  { id: "doing", label: "Doing" },
  { id: "done", label: "Done" },
];

/* ── Task card ─────────────────────────────────────────────────────── */
interface TaskCardProps {
  task: Task;
  isDragging?: boolean | undefined;
  allTags: Tag[];
  onToggleDone: () => void;
  onRename: (title: string) => void;
  onDelete: () => void;
  onAddTag: (name: string) => void;
  onRemoveTag: (tagId: string) => void;
}

function TaskCard({
  task,
  isDragging,
  allTags,
  onToggleDone,
  onRename,
  onDelete,
  onAddTag,
  onRemoveTag,
}: TaskCardProps): React.JSX.Element {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(task.title);
  const [confirmingDelete, setConfirmingDelete] = useState(false);
  const cancelEditRef = useRef(false);

  useEffect(() => {
    if (!editing) setDraft(task.title);
  }, [task.title, editing]);
  const done = task.col === "done";

  const { attributes, listeners, setNodeRef, transform, transition } = useSortable({
    id: task.id,
  });

  const style: React.CSSProperties = {
    transform: CSS.Transform.toString(transform),
    transition,
    rotate: isDragging === true ? "-1.5deg" : undefined,
    boxShadow:
      isDragging === true
        ? "0 4px 8px rgba(10,10,10,0.05), 0 14px 32px rgba(10,10,10,0.08)"
        : "0 1px 0 rgba(10,10,10,0.04), 0 1px 2px rgba(10,10,10,0.04)",
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      className={`group flex items-start gap-[10px] rounded-[10px] bg-paper p-3 transition-shadow ${
        editing
          ? "ring-2 ring-accent shadow-none"
          : "hover:[box-shadow:0_2px_4px_rgba(10,10,10,0.04),0_6px_16px_rgba(10,10,10,0.06)]"
      }`}
    >
      {/* Grip */}
      <div
        {...attributes}
        {...listeners}
        className="mt-[2px] cursor-grab active:cursor-grabbing shrink-0 opacity-35 hover:opacity-100"
      >
        <svg width="10" height="14" viewBox="0 0 10 14" fill="currentColor" className="text-ink-3">
          <circle cx="2" cy="2" r="1.5" />
          <circle cx="8" cy="2" r="1.5" />
          <circle cx="2" cy="7" r="1.5" />
          <circle cx="8" cy="7" r="1.5" />
          <circle cx="2" cy="12" r="1.5" />
          <circle cx="8" cy="12" r="1.5" />
        </svg>
      </div>

      {/* Checkbox */}
      <button
        type="button"
        onClick={onToggleDone}
        className={`mt-[1px] flex h-[15px] w-[15px] shrink-0 cursor-pointer items-center justify-center rounded-[4px] border transition-colors ${
          done ? "border-accent bg-accent" : "border-ink-3 bg-paper hover:border-accent"
        }`}
      >
        {done && (
          <svg width="9" height="7" viewBox="0 0 9 7" fill="none" stroke="white" strokeWidth="1.5">
            <polyline points="1,3.5 3.5,6 8,1" />
          </svg>
        )}
      </button>

      {/* Title + tags */}
      {editing ? (
        <div className="flex flex-1 flex-col gap-[6px]">
          <textarea
            autoFocus
            rows={1}
            value={draft}
            onChange={(e) => {
              setDraft(e.target.value);
              e.target.style.height = "auto";
              e.target.style.height = `${String(e.target.scrollHeight)}px`;
            }}
            onFocus={(e) => {
              e.target.style.height = "auto";
              e.target.style.height = `${String(e.target.scrollHeight)}px`;
              e.target.setSelectionRange(e.target.value.length, e.target.value.length);
            }}
            onBlur={() => {
              if (!cancelEditRef.current) onRename(draft);
              cancelEditRef.current = false;
              setEditing(false);
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); cancelEditRef.current = true; onRename(draft); setEditing(false); }
              if (e.key === "Escape") { cancelEditRef.current = true; setDraft(task.title); setEditing(false); }
            }}
            className="w-full resize-none overflow-hidden border-b border-accent bg-transparent pb-[2px] text-[14px] leading-[1.45] text-ink outline-none"
          />
          <span className="text-[10px] text-ink-4">Shift+Enter for new line · Enter to save · Esc to cancel</span>
          <TagInput tags={task.tags} allTags={allTags} onAdd={onAddTag} onRemove={onRemoveTag} />
        </div>
      ) : (
        <div className="flex flex-1 flex-col gap-[4px]">
          <span
            className={`cursor-text whitespace-pre-wrap text-[14px] leading-[1.45] ${
              done ? "text-ink-4 line-through" : "text-ink"
            }`}
            onDoubleClick={() => { setEditing(true); }}
          >
            {task.title}
          </span>
          {task.tags.length > 0 && (
            <div className="flex flex-wrap gap-[3px]">
              {task.tags.map((tag) => (
                <span
                  key={tag.id}
                  className="rounded-full bg-accent/10 px-[6px] py-[1px] font-mono text-[9px] tracking-[0.04em] text-accent"
                >
                  {tag.name}
                </span>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Delete control */}
      {!editing && (
        confirmingDelete ? (
          <div className="flex shrink-0 items-center gap-[8px] text-[11px]">
            <button
              type="button"
              onClick={() => { onDelete(); setConfirmingDelete(false); }}
              className="cursor-pointer text-danger hover:underline"
            >
              Delete
            </button>
            <button
              type="button"
              onClick={() => { setConfirmingDelete(false); }}
              className="cursor-pointer text-ink-3 hover:text-ink"
            >
              Cancel
            </button>
          </div>
        ) : (
          <button
            type="button"
            onClick={(e) => { e.stopPropagation(); setConfirmingDelete(true); }}
            className="shrink-0 cursor-pointer text-[18px] leading-none text-ink-4 opacity-0 transition-opacity group-hover:opacity-100 hover:text-danger"
            aria-label="Delete task"
          >
            ×
          </button>
        )
      )}
    </div>
  );
}

/* ── Add task row ──────────────────────────────────────────────────── */
interface AddTaskRowProps {
  onAdd: (title: string) => void;
}

function AddTaskRow({ onAdd }: AddTaskRowProps): React.JSX.Element {
  const [active, setActive] = useState(false);
  const [value, setValue] = useState("");
  const addedRef = useRef(false);

  if (!active) {
    return (
      <button
        type="button"
        onClick={() => { setActive(true); }}
        className="flex w-full cursor-pointer items-center gap-2 rounded-[10px] px-3 py-2 text-[14px] text-ink-4 hover:bg-paper-deep transition-colors"
      >
        <span className="flex h-[15px] w-[15px] shrink-0 items-center justify-center rounded-[4px] border border-dashed border-ink-4 text-[11px]" />
        <span>＋ Add task</span>
      </button>
    );
  }

  return (
    <div className="rounded-[10px] bg-paper p-3 shadow-card">
      <input
        autoFocus
        type="text"
        value={value}
        onChange={(e) => { setValue(e.target.value); }}
        onKeyDown={(e) => {
          if (e.key === "Enter" && value.trim().length > 0) {
            addedRef.current = true;
            onAdd(value.trim());
            setValue("");
            setActive(false);
          }
          if (e.key === "Escape") { addedRef.current = true; setValue(""); setActive(false); }
        }}
        onBlur={() => {
          if (value.trim().length > 0 && !addedRef.current) onAdd(value.trim());
          addedRef.current = false;
          setValue("");
          setActive(false);
        }}
        placeholder="Task title…"
        className="w-full bg-transparent text-[14px] text-ink outline-none placeholder-ink-4"
      />
    </div>
  );
}

/* ── Board column ──────────────────────────────────────────────────── */
interface BoardColumnProps {
  col: (typeof COLUMNS)[number];
  tasks: Task[];
  allTags: Tag[];
  onAdd: (title: string) => void;
  onToggleDone: (id: string) => void;
  onRename: (id: string, title: string) => void;
  onRemove: (id: string) => void;
  onAddTag: (taskId: string, name: string) => void;
  onRemoveTag: (taskId: string, tagId: string) => void;
}

function BoardColumn({
  col,
  tasks,
  allTags,
  onAdd,
  onToggleDone,
  onRename,
  onRemove,
  onAddTag,
  onRemoveTag,
}: BoardColumnProps): React.JSX.Element {
  const { setNodeRef } = useDroppable({ id: `col-${col.id}` });

  return (
    <div ref={setNodeRef} className="flex flex-col rounded-[14px] bg-paper-sunken p-[14px_12px_10px]">
      <div className="mb-3 flex items-center gap-2">
        {col.id === "doing" ? (
          <em className="font-sans text-[13px] font-[500] not-italic text-accent italic">
            {col.label}
          </em>
        ) : (
          <span className="font-sans text-[13px] font-[500] text-ink">{col.label}</span>
        )}
        <span className="font-mono text-[11px] text-ink-4">{String(tasks.length)}</span>
      </div>

      <div className="flex flex-1 flex-col gap-2">
        <SortableContext items={tasks.map((t) => t.id)} strategy={verticalListSortingStrategy}>
          {tasks.map((task) => (
            <TaskCard
              key={task.id}
              task={task}
              allTags={allTags}
              onToggleDone={() => { onToggleDone(task.id); }}
              onRename={(title) => { onRename(task.id, title); }}
              onDelete={() => { onRemove(task.id); }}
              onAddTag={(name) => { onAddTag(task.id, name); }}
              onRemoveTag={(tagId) => { onRemoveTag(task.id, tagId); }}
            />
          ))}
        </SortableContext>
      </div>

      <div className="mt-2">
        <AddTaskRow onAdd={onAdd} />
      </div>
    </div>
  );
}

/* ── Board screen ──────────────────────────────────────────────────── */
interface BoardScreenProps {
  tasks: Task[];
  allTags: Tag[];
  onCreate: (title: string, col: TaskColumn) => Promise<Task>;
  onMove: (id: string, col: TaskColumn, position: number) => Promise<void>;
  onRename: (id: string, title: string) => Promise<void>;
  onRemove: (id: string) => Promise<void>;
  onAddTag: (taskId: string, name: string) => Promise<void>;
  onRemoveTag: (taskId: string, tagId: string) => Promise<void>;
}

export function BoardScreen({
  tasks,
  allTags,
  onCreate,
  onMove,
  onRename,
  onRemove,
  onAddTag,
  onRemoveTag,
}: BoardScreenProps): React.JSX.Element {
  const [activeId, setActiveId] = useState<string | null>(null);
  const [boardTitle, setBoardTitle] = useState("Board");
  const [editingTitle, setEditingTitle] = useState(false);
  const [titleDraft, setTitleDraft] = useState("Board");
  const titleInputRef = useRef<HTMLInputElement>(null);
  const cancelTitleRef = useRef(false);

  useEffect(() => {
    getSetting("board.title").then((val) => {
      if (val !== null && val.trim().length > 0) {
        setBoardTitle(val);
        setTitleDraft(val);
      }
    }).catch(() => undefined);
  }, []);

  const startEditingTitle = (): void => {
    setTitleDraft(boardTitle);
    setEditingTitle(true);
    setTimeout(() => { titleInputRef.current?.select(); }, 0);
  };

  const commitTitle = (): void => {
    const trimmed = titleDraft.trim();
    const next = trimmed.length > 0 ? trimmed : "Board";
    setBoardTitle(next);
    setTitleDraft(next);
    setEditingTitle(false);
    void saveSetting("board.title", next);
  };

  const cancelTitle = (): void => {
    cancelTitleRef.current = true;
    setTitleDraft(boardTitle);
    setEditingTitle(false);
  };

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
  );

  const tasksByCol = (col: TaskColumn): Task[] =>
    tasks.filter((t) => t.col === col).sort((a, b) => a.position - b.position);

  const handleDragStart = (e: DragStartEvent): void => {
    setActiveId(String(e.active.id));
  };

  const handleDragEnd = (e: DragEndEvent): void => {
    const { active, over } = e;
    setActiveId(null);
    if (over === null || active.id === over.id) return;

    const draggedTask = tasks.find((t) => t.id === active.id);
    if (draggedTask === undefined) return;

    const overId = String(over.id);

    // Dropped onto a column droppable (e.g. empty column or column background)
    if (overId.startsWith("col-")) {
      const targetCol = overId.slice(4) as TaskColumn;
      if (draggedTask.col === targetCol) return;
      const newPosition = tasksByCol(targetCol).length;
      void onMove(draggedTask.id, targetCol, newPosition);
      return;
    }

    // Dropped onto another task
    const overTask = tasks.find((t) => t.id === overId);
    if (overTask === undefined) return;

    const targetCol = overTask.col;
    const colTasks = tasksByCol(targetCol).filter((t) => t.id !== draggedTask.id);
    const overIndex = colTasks.findIndex((t) => t.id === overId);
    const newPosition = overIndex >= 0 ? overIndex : colTasks.length;

    void onMove(draggedTask.id, targetCol, newPosition);
  };

  const handleToggleDone = (id: string): void => {
    const task = tasks.find((t) => t.id === id);
    if (task === undefined) return;
    const newCol: TaskColumn = task.col === "done" ? "todo" : "done";
    void onMove(id, newCol, tasksByCol(newCol).length);
  };

  const draggingTask = tasks.find((t) => t.id === activeId);
  const totalCount = tasks.length;

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      <div className="shrink-0 px-7 py-[18px] pb-[6px]">
        <div className="flex items-baseline gap-3">
          {editingTitle ? (
            <input
              ref={titleInputRef}
              autoFocus
              type="text"
              value={titleDraft}
              onChange={(e) => { setTitleDraft(e.target.value); }}
              onBlur={() => {
                if (!cancelTitleRef.current) commitTitle();
                cancelTitleRef.current = false;
              }}
              onKeyDown={(e) => {
                if (e.key === "Enter") commitTitle();
                if (e.key === "Escape") cancelTitle();
              }}
              className="font-sans text-[26px] font-[500] tracking-[-0.02em] text-ink bg-transparent border-b border-accent outline-none w-[220px]"
            />
          ) : (
            <h2
              className="font-sans text-[26px] font-[500] tracking-[-0.02em] text-ink cursor-text hover:text-ink-2 transition-colors"
              onClick={startEditingTitle}
              title="Click to rename"
            >
              {boardTitle}
            </h2>
          )}
          <span className="font-mono text-[12px] text-ink-3">
            {String(totalCount)} {totalCount === 1 ? "task" : "tasks"} · drag to move
          </span>
        </div>
      </div>

      <div className="flex-1 overflow-auto px-7 py-3 pb-5">
        <DndContext sensors={sensors} onDragStart={handleDragStart} onDragEnd={handleDragEnd}>
          <div className="grid min-h-full grid-cols-3 gap-[18px]">
            {COLUMNS.map((col) => (
              <BoardColumn
                key={col.id}
                col={col}
                tasks={tasksByCol(col.id)}
                allTags={allTags}
                onAdd={(title) => { void onCreate(title, col.id); }}
                onToggleDone={handleToggleDone}
                onRename={(id, title) => { void onRename(id, title); }}
                onRemove={(id) => { void onRemove(id); }}
                onAddTag={(taskId, name) => { void onAddTag(taskId, name); }}
                onRemoveTag={(taskId, tagId) => { void onRemoveTag(taskId, tagId); }}
              />
            ))}
          </div>

          <DragOverlay>
            {draggingTask !== undefined ? (
              <TaskCard
                task={draggingTask}
                isDragging
                allTags={allTags}
                onToggleDone={() => undefined}
                onRename={() => undefined}
                onDelete={() => undefined}
                onAddTag={() => undefined}
                onRemoveTag={() => undefined}
              />
            ) : null}
          </DragOverlay>
        </DndContext>
      </div>
    </div>
  );
}
