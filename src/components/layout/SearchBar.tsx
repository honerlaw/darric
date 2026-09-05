import type React from "react";
import { useEffect, useRef, useState } from "react";
import { useSearch } from "../../hooks/useSearch";
import type { SearchResultNote, SearchResultSession, SearchResultTask, Tag } from "../../types";

interface SearchBarProps {
  onClose: () => void;
  onNavigateMeeting: (sessionId: string) => void;
  onNavigateNote: (noteId: string) => void;
  onNavigateBoard: () => void;
}

type ResultItem =
  | { kind: "session"; data: SearchResultSession }
  | { kind: "note"; data: SearchResultNote }
  | { kind: "task"; data: SearchResultTask };

function formatDate(iso: string): string {
  const d = new Date(iso);
  return d.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

function TagPills({ tags }: { tags: Tag[] }): React.JSX.Element | null {
  if (tags.length === 0) return null;
  return (
    <div className="mt-[3px] flex flex-wrap gap-1">
      {tags.slice(0, 4).map((t) => (
        <span
          key={t.id}
          className="rounded-full bg-paper-sunken px-[7px] py-[1px] font-mono text-[10px] text-ink-3"
        >
          {t.name}
        </span>
      ))}
    </div>
  );
}

function ResultRow({
  item,
  selected,
  onClick,
}: {
  item: ResultItem;
  selected: boolean;
  onClick: () => void;
}): React.JSX.Element {
  const ref = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (selected) ref.current?.scrollIntoView({ block: "nearest" });
  }, [selected]);

  let kindLabel: string;
  if (item.kind === "session") kindLabel = "Meeting";
  else if (item.kind === "note") kindLabel = "Note";
  else kindLabel = "Task";

  let kindColor: string;
  if (item.kind === "session") kindColor = "text-accent";
  else if (item.kind === "note") kindColor = "text-emerald-500 dark:text-emerald-400";
  else kindColor = "text-amber-500 dark:text-amber-400";

  let title: string;
  if (item.kind === "session") {
    title = item.data.topic ?? "Untitled meeting";
  } else {
    const fallback = item.kind === "note" ? "Untitled note" : "Untitled task";
    title = item.data.title !== "" ? item.data.title : fallback;
  }

  const snippet = item.kind !== "task" ? item.data.snippet : null;
  const tags = item.data.tags;
  const date = item.kind === "session" ? formatDate(item.data.started_at) : null;

  return (
    <button
      ref={ref}
      type="button"
      onClick={onClick}
      className={`w-full cursor-pointer px-4 py-[10px] text-left transition-colors ${
        selected ? "bg-paper-sunken" : "hover:bg-paper-sunken/60"
      }`}
    >
      <div className="flex items-baseline gap-2">
        <span className={`shrink-0 font-mono text-[10px] tracking-eyebrow uppercase ${kindColor}`}>
          {kindLabel}
        </span>
        <span className="truncate text-[14px] font-[450] text-ink">{title}</span>
        {date !== null && (
          <span className="ml-auto shrink-0 font-mono text-[11px] text-ink-4">{date}</span>
        )}
      </div>
      {snippet !== null && snippet.length > 0 && (
        <p className="mt-[2px] line-clamp-1 text-[12px] text-ink-3">{snippet}</p>
      )}
      <TagPills tags={tags} />
    </button>
  );
}

function SectionHeader({ label, count }: { label: string; count: number }): React.JSX.Element {
  return (
    <div className="flex items-center gap-2 border-t border-line px-4 py-[6px] first:border-t-0">
      <span className="font-mono text-[10px] tracking-eyebrow text-ink-4 uppercase">{label}</span>
      <span className="font-mono text-[10px] text-ink-4">({count})</span>
    </div>
  );
}

const MAX_PER_GROUP = 5;

export function SearchBar({
  onClose,
  onNavigateMeeting,
  onNavigateNote,
  onNavigateBoard,
}: SearchBarProps): React.JSX.Element {
  const { query, setQuery, results, isSearching, clear } = useSearch();
  const inputRef = useRef<HTMLInputElement>(null);
  const [selectedIndex, setSelectedIndex] = useState(0);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  // Reset selection when results change
  useEffect(() => {
    setSelectedIndex(0);
  }, [results]);

  // Build flat list of result items for keyboard nav
  const items: ResultItem[] = [];
  if (results !== null) {
    results.sessions
      .slice(0, MAX_PER_GROUP)
      .forEach((s) => items.push({ kind: "session", data: s }));
    results.notes.slice(0, MAX_PER_GROUP).forEach((n) => items.push({ kind: "note", data: n }));
    results.tasks.slice(0, MAX_PER_GROUP).forEach((t) => items.push({ kind: "task", data: t }));
  }

  const activate = (item: ResultItem): void => {
    clear();
    onClose();
    if (item.kind === "session") onNavigateMeeting(item.data.id);
    else if (item.kind === "note") onNavigateNote(item.data.id);
    else onNavigateBoard();
  };
  const activateRef = useRef(activate);
  activateRef.current = activate;

  useEffect(() => {
    const handler = (e: KeyboardEvent): void => {
      if (e.key === "Escape") {
        clear();
        onClose();
        return;
      }
      if (items.length === 0) return;
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelectedIndex((i) => Math.min(i + 1, items.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelectedIndex((i) => Math.max(i - 1, 0));
      } else if (e.key === "Enter") {
        e.preventDefault();
        const item = items[selectedIndex];
        if (item !== undefined) activateRef.current(item);
      }
    };
    window.addEventListener("keydown", handler);
    return () => {
      window.removeEventListener("keydown", handler);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [items, selectedIndex]);

  const hasResults = results !== null && items.length > 0;
  const isEmpty = results !== null && items.length === 0;

  // Track running index for correct selectedIndex mapping across groups
  let runningIndex = 0;

  return (
    <div
      className="fixed inset-0 z-[60] flex items-start justify-center bg-ink/20 pt-[15vh] backdrop-blur-[2px]"
      onClick={(e) => {
        if (e.target === e.currentTarget) {
          clear();
          onClose();
        }
      }}
    >
      <div className="flex w-[560px] flex-col overflow-hidden rounded-[14px] bg-paper shadow-[0_8px_32px_rgba(10,10,10,0.18)]">
        {/* Search input */}
        <div className="flex items-center gap-3 border-b border-line px-4 py-3">
          <svg
            className="shrink-0 text-ink-3"
            width="16"
            height="16"
            viewBox="0 0 16 16"
            fill="none"
            aria-hidden="true"
          >
            <circle cx="6.5" cy="6.5" r="5" stroke="currentColor" strokeWidth="1.5" />
            <path
              d="M10.5 10.5L14 14"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
            />
          </svg>
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
            }}
            placeholder="Search meetings, notes, tasks…"
            className="flex-1 bg-transparent text-[14px] text-ink outline-none placeholder:text-ink-4"
          />
          {isSearching && (
            <span className="shrink-0 font-mono text-[10px] text-ink-4">searching…</span>
          )}
          <kbd className="shrink-0 rounded border border-line px-1 font-mono text-[10px] text-ink-4">
            esc
          </kbd>
        </div>

        {/* Results */}
        {(hasResults || isEmpty) && (
          <div className="max-h-[380px] overflow-y-auto">
            {isEmpty && (
              <p className="px-4 py-6 text-center text-[13px] text-ink-3">
                No results for &ldquo;{query}&rdquo;
              </p>
            )}

            {hasResults && (
              <>
                {results.sessions.length > 0 &&
                  (() => {
                    const groupItems = results.sessions.slice(0, MAX_PER_GROUP);
                    const groupStart = runningIndex;
                    runningIndex += groupItems.length;
                    const extra = results.sessions.length - MAX_PER_GROUP;
                    return (
                      <div>
                        <SectionHeader label="Meetings" count={results.sessions.length} />
                        {groupItems.map((s, i) => (
                          <ResultRow
                            key={s.id}
                            item={{ kind: "session", data: s }}
                            selected={selectedIndex === groupStart + i}
                            onClick={() => {
                              activate({ kind: "session", data: s });
                            }}
                          />
                        ))}
                        {extra > 0 && (
                          <p className="px-4 py-[6px] font-mono text-[10px] text-ink-4">
                            and {extra} more
                          </p>
                        )}
                      </div>
                    );
                  })()}

                {results.notes.length > 0 &&
                  (() => {
                    const groupItems = results.notes.slice(0, MAX_PER_GROUP);
                    const groupStart = runningIndex;
                    runningIndex += groupItems.length;
                    const extra = results.notes.length - MAX_PER_GROUP;
                    return (
                      <div>
                        <SectionHeader label="Notes" count={results.notes.length} />
                        {groupItems.map((n, i) => (
                          <ResultRow
                            key={n.id}
                            item={{ kind: "note", data: n }}
                            selected={selectedIndex === groupStart + i}
                            onClick={() => {
                              activate({ kind: "note", data: n });
                            }}
                          />
                        ))}
                        {extra > 0 && (
                          <p className="px-4 py-[6px] font-mono text-[10px] text-ink-4">
                            and {extra} more
                          </p>
                        )}
                      </div>
                    );
                  })()}

                {results.tasks.length > 0 &&
                  (() => {
                    const groupItems = results.tasks.slice(0, MAX_PER_GROUP);
                    const groupStart = runningIndex;
                    runningIndex += groupItems.length;
                    const extra = results.tasks.length - MAX_PER_GROUP;
                    return (
                      <div>
                        <SectionHeader label="Tasks" count={results.tasks.length} />
                        {groupItems.map((t, i) => (
                          <ResultRow
                            key={t.id}
                            item={{ kind: "task", data: t }}
                            selected={selectedIndex === groupStart + i}
                            onClick={() => {
                              activate({ kind: "task", data: t });
                            }}
                          />
                        ))}
                        {extra > 0 && (
                          <p className="px-4 py-[6px] font-mono text-[10px] text-ink-4">
                            and {extra} more
                          </p>
                        )}
                      </div>
                    );
                  })()}
              </>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
