import type React from "react";

interface ModelDownloadBannerProps {
  /** Percentage downloaded, or null when no download is in flight. */
  progress: number | null;
}

/**
 * App-scoped, because the download is app-scoped. This previously lived inside
 * `RecorderPane`, which returns a placeholder before reaching its body whenever
 * no recording is selected — so on a fresh install, the only situation the
 * indicator exists for, it could never render.
 */
export function ModelDownloadBanner({
  progress,
}: ModelDownloadBannerProps): React.JSX.Element | null {
  if (progress === null) return null;

  return (
    <div className="shrink-0 border-b border-line bg-paper-sunken px-6 py-3">
      <div className="flex items-center justify-between gap-4">
        <p className="text-[13px] text-ink-2">
          Downloading speech model — recording starts once this finishes.
        </p>
        <span className="font-mono text-[11px] text-ink-3">{String(progress)}%</span>
      </div>
      <div
        className="mt-2 h-1.5 w-full overflow-hidden rounded-full bg-paper"
        role="progressbar"
        aria-label="Speech model download"
        aria-valuenow={progress}
        aria-valuemin={0}
        aria-valuemax={100}
      >
        <div
          className="h-full rounded-full bg-accent transition-all duration-300"
          style={{ width: `${String(progress)}%` }}
        />
      </div>
    </div>
  );
}
