import { useSyncExternalStore } from "react";

/** Set once the player solves a puzzle; unlocks the profile button and the résumé. */
const KEY = "patches-resume-unlocked";

const listeners = new Set<() => void>();
// Also kept in memory, so a browser that refuses storage still unlocks for this visit.
let unlocked = false;

function isUnlocked(): boolean {
  if (unlocked) return true;
  try {
    return localStorage.getItem(KEY) === "1";
  } catch {
    return false;
  }
}

export function unlockResume() {
  if (isUnlocked()) return;
  unlocked = true;
  try {
    localStorage.setItem(KEY, "1");
  } catch {}
  listeners.forEach((l) => l());
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** Whether the résumé is unlocked; null while prerendering, when nobody knows yet. */
export function useResumeUnlocked(): boolean | null {
  return useSyncExternalStore(subscribe, isUnlocked, () => null);
}
