import type React from "react";
import { useEffect, useRef } from "react";

interface ConfirmDialogProps {
  title: string;
  /** The consequence, stated plainly — this is the last thing shown before an irreversible action. */
  body: string;
  confirmLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
}

/**
 * A modal confirmation for a destructive action. The confirm button carries the
 * danger colour because every current caller is destroying something; if a
 * non-destructive caller appears, that styling is what needs a prop, not the
 * dialog's shape.
 */
export function ConfirmDialog({
  title,
  body,
  confirmLabel,
  onConfirm,
  onCancel,
}: ConfirmDialogProps): React.JSX.Element {
  const panelRef = useRef<HTMLDivElement>(null);
  const confirmRef = useRef<HTMLButtonElement>(null);

  // Focus lands on Confirm rather than Cancel so the dialog is operable from the
  // keyboard without a tab; Escape is the one-key way out, so the destructive
  // default costs nothing. The opener is restored on close — without it a
  // keyboard user who cancels is dropped on `<body>` and has to re-traverse the
  // whole sidebar to get back to where they were.
  useEffect(() => {
    const opener = document.activeElement;
    confirmRef.current?.focus();
    return () => {
      if (opener instanceof HTMLElement) opener.focus();
    };
  }, []);

  // Bound to the document, not to the panel, and handling Tab as well as Escape.
  //
  // `aria-modal="true"` tells assistive technology the rest of the page is inert.
  // Nothing enforces that on its own: without this, three Tabs from Confirm reach
  // the Record button behind the backdrop, where Enter starts a recording under a
  // modal the user cannot see past. The promise has to be kept, not just made.
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent): void => {
      if (e.key === "Escape") {
        onCancel();
        return;
      }
      if (e.key !== "Tab") return;

      const panel = panelRef.current;
      if (panel === null) return;
      // Every focusable thing in this dialog is a button; if that stops being
      // true, this selector is what has to grow.
      const focusable = Array.from(panel.querySelectorAll("button"));
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (first === undefined || last === undefined) return;

      const active = document.activeElement;
      if (!panel.contains(active)) {
        e.preventDefault();
        first.focus();
      } else if (e.shiftKey && active === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && active === last) {
        e.preventDefault();
        first.focus();
      }
    };
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [onCancel]);

  return (
    // Fixed, so the sidebar's `overflow-y-auto` cannot clip or scroll it away.
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-ink/30 px-6"
      // Dismissal is keyed to where the press *started*, not to where the click
      // resolves. A click's target is the common ancestor of its press and its
      // release, so selecting the dialog's own body text and releasing over the
      // dim area reports the backdrop — and a click-based backdrop closes the
      // dialog mid-selection.
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onCancel();
      }}
    >
      <div
        ref={panelRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="confirm-dialog-title"
        aria-describedby="confirm-dialog-body"
        className="w-full max-w-[360px] rounded-[10px] border border-line bg-paper p-5 shadow-lg"
      >
        <h3
          id="confirm-dialog-title"
          className="font-sans text-[15px] font-[500] tracking-[-0.01em] text-ink"
        >
          {title}
        </h3>
        <p id="confirm-dialog-body" className="mt-2 text-[13px] leading-[1.5] text-ink-3">
          {body}
        </p>

        <div className="mt-5 flex justify-end gap-2">
          <button
            type="button"
            onClick={onCancel}
            className="cursor-pointer rounded-full border border-line bg-paper px-[14px] py-[5px] text-[13px] text-ink transition-colors hover:border-line-strong hover:bg-paper-sunken"
          >
            Cancel
          </button>
          <button
            ref={confirmRef}
            type="button"
            onClick={onConfirm}
            className="cursor-pointer rounded-full bg-danger px-[14px] py-[5px] text-[13px] font-[500] text-paper transition-opacity hover:opacity-90"
          >
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
