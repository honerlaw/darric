import type { Session } from "../types";

export function formatElapsed(seconds: number): string {
  const m = Math.floor(seconds / 60)
    .toString()
    .padStart(2, "0");
  const s = (seconds % 60).toString().padStart(2, "0");
  return `${m}:${s}`;
}

export function formatTime(iso: string): string {
  return new Date(iso).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function groupSessionsByDate(sessions: Session[]): Record<string, Session[]> {
  const today = new Date().toDateString();
  const yesterday = new Date(Date.now() - 86400000).toDateString();
  const groups: Record<string, Session[]> = {};

  for (const s of sessions) {
    const d = new Date(s.started_at).toDateString();
    let label: string;
    if (d === today) {
      label = "Today";
    } else if (d === yesterday) {
      label = "Yesterday";
    } else {
      label = d;
    }
    (groups[label] ??= []).push(s);
  }
  return groups;
}

/**
 * What to call a recording in prose. One answer, used by every label site — an
 * empty-string topic and a null one must not name the same recording differently.
 */
export function sessionLabel(session: Session): string {
  return session.topic !== null && session.topic.length > 0 ? session.topic : "Untitled recording";
}
