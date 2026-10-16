import { type YNode } from "./_core";
import { NativeNode } from "./animated";
import { type CanvasContextMethods, type CanvasNumber, type CanvasPathMethods, type CanvasProperties } from "./canvas.gen";
import type { ImageAsset, ViewProps } from "./types";
export type { CanvasContextMethods, CanvasNumber, CanvasPathMethods, CanvasProperties } from "./canvas.gen";
/** A color, a gradient, or a native value interpolating colors. */
export type CanvasPaint = string | CanvasGradient | NativeNode;
/** Path building, shared by the context (its current path) and `Path2D`. */
export interface CanvasPath extends CanvasPathMethods {}
export declare abstract class CanvasPath {
	/** Where the operations go. */
	protected abstract out: unknown[];
	/** `radii`: one radius, or 2–4 as in CSS `border-radius` (top-left first). */
	roundRect(x: CanvasNumber, y: CanvasNumber, w: CanvasNumber, h: CanvasNumber, radii?: CanvasNumber | CanvasNumber[]): void;
}
/** A path to fill, stroke or clip; `new Path2D("M0 0 L10 10")` takes SVG
* path data (parsed natively). */
export declare class Path2D extends CanvasPath {
	/** The recorded operations. */
	readonly ops: unknown[];
	protected get out(): unknown[];
	constructor(path?: Path2D | string);
	/** Appends `path`'s subpaths. */
	addPath(path: Path2D): void;
}
/** A gradient from `createLinearGradient` / `createRadialGradient` /
* `createConicGradient`. */
export declare class CanvasGradient {
	/** `[kind, ...geometry, stops]`: what the drawing carries. */
	readonly spec: unknown[];
	private readonly stops;
	constructor(kind: number, geometry: number[]);
	/** `offset` 0..1; any CSS color. */
	addColorStop(offset: number, color: string): void;
}
export interface TextMetrics {
	/** The advance width in points. */
	width: number;
}
/**
* The 2D context `<Canvas draw>` gets (an HTML Canvas 2D subset). Calls only
* record operations; the platform draws them after `draw` returns (and again
* whenever a native value they read changes). Coordinates are in points,
* the origin at the canvas's top left.
*/
export interface CanvasRenderingContext2D extends CanvasContextMethods, CanvasProperties {}
export declare class CanvasRenderingContext2D extends CanvasPath {
	protected out: unknown[];
	/** The canvas's size in points. */
	readonly canvas: {
		readonly width: number;
		readonly height: number;
	};
	private s;
	private stack;
	constructor(out: unknown[], size: {
		width: number;
		height: number;
	});
	/** Ends recording: later calls (e.g. from a timer) are ignored. */
	close(): void;
	private set;
	get fillStyle(): CanvasPaint;
	set fillStyle(v: CanvasPaint);
	get strokeStyle(): CanvasPaint;
	set strokeStyle(v: CanvasPaint);
	get shadowOffsetX(): number;
	set shadowOffsetX(v: number);
	get shadowOffsetY(): number;
	set shadowOffsetY(v: number);
	save(): void;
	restore(): void;
	resetTransform(): void;
	setLineDash(segments: number[]): void;
	getLineDash(): number[];
	fill(path?: Path2D | "nonzero" | "evenodd", rule?: "nonzero" | "evenodd"): void;
	stroke(path?: Path2D): void;
	clip(path?: Path2D | "nonzero" | "evenodd", rule?: "nonzero" | "evenodd"): void;
	fillText(text: string, x: CanvasNumber, y: CanvasNumber, maxWidth?: number): void;
	strokeText(text: string, x: CanvasNumber, y: CanvasNumber, maxWidth?: number): void;
	/** Measured by the platform's text engine with the current `font`. */
	measureText(text: string): TextMetrics;
	createLinearGradient(x0: number, y0: number, x1: number, y1: number): CanvasGradient;
	createRadialGradient(x0: number, y0: number, r0: number, x1: number, y1: number, r1: number): CanvasGradient;
	/** `startAngle` in radians, clockwise from the x axis. */
	createConicGradient(startAngle: number, x: number, y: number): CanvasGradient;
	/**
	* Draws an image (a URL or an imported asset), loaded natively:
	* `(image, dx, dy)` at its natural size, `(image, dx, dy, dw, dh)`, or
	* `(image, sx, sy, sw, sh, dx, dy, dw, dh)` for part of it (source in
	* image pixels). The canvas draws again once it has loaded.
	*/
	drawImage(image: string | ImageAsset | {
		uri: string;
	}, ...args: CanvasNumber[]): void;
}
export interface CanvasSize {
	width: number;
	height: number;
}
export interface CanvasProps extends ViewProps {
	/**
	* Draws the canvas. Runs again when signals it reads change and when the
	* canvas resizes (it first runs once the size is known); native values
	* passed as numbers or colors animate without running it.
	*/
	draw: (ctx: CanvasRenderingContext2D, size: CanvasSize) => void;
}
/**
* A view drawn with a 2D context (HTML Canvas 2D API). `draw` records the
* drawing in one go; the platform replays it natively (CGContext, Android
* Canvas), at the screen's resolution.
*/
export declare function Canvas(props: CanvasProps): YNode;
