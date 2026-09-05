import type React from "react";
import type { CaptureDevice } from "../types";

interface DeviceRowProps {
  devices: CaptureDevice[];
}

/**
 * Phase 1 renders the fixed set of devices the backend currently captures — today
 * that is the default microphone alone. Live level meters and per-device toggles
 * arrive with the multi-device capture engine.
 */
export function DeviceRow({ devices }: DeviceRowProps): React.JSX.Element {
  return (
    <div className="flex flex-wrap items-center gap-2">
      {devices.map((d) => (
        <span
          key={d.id}
          className="flex items-center gap-[6px] rounded-full border border-line bg-paper px-[10px] py-[3px] font-mono text-[11px] text-ink-3"
        >
          <span
            className={`h-[6px] w-[6px] rounded-full ${d.enabled ? "bg-accent" : "bg-ink-4"}`}
          />
          {d.name}
          <span className="text-ink-4">{d.direction}</span>
        </span>
      ))}
    </div>
  );
}
