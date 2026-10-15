// `nsh/accessibility`: screen readers and motion settings. The screen
// reader's commands are Rust (`nsh_core::runtime`); the settings are the
// environment's signals, read at once.

import { type Accessor, createRenderEffect, createRoot, untrack } from "solid-js";
import { reduceMotion, screenReaderEnabled } from "./_core";
import { AccessibilityInfo as NativeAccessibilityInfo } from "./accessibility.gen";

export { reduceMotion, screenReaderEnabled } from "./_core";

export type AccessibilityChangeEvent = "screenReaderChanged" | "reduceMotionChanged";

const accessibilityListeners: Record<AccessibilityChangeEvent, Set<(enabled: boolean) => void>> = {
  screenReaderChanged: new Set(),
  reduceMotionChanged: new Set(),
};
createRoot(() => {
  const watch = (name: AccessibilityChangeEvent, value: Accessor<boolean>) => {
    let last = untrack(value);
    createRenderEffect(() => {
      const now = value();
      if (now === last) return;
      last = now;
      for (const listener of [...accessibilityListeners[name]]) listener(now);
    });
  };
  watch("screenReaderChanged", screenReaderEnabled);
  watch("reduceMotionChanged", reduceMotion);
});

/** The settings, at once, and their changes. */
export interface AccessibilitySettings {
  isScreenReaderEnabled(): Promise<boolean>;
  isReduceMotionEnabled(): Promise<boolean>;
  addEventListener(event: AccessibilityChangeEvent, listener: (enabled: boolean) => void): { remove(): void };
}

/** Screen reader and motion settings. `screenReaderEnabled()` and
 * `reduceMotion()` are the reactive forms. */
export const AccessibilityInfo: typeof NativeAccessibilityInfo & AccessibilitySettings = Object.assign(NativeAccessibilityInfo, {
  isScreenReaderEnabled(): Promise<boolean> {
    return Promise.resolve(untrack(screenReaderEnabled));
  },
  isReduceMotionEnabled(): Promise<boolean> {
    return Promise.resolve(untrack(reduceMotion));
  },
  addEventListener(event: AccessibilityChangeEvent, listener: (enabled: boolean) => void): { remove(): void } {
    accessibilityListeners[event].add(listener);
    return { remove: () => accessibilityListeners[event].delete(listener) };
  },
});
