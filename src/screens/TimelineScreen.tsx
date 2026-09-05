import { useVirtualizer } from "@tanstack/react-virtual";
import type React from "react";
import { useRef, useState } from "react";
import type { Note, Session, Tag, Task, TimelineEntry } from "../types";

type FilterKey = "all" | "meeting" | "note" | "task" | "darric";
type RangeMode = "day" | "3d" | "7d" | "14d" | "30d" | "60d" | "all";

type FlatItem =
  | { type: "day-header"; label: string; dateKey: string }
  | { type: "entry"; entry: TimelineEntry };

function fmtTime(iso: string): string {
  return new Date(iso).toLocaleTimeString("en-US", {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  });
}

function sessionDuration(s: Session): number | undefined {
  if (s.ended_at === null) return undefined;
  return s.recorded_minutes > 0 ? s.recorded_minutes : undefined;
}

const COL_LABEL: Record<string, string> = {
  todo: "to-do",
  doing: "doing",
  done: "done",
};

const DAY_NAMES = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
const MONTH_NAMES = [
  "january",
  "february",
  "march",
  "april",
  "may",
  "june",
  "july",
  "august",
  "september",
  "october",
  "november",
  "december",
];

function buildTimeline(
  sessions: Session[],
  notes: Note[],
  tasks: Task[],
  agentEntries: TimelineEntry[],
  targetDateStr: string,
): TimelineEntry[] {
  const sessionEntries: TimelineEntry[] = sessions
    .filter((s) => new Date(s.started_at).toDateString() === targetDateStr)
    .map((s) => ({
      id: `session-${s.id}`,
      kind: "meeting" as const,
      timestamp: s.started_at,
      title: s.topic ?? "Meeting",
      body: s.ended_at !== null ? "Notes and transcript captured." : "Recording in progress…",
      refId: s.id,
      durationMin: sessionDuration(s),
      tags: s.tags,
    }));

  const noteEntries: TimelineEntry[] = notes
    .filter((n) => new Date(n.created_at).toDateString() === targetDateStr)
    .map((n) => ({
      id: `note-${n.id}`,
      kind: "note" as const,
      timestamp: n.created_at,
      title: n.title.length > 0 ? n.title : "Untitled note",
      body: n.body.length > 0 ? n.body.slice(0, 140) : undefined,
      refId: n.id,
      tags: n.tags,
    }));

  const taskEntries: TimelineEntry[] = tasks
    .filter((t) => new Date(t.created_at).toDateString() === targetDateStr)
    .map((t) => ({
      id: `task-${t.id}`,
      kind: "task" as const,
      timestamp: t.created_at,
      title: t.title,
      body: COL_LABEL[t.col] ?? t.col,
      refId: t.id,
      tags: t.tags,
    }));

  const dateAgent = agentEntries.filter(
    (e) => new Date(e.timestamp).toDateString() === targetDateStr,
  );

  return [...sessionEntries, ...noteEntries, ...taskEntries, ...dateAgent].sort(
    (a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime(),
  );
}

function buildRangeTimeline(
  sessions: Session[],
  notes: Note[],
  tasks: Task[],
  agentEntries: TimelineEntry[],
  startDate: Date | null,
): TimelineEntry[] {
  const after = (ts: string): boolean =>
    startDate === null || new Date(ts).getTime() >= startDate.getTime();

  const sessionEntries: TimelineEntry[] = sessions
    .filter((s) => after(s.started_at))
    .map((s) => ({
      id: `session-${s.id}`,
      kind: "meeting" as const,
      timestamp: s.started_at,
      title: s.topic ?? "Meeting",
      body: s.ended_at !== null ? "Notes and transcript captured." : "Recording in progress…",
      refId: s.id,
      durationMin: sessionDuration(s),
      tags: s.tags,
    }));

  const noteEntries: TimelineEntry[] = notes
    .filter((n) => after(n.created_at))
    .map((n) => ({
      id: `note-${n.id}`,
      kind: "note" as const,
      timestamp: n.created_at,
      title: n.title.length > 0 ? n.title : "Untitled note",
      body: n.body.length > 0 ? n.body.slice(0, 140) : undefined,
      refId: n.id,
      tags: n.tags,
    }));

  const taskEntries: TimelineEntry[] = tasks
    .filter((t) => after(t.created_at))
    .map((t) => ({
      id: `task-${t.id}`,
      kind: "task" as const,
      timestamp: t.created_at,
      title: t.title,
      body: COL_LABEL[t.col] ?? t.col,
      refId: t.id,
      tags: t.tags,
    }));

  const filteredAgent = agentEntries.filter((e) => after(e.timestamp));

  return [...sessionEntries, ...noteEntries, ...taskEntries, ...filteredAgent].sort(
    (a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime(),
  );
}

function buildFlatItems(entries: TimelineEntry[]): FlatItem[] {
  const groups = new Map<string, TimelineEntry[]>();
  for (const entry of entries) {
    const key = new Date(entry.timestamp).toDateString();
    const group = groups.get(key) ?? [];
    group.push(entry);
    groups.set(key, group);
  }

  const sortedKeys = [...groups.keys()].sort(
    (a, b) => new Date(a).getTime() - new Date(b).getTime(),
  );

  const flat: FlatItem[] = [];
  for (const key of sortedKeys) {
    const d = new Date(key);
    const dayName = DAY_NAMES[d.getDay()] ?? "Day";
    const label = `${dayName}, ${String(d.getDate())} ${MONTH_NAMES[d.getMonth()] ?? ""}`;
    flat.push({ type: "day-header", label, dateKey: key });
    for (const entry of groups.get(key) ?? []) {
      flat.push({ type: "entry", entry });
    }
  }
  return flat;
}

function buildTagTimeline(
  sessions: Session[],
  notes: Note[],
  tasks: Task[],
  tagId: string,
): TimelineEntry[] {
  const sessionEntries: TimelineEntry[] = sessions
    .filter((s) => s.tags.some((t) => t.id === tagId))
    .map((s) => ({
      id: `session-${s.id}`,
      kind: "meeting" as const,
      timestamp: s.started_at,
      title: s.topic ?? "Meeting",
      body: s.ended_at !== null ? "Notes and transcript captured." : "Recording in progress…",
      refId: s.id,
      durationMin: sessionDuration(s),
      tags: s.tags,
    }));

  const noteEntries: TimelineEntry[] = notes
    .filter((n) => n.tags.some((t) => t.id === tagId))
    .map((n) => ({
      id: `note-${n.id}`,
      kind: "note" as const,
      timestamp: n.created_at,
      title: n.title.length > 0 ? n.title : "Untitled note",
      body: n.body.length > 0 ? n.body.slice(0, 140) : undefined,
      refId: n.id,
      tags: n.tags,
    }));

  const taskEntries: TimelineEntry[] = tasks
    .filter((t) => t.tags.some((tg) => tg.id === tagId))
    .map((t) => ({
      id: `task-${t.id}`,
      kind: "task" as const,
      timestamp: t.created_at,
      title: t.title,
      body: COL_LABEL[t.col] ?? t.col,
      refId: t.id,
      tags: t.tags,
    }));

  return [...sessionEntries, ...noteEntries, ...taskEntries].sort(
    (a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime(),
  );
}

function applyFilter(entries: TimelineEntry[], filter: FilterKey): TimelineEntry[] {
  if (filter === "all") return entries;
  if (filter === "darric")
    return entries.filter((e) => e.kind === "you-asked-darric" || e.kind === "darric");
  return entries.filter((e) => e.kind === filter);
}

function kindLabel(entry: TimelineEntry): string {
  if (entry.kind === "meeting") {
    return entry.durationMin !== undefined
      ? `meeting · ${String(entry.durationMin)} min`
      : "meeting";
  }
  if (entry.kind === "you-asked-darric") return "you asked Darric";
  return entry.kind;
}

const RANGE_DAYS: Partial<Record<RangeMode, number>> = {
  "3d": 3,
  "7d": 7,
  "14d": 14,
  "30d": 30,
  "60d": 60,
};

function rangeModeStartDate(mode: RangeMode): Date | null {
  if (mode === "all" || mode === "day") return null;
  const days = RANGE_DAYS[mode];
  if (days === undefined) return null;
  const d = new Date();
  d.setHours(0, 0, 0, 0);
  d.setDate(d.getDate() - (days - 1));
  return d;
}

function rangeModeLabel(mode: RangeMode): string {
  if (mode === "all") return "All time";
  const days = RANGE_DAYS[mode];
  return days !== undefined ? `Last ${String(days)} days` : "All time";
}

/* ── Tag selector ────────────────────────────────────────────────────── */
interface TagSelectorProps {
  allTags: Tag[];
  activeTag: Tag | null;
  onSelect: (tag: Tag | null) => void;
}

function TagSelector({ allTags, activeTag, onSelect }: TagSelectorProps): React.JSX.Element | null {
  if (allTags.length === 0) return null;
  return (
    <div className="mb-5 flex flex-wrap items-center gap-[6px]">
      <span className="mr-1 font-mono text-[10px] tracking-[0.08em] text-ink-4 uppercase">
        tags
      </span>
      {allTags.map((tag) => {
        const isActive = activeTag?.id === tag.id;
        return (
          <button
            key={tag.id}
            type="button"
            onClick={() => {
              onSelect(isActive ? null : tag);
            }}
            className={`cursor-pointer rounded-full px-[9px] py-[2px] font-mono text-[10px] tracking-[0.04em] transition-colors ${
              isActive ? "bg-accent text-accent-fg" : "bg-accent/10 text-accent hover:bg-accent/20"
            }`}
          >
            {tag.name}
          </button>
        );
      })}
      {activeTag !== null && (
        <button
          type="button"
          onClick={() => {
            onSelect(null);
          }}
          className="cursor-pointer font-mono text-[10px] text-ink-4 transition-colors hover:text-ink"
        >
          × clear
        </button>
      )}
    </div>
  );
}

/* ── View mode bar ──────────────────────────────────────────────────── */
const RANGE_MODES: { key: RangeMode; label: string }[] = [
  { key: "day", label: "day" },
  { key: "3d", label: "3d" },
  { key: "7d", label: "7d" },
  { key: "14d", label: "14d" },
  { key: "30d", label: "30d" },
  { key: "60d", label: "60d" },
  { key: "all", label: "all" },
];

function ViewModeBar({
  active,
  onChange,
}: {
  active: RangeMode;
  onChange: (m: RangeMode) => void;
}): React.JSX.Element {
  return (
    <div className="flex flex-wrap gap-x-5 gap-y-1">
      {RANGE_MODES.map((m) => (
        <button
          key={m.key}
          type="button"
          onClick={() => {
            onChange(m.key);
          }}
          className={`cursor-pointer font-mono text-[11px] tracking-[0.06em] transition-colors ${
            active === m.key
              ? "text-ink underline underline-offset-[3px]"
              : "text-ink-3 hover:text-ink-2"
          }`}
        >
          {m.label}
        </button>
      ))}
    </div>
  );
}

/* ── Filter bar ─────────────────────────────────────────────────────── */
interface FilterBarProps {
  allEntries: TimelineEntry[];
  active: FilterKey;
  onChange: (f: FilterKey) => void;
}

const FILTER_DEFS: { key: FilterKey; label: string }[] = [
  { key: "all", label: "all" },
  { key: "meeting", label: "meetings" },
  { key: "note", label: "notes" },
  { key: "task", label: "tasks" },
  { key: "darric", label: "Darric" },
];

function countForFilter(entries: TimelineEntry[], key: FilterKey): number {
  return applyFilter(entries, key).length;
}

function FilterBar({ allEntries, active, onChange }: FilterBarProps): React.JSX.Element {
  const visible = FILTER_DEFS.filter(
    (f) => f.key === "all" || countForFilter(allEntries, f.key) > 0,
  );

  return (
    <div className="mb-6 flex flex-wrap gap-x-5 gap-y-1">
      {visible.map((f) => {
        const count = countForFilter(allEntries, f.key);
        const isActive = active === f.key;
        return (
          <button
            key={f.key}
            type="button"
            onClick={() => {
              onChange(f.key);
            }}
            className={`cursor-pointer font-mono text-[11px] tracking-[0.06em] transition-colors ${
              isActive ? "text-ink underline underline-offset-[3px]" : "text-ink-3 hover:text-ink-2"
            }`}
          >
            {f.label} · {String(count)}
          </button>
        );
      })}
    </div>
  );
}

/* ── Day header (range mode) ─────────────────────────────────────────── */
function DayHeader({ label }: { label: string }): React.JSX.Element {
  return (
    <div className="border-b border-line pt-8 pb-2 font-mono text-[11px] tracking-[0.06em] text-ink-3 uppercase">
      {label}
    </div>
  );
}

/* ── Entry row ───────────────────────────────────────────────────────── */
interface EntryRowProps {
  entry: TimelineEntry;
  isConfirming: boolean;
  onClickNote?: ((id: string) => void) | undefined;
  onClickMeeting?: ((id: string) => void) | undefined;
  onDeleteRequest: () => void;
  onDeleteConfirm: () => void;
  onDeleteCancel: () => void;
  showBorder?: boolean;
}

function EntryRow({
  entry,
  isConfirming,
  onClickNote,
  onClickMeeting,
  onDeleteRequest,
  onDeleteConfirm,
  onDeleteCancel,
  showBorder = false,
}: EntryRowProps): React.JSX.Element {
  const isDarric = entry.kind === "darric";
  const isYouAsked = entry.kind === "you-asked-darric";
  const isDeletable = entry.kind === "note" || entry.kind === "meeting";
  const clickable = isDeletable && entry.refId !== undefined;

  const handleClick = (): void => {
    if (isConfirming) return;
    if (!clickable || entry.refId === undefined) return;
    if (entry.kind === "note") onClickNote?.(entry.refId);
    if (entry.kind === "meeting") onClickMeeting?.(entry.refId);
  };

  return (
    <div
      className={`group grid [grid-template-columns:56px_1fr] gap-x-[22px] py-[18px] ${showBorder ? "border-b border-line" : ""}`}
    >
      <div className="pt-[2px] font-mono text-[11px] text-ink-4">{fmtTime(entry.timestamp)}</div>

      <div className="flex items-start gap-3">
        <div
          className={`min-w-0 flex-1 ${clickable && !isConfirming ? "cursor-pointer" : ""}`}
          onClick={handleClick}
        >
          <div
            className={`mb-[3px] font-mono text-[11px] tracking-[0.06em] ${
              isDarric ? "text-accent italic" : "text-ink-3"
            }`}
          >
            {kindLabel(entry)}
          </div>

          {entry.title !== undefined && (
            <div
              className={`text-[18px] leading-snug tracking-tight ${
                isYouAsked
                  ? "font-[400] text-ink-2 italic"
                  : `font-[400] text-ink ${clickable && !isConfirming ? "group-hover:text-ink-2" : ""}`
              }`}
            >
              {isYouAsked ? `"${entry.title}"` : entry.title}
            </div>
          )}

          {(entry.body !== undefined || entry.isStreaming === true) && (
            <div
              className={`mt-[4px] text-[15px] leading-[1.6] text-ink-2 ${
                isDarric ? "mt-[8px] border-l-[2px] border-accent pl-[14px] italic" : ""
              }`}
            >
              {entry.body}
              {entry.isStreaming === true && (
                <span className="ml-[2px] inline-block h-[1em] w-[2px] animate-[blink_1s_step-end_infinite] bg-accent align-middle" />
              )}
            </div>
          )}

          {entry.tags !== undefined && entry.tags.length > 0 && (
            <div className="mt-[6px] flex flex-wrap gap-[4px]">
              {entry.tags.map((tag) => (
                <span
                  key={tag.id}
                  className="rounded-full bg-accent/10 px-[7px] py-[1px] font-mono text-[9px] tracking-[0.04em] text-accent"
                >
                  {tag.name}
                </span>
              ))}
            </div>
          )}
        </div>

        {isDeletable && (
          <div className="shrink-0 pt-[3px]">
            {isConfirming ? (
              <span className="flex items-center gap-[10px] text-[12px]">
                <span className="text-ink-3">Delete?</span>
                <button
                  type="button"
                  onClick={onDeleteConfirm}
                  className="cursor-pointer text-danger hover:underline"
                >
                  Yes
                </button>
                <button
                  type="button"
                  onClick={onDeleteCancel}
                  className="cursor-pointer text-ink-3 hover:text-ink"
                >
                  Cancel
                </button>
              </span>
            ) : (
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  onDeleteRequest();
                }}
                className="cursor-pointer text-[18px] leading-none text-ink-4 opacity-0 transition-opacity group-hover:opacity-100 hover:text-danger"
                aria-label="Delete"
              >
                ×
              </button>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

/* ── Screen ──────────────────────────────────────────────────────────── */
interface TimelineScreenProps {
  sessions: Session[];
  notes: Note[];
  tasks: Task[];
  agentEntries: TimelineEntry[];
  allTags: Tag[];
  onNavigateNotes: (noteId: string) => void;
  onNavigateMeeting: (sessionId?: string) => void;
  onDeleteNote: (id: string) => Promise<void>;
  onDeleteMeeting: (id: string) => Promise<void>;
}

export function TimelineScreen({
  sessions,
  notes,
  tasks,
  agentEntries,
  allTags,
  onNavigateNotes,
  onNavigateMeeting,
  onDeleteNote,
  onDeleteMeeting,
}: TimelineScreenProps): React.JSX.Element {
  const [confirmingId, setConfirmingId] = useState<string | null>(null);
  const [activeFilter, setActiveFilter] = useState<FilterKey>("all");
  const [rangeMode, setRangeMode] = useState<RangeMode>("day");
  const [activeTag, setActiveTag] = useState<Tag | null>(null);
  const [selectedDate, setSelectedDate] = useState<Date>(() => {
    const d = new Date();
    d.setHours(0, 0, 0, 0);
    return d;
  });

  const scrollRef = useRef<HTMLDivElement>(null);

  const todayMidnight = new Date();
  todayMidnight.setHours(0, 0, 0, 0);
  const isToday = selectedDate.toDateString() === todayMidnight.toDateString();
  const isYesterday =
    selectedDate.toDateString() === new Date(todayMidnight.getTime() - 86_400_000).toDateString();

  const prevDay = (): void => {
    setSelectedDate((d) => new Date(d.getTime() - 86_400_000));
    setActiveFilter("all");
    setConfirmingId(null);
  };
  const nextDay = (): void => {
    if (!isToday) setSelectedDate((d) => new Date(d.getTime() + 86_400_000));
    setActiveFilter("all");
    setConfirmingId(null);
  };

  const handleRangeModeChange = (m: RangeMode): void => {
    setRangeMode(m);
    setActiveFilter("all");
    setConfirmingId(null);
  };

  const handleSelectTag = (tag: Tag | null): void => {
    setActiveTag(tag);
    setActiveFilter("all");
    setConfirmingId(null);
  };

  const isTagMode = activeTag !== null;

  /* ── Tag mode data ── */
  const tagAllEntries = isTagMode ? buildTagTimeline(sessions, notes, tasks, activeTag.id) : [];
  const tagEntries = applyFilter(tagAllEntries, activeFilter);

  /* ── Day mode data ── */
  const dayAllEntries = buildTimeline(
    sessions,
    notes,
    tasks,
    agentEntries,
    selectedDate.toDateString(),
  );
  const dayEntries = applyFilter(dayAllEntries, activeFilter);

  /* ── Range mode data ── */
  const rangeAllEntries = buildRangeTimeline(
    sessions,
    notes,
    tasks,
    agentEntries,
    rangeMode !== "day" ? rangeModeStartDate(rangeMode) : null,
  );
  const rangeFiltered = applyFilter(rangeAllEntries, activeFilter);
  const flatItems = buildFlatItems(rangeFiltered);

  /* ── Virtualizer (range mode only) ── */
  const virtualizer = useVirtualizer({
    count: rangeMode !== "day" ? flatItems.length : 0,
    getScrollElement: () => scrollRef.current,
    estimateSize: (i) => (flatItems[i]?.type === "day-header" ? 60 : 84),
    measureElement: (el) => el.getBoundingClientRect().height,
    overscan: 5,
  });

  const dayName = DAY_NAMES[selectedDate.getDay()] ?? "Day";
  const monthName = MONTH_NAMES[selectedDate.getMonth()] ?? "unknown";
  const dateStr = `${String(selectedDate.getDate())} ${monthName}`;
  let eyebrowLabel: string;
  if (isToday) eyebrowLabel = "Today";
  else if (isYesterday) eyebrowLabel = "Yesterday";
  else eyebrowLabel = dateStr;

  const filterLabel = activeFilter === "darric" ? "Darric" : activeFilter;
  const rangeModeStr = rangeModeLabel(rangeMode).toLowerCase();

  let dayEmptyText: string;
  if (activeFilter === "all") {
    dayEmptyText = isToday
      ? "Nothing here yet — start recording or add a note."
      : "Nothing logged that day.";
  } else {
    const when = isToday ? "today" : "that day";
    dayEmptyText = `No ${filterLabel} entries ${when}.`;
  }

  let rangeEmptyText: string;
  if (activeFilter === "all") {
    rangeEmptyText =
      rangeMode === "all" ? "Nothing logged yet." : `Nothing logged in the ${rangeModeStr}.`;
  } else {
    rangeEmptyText = `No ${filterLabel} entries in the ${rangeModeStr}.`;
  }

  const handleConfirmDelete = (entry: TimelineEntry): void => {
    if (entry.refId === undefined) return;
    if (entry.kind === "note") void onDeleteNote(entry.refId);
    else if (entry.kind === "meeting") void onDeleteMeeting(entry.refId);
    setConfirmingId(null);
  };

  const isRangeMode = rangeMode !== "day";
  const rangeEntryCount = rangeFiltered.length;
  const dayEntryCount = dayEntries.length;
  const tagEntryCount = tagEntries.length;

  return (
    <div ref={scrollRef} className="flex-1 overflow-y-auto">
      <div className="mx-auto max-w-[760px] px-8 py-10">
        {/* Tag selector — always visible when tags exist */}
        <TagSelector allTags={allTags} activeTag={activeTag} onSelect={handleSelectTag} />

        {isTagMode ? (
          /* ── Tag mode ─────────────────────────────────────────────── */
          <>
            <div className="mb-2 font-mono text-[11px] tracking-eyebrow text-accent uppercase">
              Tagged
            </div>
            <h1 className="font-sans text-[56px] leading-none font-[500] tracking-[-0.03em] text-ink">
              {activeTag.name}
            </h1>
            <div className="mt-2 mb-6 font-mono text-[13px] text-ink-3">
              {String(tagEntryCount)} {tagEntryCount === 1 ? "entry" : "entries"} · all time
            </div>

            <FilterBar
              allEntries={tagAllEntries}
              active={activeFilter}
              onChange={setActiveFilter}
            />
            <div className="border-t border-line" />

            {tagEntries.length === 0 ? (
              <div className="py-16 text-center text-[15px] text-ink-4 italic">
                {activeFilter === "all"
                  ? `No entries tagged "${activeTag.name}".`
                  : `No ${activeFilter} entries tagged "${activeTag.name}".`}
              </div>
            ) : (
              <div className="divide-y divide-line">
                {tagEntries.map((entry) => (
                  <EntryRow
                    key={entry.id}
                    entry={entry}
                    isConfirming={confirmingId === entry.id}
                    onClickNote={(id) => {
                      onNavigateNotes(id);
                    }}
                    onClickMeeting={(id) => {
                      onNavigateMeeting(id);
                    }}
                    onDeleteRequest={() => {
                      setConfirmingId(entry.id);
                    }}
                    onDeleteConfirm={() => {
                      handleConfirmDelete(entry);
                    }}
                    onDeleteCancel={() => {
                      setConfirmingId(null);
                    }}
                  />
                ))}
              </div>
            )}
          </>
        ) : (
          /* ── Day / range mode ─────────────────────────────────────── */
          <>
            {/* View mode selector */}
            <div className="mb-6">
              <ViewModeBar active={rangeMode} onChange={handleRangeModeChange} />
            </div>

            {/* Header */}
            {!isRangeMode ? (
              <>
                <div className="mb-2 flex items-center gap-3">
                  <button
                    type="button"
                    onClick={prevDay}
                    className="cursor-pointer font-mono text-[11px] text-ink-3 select-none hover:text-ink"
                    aria-label="Previous day"
                  >
                    ←
                  </button>
                  <div className="font-mono text-[11px] tracking-eyebrow text-accent uppercase">
                    {eyebrowLabel}
                  </div>
                  <button
                    type="button"
                    onClick={nextDay}
                    disabled={isToday}
                    className={`font-mono text-[11px] select-none ${isToday ? "cursor-default text-ink-4" : "cursor-pointer text-ink-3 hover:text-ink"}`}
                    aria-label="Next day"
                  >
                    →
                  </button>
                </div>

                <h1 className="font-sans text-[56px] leading-none font-[500] tracking-[-0.03em] text-ink">
                  {dayName}
                </h1>

                <div className="mt-2 mb-6 font-mono text-[13px] text-ink-3">
                  {dateStr} · {String(dayEntryCount)} {dayEntryCount === 1 ? "entry" : "entries"}
                </div>
              </>
            ) : (
              <>
                <h1 className="mb-2 font-sans text-[48px] leading-none font-[500] tracking-[-0.03em] text-ink">
                  {rangeModeLabel(rangeMode)}
                </h1>
                <div className="mb-6 font-mono text-[13px] text-ink-3">
                  {String(rangeEntryCount)} {rangeEntryCount === 1 ? "entry" : "entries"}
                </div>
              </>
            )}

            {/* Kind filter */}
            <FilterBar
              allEntries={isRangeMode ? rangeAllEntries : dayAllEntries}
              active={activeFilter}
              onChange={setActiveFilter}
            />

            <div className="border-t border-line" />

            {!isRangeMode && dayEntries.length === 0 && (
              <div className="py-16 text-center text-[15px] text-ink-4 italic">{dayEmptyText}</div>
            )}
            {!isRangeMode && dayEntries.length > 0 && (
              <div className="divide-y divide-line">
                {dayEntries.map((entry) => (
                  <EntryRow
                    key={entry.id}
                    entry={entry}
                    isConfirming={confirmingId === entry.id}
                    onClickNote={(id) => {
                      onNavigateNotes(id);
                    }}
                    onClickMeeting={(id) => {
                      onNavigateMeeting(id);
                    }}
                    onDeleteRequest={() => {
                      setConfirmingId(entry.id);
                    }}
                    onDeleteConfirm={() => {
                      handleConfirmDelete(entry);
                    }}
                    onDeleteCancel={() => {
                      setConfirmingId(null);
                    }}
                  />
                ))}
              </div>
            )}
            {isRangeMode && flatItems.length === 0 && (
              <div className="py-16 text-center text-[15px] text-ink-4 italic">
                {rangeEmptyText}
              </div>
            )}
            {isRangeMode && flatItems.length > 0 && (
              <div
                style={{ height: `${String(virtualizer.getTotalSize())}px`, position: "relative" }}
              >
                {virtualizer.getVirtualItems().map((vItem) => {
                  const item = flatItems[vItem.index];
                  return (
                    <div
                      key={vItem.key}
                      ref={virtualizer.measureElement}
                      data-index={vItem.index}
                      style={{
                        position: "absolute",
                        top: 0,
                        left: 0,
                        width: "100%",
                        transform: `translateY(${String(vItem.start)}px)`,
                      }}
                    >
                      {item !== undefined &&
                        (item.type === "day-header" ? (
                          <DayHeader label={item.label} />
                        ) : (
                          <EntryRow
                            entry={item.entry}
                            isConfirming={confirmingId === item.entry.id}
                            onClickNote={(id) => {
                              onNavigateNotes(id);
                            }}
                            onClickMeeting={(id) => {
                              onNavigateMeeting(id);
                            }}
                            onDeleteRequest={() => {
                              setConfirmingId(item.entry.id);
                            }}
                            onDeleteConfirm={() => {
                              handleConfirmDelete(item.entry);
                            }}
                            onDeleteCancel={() => {
                              setConfirmingId(null);
                            }}
                            showBorder
                          />
                        ))}
                    </div>
                  );
                })}
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
