import type { SessionTarget, Snapshot } from "./types";

export function terminalKey(target: SessionTarget) {
  return target.terminalId ? `${target.source}:${target.terminalId}` : null;
}

export function isMonitored(
  snapshot: Snapshot,
  target: SessionTarget,
): boolean {
  const key = terminalKey(target) ?? snapshot.sessionTerminals[target.id];
  return (
    snapshot.sessionMonitoring[target.id] ??
    (key ? snapshot.terminalMonitoring[key] : undefined) ??
    true
  );
}

export function changeMonitoring(
  snapshot: Snapshot,
  target: SessionTarget,
  enabled: boolean,
  wholeTerminal: boolean,
): Snapshot {
  const sessionMonitoring = { ...snapshot.sessionMonitoring };
  const terminalMonitoring = { ...snapshot.terminalMonitoring };
  const key = terminalKey(target);
  if (wholeTerminal && key) {
    terminalMonitoring[key] = enabled;
    for (const [id, owner] of Object.entries(snapshot.sessionTerminals)) {
      if (owner === key) delete sessionMonitoring[id];
    }
    for (const sibling of [
      ...snapshot.orcaSessions,
      ...snapshot.otherSessions,
    ]) {
      if (terminalKey(sibling) === key) delete sessionMonitoring[sibling.id];
    }
  } else sessionMonitoring[target.id] = enabled;
  return { ...snapshot, sessionMonitoring, terminalMonitoring };
}
