import type React from "react";
import { useEffect, useRef, useState } from "react";
import { Dock } from "./components/layout/Dock";
import { Header } from "./components/layout/Header";
import { SearchBar } from "./components/layout/SearchBar";
import { NoteModal } from "./components/notes/NoteModal";
import { SettingsModal } from "./components/settings/SettingsModal";
import { useConversation } from "./hooks/useConversation";
import { useNotes } from "./hooks/useNotes";
import { useSession } from "./hooks/useSession";
import { useTags } from "./hooks/useTags";
import { useTasks } from "./hooks/useTasks";
import { useTranscript } from "./hooks/useTranscript";
import { updateSessionNotes } from "./lib/tauri";
import { BoardScreen } from "./screens/BoardScreen";
import { MeetingScreen } from "./screens/MeetingScreen";
import { TimelineScreen } from "./screens/TimelineScreen";
import type { Note, Screen } from "./types";

export default function App(): React.JSX.Element {
  const [screen, setScreen] = useState<Screen>("timeline");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [searchOpen, setSearchOpen] = useState(false);
  // Store the full Note object so the modal opens immediately without
  // depending on the notes list re-render completing first.
  const [openNote, setOpenNote] = useState<Note | null>(null);
  const openNoteIdRef = useRef<string | null>(null);

  const [viewingSessionId, setViewingSessionId] = useState<string | null>(null);

  const {
    sessions,
    activeSessionId,
    isRecording,
    isStarting,
    elapsedSeconds,
    start,
    stop,
    resume,
    update,
    remove: removeSession,
    refresh: refreshSessions,
  } = useSession();

  const transcriptLines = useTranscript(
    viewingSessionId,
    isRecording && viewingSessionId === activeSessionId,
  );
  const {
    notes,
    create: createNote,
    update: updateNote,
    remove: deleteNote,
    refresh: refreshNotes,
  } = useNotes();
  const {
    tasks,
    create: createTask,
    move: moveTask,
    rename: renameTask,
    remove: removeTask,
    refresh: refreshTasks,
  } = useTasks();

  const {
    allTags,
    addToSession: addTagToSession,
    removeFromSession: removeTagFromSession,
    addToNote: addTagToNote,
    removeFromNote: removeTagFromNote,
    addToTask: addTagToTask,
    removeFromTask: removeTagFromTask,
  } = useTags({
    onRefreshSessions: refreshSessions,
    onRefreshNotes: refreshNotes,
    onRefreshTasks: refreshTasks,
  });
  const { agentEntries, submit: submitPrompt } = useConversation();

  // Keep openNote in sync when the notes list refreshes (e.g. after autosave).
  // If the note is no longer in the list (deleted externally), close the modal.
  useEffect(() => {
    if (openNoteIdRef.current === null) return;
    const updated = notes.find((n) => n.id === openNoteIdRef.current);
    if (updated !== undefined) {
      setOpenNote(updated);
    } else {
      openNoteIdRef.current = null;
      setOpenNote(null);
    }
  }, [notes]);

  useEffect(() => {
    const handler = (e: KeyboardEvent): void => {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        setSearchOpen((v) => !v);
      }
    };
    window.addEventListener("keydown", handler);
    return () => {
      window.removeEventListener("keydown", handler);
    };
  }, []);

  useEffect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = (dark: boolean): void => {
      document.documentElement.classList.toggle("dark", dark);
    };
    apply(mq.matches);
    const handler = (e: MediaQueryListEvent): void => {
      apply(e.matches);
    };
    mq.addEventListener("change", handler);
    return () => {
      mq.removeEventListener("change", handler);
    };
  }, []);

  useEffect(() => {
    if (isRecording && activeSessionId !== null) setViewingSessionId(activeSessionId);
  }, [isRecording, activeSessionId]);

  const handleRecordClick = (): void => {
    if (isRecording) {
      setViewingSessionId(activeSessionId);
      setScreen("meeting");
      return;
    }
    void start("Meeting")
      .then(() => {
        setScreen("meeting");
      })
      .catch((err: unknown) => {
        console.error("start recording failed:", err);
      });
  };

  const handleStop = (): void => {
    void stop()
      .then(() => {
        setScreen(viewingSessionId !== null ? "meeting" : "timeline");
      })
      .catch((err: unknown) => {
        console.error("stop recording failed:", err);
      });
  };

  const handleNavigateMeeting = (id?: string): void => {
    const target = id ?? activeSessionId;
    if (target === null) return;
    setViewingSessionId(target);
    setScreen("meeting");
  };

  const handleResume = (): void => {
    if (viewingSessionId === null) return;
    void resume(viewingSessionId);
  };

  const handleUpdateTitle = (newTitle: string): void => {
    if (viewingSessionId === null) return;
    const trimmed = newTitle.trim();
    void update(viewingSessionId, trimmed !== "" ? trimmed : undefined);
  };

  const handleSaveNotes = (newNotes: string): void => {
    if (viewingSessionId === null) return;
    void updateSessionNotes(viewingSessionId, newNotes)
      .then(() => {
        void refreshSessions();
      })
      .catch((err: unknown) => {
        console.error("save notes failed:", err);
      });
  };

  const handleDockSubmit = (value: string): void => {
    submitPrompt(value);
    if (screen !== "timeline") setScreen("timeline");
  };

  const openNoteById = (id: string): void => {
    openNoteIdRef.current = id;
    const found = notes.find((n) => n.id === id) ?? null;
    setOpenNote(found);
  };

  const handleNewNote = (): void => {
    void createNote("", "")
      .then((note) => {
        openNoteIdRef.current = note.id;
        setOpenNote(note);
      })
      .catch((err: unknown) => {
        console.error("create note failed:", err);
      });
  };

  const handleCloseNote = (): void => {
    openNoteIdRef.current = null;
    setOpenNote(null);
  };

  const viewingSession = sessions.find((s) => s.id === viewingSessionId) ?? null;

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-paper font-sans text-ink">
      <Header
        activeScreen={screen}
        isRecording={isRecording}
        elapsedSeconds={elapsedSeconds}
        onNavigate={(s) => {
          setScreen(s);
        }}
        onRecordClick={handleRecordClick}
        onNewNote={handleNewNote}
        onOpenSettings={() => {
          setSettingsOpen(true);
        }}
        onOpenSearch={() => {
          setSearchOpen(true);
        }}
      />

      <div className="border-t border-line" />

      <div className="flex flex-1 overflow-hidden">
        {screen === "timeline" && (
          <TimelineScreen
            sessions={sessions}
            notes={notes}
            tasks={tasks}
            agentEntries={agentEntries}
            allTags={allTags}
            onNavigateNotes={openNoteById}
            onNavigateMeeting={handleNavigateMeeting}
            onDeleteNote={deleteNote}
            onDeleteMeeting={removeSession}
          />
        )}

        {screen === "meeting" && (
          <MeetingScreen
            sessionId={viewingSessionId}
            sessionTopic={viewingSession?.topic ?? null}
            sessionStartedAt={viewingSession?.started_at ?? null}
            sessionNotes={viewingSession?.notes ?? ""}
            sessionTags={viewingSession?.tags ?? []}
            allTags={allTags}
            transcriptLines={transcriptLines}
            isRecording={isRecording}
            elapsedSeconds={elapsedSeconds}
            canResume={!isRecording}
            onStop={handleStop}
            onResume={handleResume}
            onUpdateTitle={handleUpdateTitle}
            onSaveNotes={handleSaveNotes}
            onAddTag={(name) => {
              if (viewingSessionId !== null) void addTagToSession(viewingSessionId, name);
            }}
            onRemoveTag={(tagId) => {
              if (viewingSessionId !== null) void removeTagFromSession(viewingSessionId, tagId);
            }}
          />
        )}

        {screen === "board" && (
          <BoardScreen
            tasks={tasks}
            allTags={allTags}
            onCreate={createTask}
            onMove={moveTask}
            onRename={renameTask}
            onRemove={removeTask}
            onAddTag={addTagToTask}
            onRemoveTag={removeTagFromTask}
          />
        )}
      </div>

      {isStarting && (
        <div className="flex shrink-0 items-center justify-center bg-accent-tint py-2">
          <span className="font-mono text-[11px] text-accent">Loading whisper model…</span>
        </div>
      )}

      <Dock
        activeScreen={screen}
        onSubmit={handleDockSubmit}
        onOpenSettings={() => {
          setSettingsOpen(true);
        }}
      />

      {openNote !== null && (
        <NoteModal
          note={openNote}
          allTags={allTags}
          onUpdate={updateNote}
          onDelete={deleteNote}
          onClose={handleCloseNote}
          onAddTag={(noteId, name) => {
            void addTagToNote(noteId, name);
          }}
          onRemoveTag={(noteId, tagId) => {
            void removeTagFromNote(noteId, tagId);
          }}
        />
      )}

      <SettingsModal
        open={settingsOpen}
        onClose={() => {
          setSettingsOpen(false);
        }}
      />

      {searchOpen && (
        <SearchBar
          onClose={() => {
            setSearchOpen(false);
          }}
          onNavigateMeeting={(id) => {
            setSearchOpen(false);
            handleNavigateMeeting(id);
          }}
          onNavigateNote={(id) => {
            setSearchOpen(false);
            openNoteById(id);
          }}
          onNavigateBoard={() => {
            setSearchOpen(false);
            setScreen("board");
          }}
        />
      )}
    </div>
  );
}
