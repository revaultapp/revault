export function scheduleStartupUpdateCheck(
  isDesktop: boolean,
  requestFrame: (callback: FrameRequestCallback) => number,
  cancelFrame: (handle: number) => void,
  check: () => void,
): () => void {
  if (!isDesktop) return () => {};
  const handle = requestFrame(() => check());
  return () => cancelFrame(handle);
}
