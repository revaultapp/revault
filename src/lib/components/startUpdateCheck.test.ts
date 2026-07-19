import { describe, expect, it, vi } from "vitest";
import { scheduleStartupUpdateCheck } from "./startUpdateCheck";

describe("startup update check", () => {
  it("waits for the next frame before checking in Tauri", () => {
    const check = vi.fn();
    let frame: FrameRequestCallback | undefined;

    const cancel = scheduleStartupUpdateCheck(true, (callback) => {
      frame = callback;
      return 7;
    }, vi.fn(), check);

    expect(check).not.toHaveBeenCalled();
    frame?.(0);
    expect(check).toHaveBeenCalledOnce();
    cancel();
  });

  it("does not schedule a check outside Tauri", () => {
    const requestFrame = vi.fn();
    scheduleStartupUpdateCheck(false, requestFrame, vi.fn(), vi.fn());
    expect(requestFrame).not.toHaveBeenCalled();
  });
});
