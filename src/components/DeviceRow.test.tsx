import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { DeviceRow } from "./DeviceRow";
import type { CaptureDevice } from "../types";

function device(over: Partial<CaptureDevice> = {}): CaptureDevice {
  return {
    id: "mic-1",
    name: "Built-in Microphone",
    direction: "input",
    enabled: true,
    state: "active",
    level: 0.5,
    ...over,
  };
}

describe("DeviceRow", () => {
  it("says so when there are no devices rather than rendering nothing", () => {
    render(<DeviceRow devices={[]} onToggle={() => undefined} />);
    expect(screen.getByText(/no capture devices/i)).toBeInTheDocument();
  });

  it("exposes each device as a labelled switch reflecting its enabled state", () => {
    render(
      <DeviceRow
        devices={[device(), device({ id: "mic-2", name: "Rode", enabled: false })]}
        onToggle={() => undefined}
      />,
    );
    expect(screen.getByRole("switch", { name: /Built-in Microphone/ })).toBeChecked();
    expect(screen.getByRole("switch", { name: /Rode/ })).not.toBeChecked();
  });

  it("reports the toggled device and its new state", async () => {
    const user = userEvent.setup();
    const onToggle = vi.fn();
    render(<DeviceRow devices={[device()]} onToggle={onToggle} />);

    await user.click(screen.getByRole("switch", { name: /Built-in Microphone/ }));
    expect(onToggle).toHaveBeenCalledWith("mic-1", false);
  });

  it("re-enables a disabled device", async () => {
    const user = userEvent.setup();
    const onToggle = vi.fn();
    render(<DeviceRow devices={[device({ enabled: false })]} onToggle={onToggle} />);

    await user.click(screen.getByRole("switch", { name: /Built-in Microphone/ }));
    expect(onToggle).toHaveBeenCalledWith("mic-1", true);
  });

  it("surfaces a device that is retrying rather than hiding it", () => {
    // A recording that silently lost a microphone looks exactly like one where
    // nobody spoke, so this state has to be visible.
    render(<DeviceRow devices={[device({ state: "retrying" })]} onToggle={() => undefined} />);
    expect(screen.getByText("retrying")).toBeInTheDocument();
  });

  it("shows a device the recorder gave up on, and says why", () => {
    // After a minute of failed rebuilds the backend stops retrying. That must
    // not look like a healthy idle device.
    const { container } = render(
      <DeviceRow devices={[device({ state: "failed" })]} onToggle={() => undefined} />,
    );
    expect(screen.getByText("failed")).toBeInTheDocument();
    expect(container.querySelector('[title*="stopped retrying"]')).not.toBeNull();
  });

  it("does not show a live meter for a device that is not capturing", () => {
    const { container } = render(
      <DeviceRow devices={[device({ enabled: false, level: 0.9 })]} onToggle={() => undefined} />,
    );
    const meter = container.querySelector<HTMLElement>('[style*="width"]');
    expect(meter?.style.width).toBe("0%");
  });
});
