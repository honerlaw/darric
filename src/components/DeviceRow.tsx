import type React from "react";
import type { CaptureDevice, DeviceState } from "../types";

/** Meter fill is clamped and gamma-adjusted so quiet speech is still visible. */
function meterWidth(level: number): string {
  const clamped = Math.min(Math.max(level, 0), 1);
  return `${String(Math.round(Math.sqrt(clamped) * 100))}%`;
}

const STATE_TITLE: Record<DeviceState, string> = {
  idle: "Not recording",
  starting: "Starting…",
  active: "Capturing",
  retrying: "Device unavailable — retrying",
  failed: "Stopped",
};

function stateDot(state: DeviceState, enabled: boolean): string {
  if (!enabled) return "bg-ink-4";
  if (state === "active") return "bg-accent";
  if (state === "retrying") return "bg-danger";
  return "bg-ink-4";
}

interface DeviceRowProps {
  devices: CaptureDevice[];
  onToggle: (id: string, enabled: boolean) => void;
}

/**
 * One row per capture device: a toggle, a live level meter, and its state.
 *
 * Everything discovered is captured by default, so these switches are
 * exceptions rather than selections — which is also why a device that is
 * retrying is shown rather than hidden. A recording that silently lost a
 * microphone looks identical to one where nobody spoke.
 */
export function DeviceRow({ devices, onToggle }: DeviceRowProps): React.JSX.Element {
  if (devices.length === 0) {
    return <p className="font-mono text-[11px] text-ink-4">No capture devices found.</p>;
  }

  return (
    <div className="flex flex-col gap-1.5">
      {devices.map((d) => (
        <div key={d.id} className="flex items-center gap-3">
          <button
            type="button"
            role="switch"
            aria-checked={d.enabled}
            aria-label={`Capture ${d.name}`}
            onClick={() => {
              onToggle(d.id, !d.enabled);
            }}
            className={`relative h-[16px] w-[28px] shrink-0 cursor-pointer rounded-full transition-colors ${
              d.enabled ? "bg-accent" : "bg-line-strong"
            }`}
          >
            <span
              className={`absolute top-[2px] h-[12px] w-[12px] rounded-full bg-paper transition-all ${
                d.enabled ? "left-[14px]" : "left-[2px]"
              }`}
            />
          </button>

          <span
            className={`h-[6px] w-[6px] shrink-0 rounded-full ${stateDot(d.state, d.enabled)} ${
              d.state === "retrying" ? "pulse-dot" : ""
            }`}
            title={STATE_TITLE[d.state]}
          />

          <span
            className={`w-[170px] shrink-0 truncate text-[12px] ${
              d.enabled ? "text-ink-2" : "text-ink-4 line-through"
            }`}
            title={d.name}
          >
            {d.name}
          </span>

          <div className="h-[4px] flex-1 overflow-hidden rounded-full bg-paper-sunken">
            <div
              className="h-full rounded-full bg-accent transition-[width] duration-150"
              style={{ width: d.enabled && d.state === "active" ? meterWidth(d.level) : "0%" }}
            />
          </div>

          {d.state === "retrying" && (
            <span className="shrink-0 font-mono text-[10px] text-danger">retrying</span>
          )}
        </div>
      ))}
    </div>
  );
}
