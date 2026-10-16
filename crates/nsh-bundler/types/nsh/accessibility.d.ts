import { AccessibilityInfo as NativeAccessibilityInfo } from "./accessibility.gen";
export { reduceMotion, screenReaderEnabled } from "./_core";
export type AccessibilityChangeEvent = "screenReaderChanged" | "reduceMotionChanged";
/** The settings, at once, and their changes. */
export interface AccessibilitySettings {
	isScreenReaderEnabled(): Promise<boolean>;
	isReduceMotionEnabled(): Promise<boolean>;
	addEventListener(event: AccessibilityChangeEvent, listener: (enabled: boolean) => void): {
		remove(): void;
	};
}
/** Screen reader and motion settings. `screenReaderEnabled()` and
* `reduceMotion()` are the reactive forms. */
export declare const AccessibilityInfo: typeof NativeAccessibilityInfo & AccessibilitySettings;
