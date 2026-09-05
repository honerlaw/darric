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
  const confirmRef = useRef<HTMLButtonElement>(null);

  // Focus lands on Confirm rather than Cancel so the dialog is operable from the
  // keyboard without a tab; Escape is the one-key way out, so the destructive
  // default costs nothing.
  useEffect(() => {
    confirmRef.current?.focus();
  }, []);

  // Bound to the document, not to the dialog: the click that opened this came
  // from a button that keeps focus until the effect above moves it, and a
  // keydown handler on the panel would miss an Escape pressed in that gap.
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent): void => {
      if (e.key === "Escape") onCancel();
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
      onClick={onCancel}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="confirm-dialog-title"
        // Without this a click anywhere inside the panel bubbles to the backdrop
        // above and cancels the dialog the user is still reading.
        onClick={(e) => {
          e.stopPropagation();
        }}
        className="w-full max-w-[360px] rounded-[10px] border border-line bg-paper p-5 shadow-lg"
      >
        <h3
          id="confirm-dialog-title"
          className="font-sans text-[15px] font-[500] tracking-[-0.01em] text-ink"
        >
          {title}
        </h3>
        <p className="mt-2 text-[13px] leading-[1.5] text-ink-3">{body}</p>

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
