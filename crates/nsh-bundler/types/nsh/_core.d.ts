// What nsh's declarations share (not a module apps import).
import type { Accessor } from "./index";
import type { ViewProps } from "./types";

/** A native node: what `ref` receives. Its methods send commands to its
 * view, in order with the changes before them. */
export interface YNode {
	/** Focuses an `<input>` (shows the keyboard). */
	focus(): void;
	/** Inputs: selects `start..end` (UTF-16 units; a caret without `end`). */
	setSelection(start: number, end?: number): void;
	/** Unfocuses an `<input>`. */
	blur(): void;
	/** Calls command `name` of a registered native component
	* (`nsh/native`) with `args` (any JSON value). */
	command(name: string, args?: unknown): void;
	/** Scrolls a `<scroll>` or `List` to an offset. */
	scrollTo(options: {
		x?: number;
		y?: number;
		animated?: boolean;
	}): void;
	/**
	* A zoomable `<scroll>` (`maximumZoomScale` above `minimumZoomScale`):
	* zooms to `scale` (clamped to the limits), with the content point
	* (`x`, `y`) (unzoomed points) in the middle of the viewport, or around
	* the middle. E.g. double tap to zoom in on the tapped point.
	*/
	zoomTo(options: {
		scale: number;
		x?: number;
		y?: number;
		animated?: boolean;
	}): void;
	/** `List`: scrolls to a content offset along the list's axis. */
	scrollToOffset(options: {
		offset: number;
		animated?: boolean;
	}): void;
	/**
	* `List`: scrolls item `index` into view. `viewPosition` places it: 0 at
	* the start of the viewport (default), 0.5 in the middle, 1 at the end;
	* `viewOffset` shifts it further. Resolved natively with the current
	* (measured or estimated) item sizes.
	*/
	scrollToIndex(options: {
		index: number;
		animated?: boolean;
		viewPosition?: number;
		viewOffset?: number;
	}): void;
	/** `List`: scrolls to the end of the content (the start of an inverted list). */
	scrollToEnd(options?: {
		animated?: boolean;
	}): void;
	/** `WebView`: reloads the page. */
	reload(): void;
	/** `WebView`: goes back in its history. */
	goBack(): void;
	/** `WebView`: goes forward in its history. */
	goForward(): void;
	/** `WebView`: stops loading. */
	stopLoading(): void;
	/** `WebView`: runs JavaScript in the page. */
	injectJavaScript(script: string): void;
	/** `WebView`: sends `data` to the page (a `message` event on `window`). */
	postMessage(data: string): void;
	/** `Video`: jumps to `seconds`. */
	seek(seconds: number): void;
}
/** What JS sets on a Rust component's host itself: the app's classes
* (appended to the component's own), its inline style, its test id, layout
* events and the ref (`ref.command(name, args)`). */
export type ComponentHostProps = Pick<ViewProps, "class" | "classList" | "style" | "testID" | "onLayout" | "ref">;
export interface Insets {
	top: number;
	right: number;
	bottom: number;
	left: number;
}
/** An operating system (`Platform.OS`). */
export type OS = "ios" | "android" | "macos" | "windows" | "linux";
/** Size of the surface in points (reactive). */
export declare const windowSize: Accessor<{
	width: number;
	height: number;
}>;
/** Insets of the surface not covered by system UI (notch, home indicator,
* title bar), in points (reactive). */
export declare const safeArea: Accessor<Insets>;
/** Height of the software keyboard over the surface (0 when hidden) and the
* duration of its animation in seconds (reactive). */
export declare const keyboard: Accessor<{
	height: number;
	duration: number;
}>;
/** The system appearance (reactive). */
export declare const colorScheme: Accessor<"light" | "dark">;
/** The user's preferred text size relative to the default (reactive). */
export declare const fontScale: Accessor<number>;
/** Whether a screen reader (VoiceOver, TalkBack) runs (reactive). */
export declare const screenReaderEnabled: Accessor<boolean>;
/** Whether the user asked for less motion (reactive; also `motion-reduce:`). */
export declare const reduceMotion: Accessor<boolean>;
/** Insets not covered by system UI or headers where this component renders:
* the surface's safe area, a modal's, or a screen's (below its header). */
export declare function useSafeAreaInsets(): Accessor<Insets>;
