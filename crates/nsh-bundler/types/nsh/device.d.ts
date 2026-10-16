import { type OS } from "./_core";
export { colorScheme, fontScale, windowSize } from "./_core";
export type { OS } from "./_core";
export interface PlatformSelectSpec<T> {
	ios?: T;
	android?: T;
	macos?: T;
	windows?: T;
	linux?: T;
	/** iOS and Android. */
	mobile?: T;
	/** macOS, Windows and Linux. */
	desktop?: T;
	/** Every platform (nsh only runs natively). */
	native?: T;
	default?: T;
}
export interface PlatformStatic {
	readonly OS: OS;
	/** A desktop OS (macOS, Windows, Linux). */
	readonly isDesktop: boolean;
	/** iOS and desktops: the version string (`"17.5"`); Android: the API level. */
	readonly Version: string | number;
	readonly isPad: boolean;
	readonly isTV: boolean;
	/** Mac Catalyst in the Mac idiom. */
	readonly isMacCatalyst: boolean;
	readonly constants: {
		readonly osVersion: string | number;
		readonly interfaceIdiom: "phone" | "pad" | "mac" | "tv" | "desktop";
	};
	/** The value for this platform: its OS (`ios`, `macos`, ...), else
	* `mobile` / `desktop`, else `native`, else `default`. */
	select<T>(spec: PlatformSelectSpec<T>): T | undefined;
}
/** The platform the app runs on (React Native's `Platform`). */
export declare const Platform: PlatformStatic;
/** Device pixels per point (React Native's `PixelRatio`). Not reactive:
* read in components that also read `windowSize()` to follow a window
* moving to another screen. */
export interface PixelRatioStatic {
	get(): number;
	getFontScale(): number;
	getPixelSizeForLayoutSize(size: number): number;
	roundToNearestPixel(size: number): number;
}
export declare const PixelRatio: PixelRatioStatic;
/** Right-to-left layout (React Native's `I18nManager`). Right to left,
* rows run from the right and `left` / `right` style properties mean start
* and end. Changes apply at once (no restart). */
export declare const I18nManager: {
	/** Whether the app lays out right to left (reactive). */
	isRTL: boolean;
	/** `left` and `right` swap right to left (always, as React Native's default). */
	doLeftAndRightSwapInRTL: true;
	/** Follow right-to-left languages (default true). */
	allowRTL(allow: boolean): void;
	/** Lay out right to left whatever the language. */
	forceRTL(force: boolean): void;
	getConstants(): {
		isRTL: boolean;
		doLeftAndRightSwapInRTL: boolean;
	};
};
