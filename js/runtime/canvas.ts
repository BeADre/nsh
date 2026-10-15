// `nsh/canvas`: a view drawn with an HTML Canvas 2D context, replayed
// natively. The methods and properties that only record an operation are
// generated (`canvas.gen.ts`, from `nsh_core::js::canvas`); the rest is here.

import { createRenderEffect, createSignal, mergeProps as mergeSolidProps, splitProps } from "solid-js";
import { createElement, ops, schedule, spread, type YNode } from "./_core";
import { NativeNode } from "./animated";
import {
  type CanvasContextMethods,
  canvasContextMethods,
  type CanvasNumber,
  type CanvasPathMethods,
  canvasPathMethods,
  type CanvasProperties,
  canvasProperties,
  initialCanvasProperties,
} from "./canvas.gen";
import {
  CV_CLIP,
  CV_DRAW_IMAGE,
  CV_FILL,
  CV_FILL_STYLE,
  CV_FILL_TEXT,
  CV_LINE_DASH,
  CV_RESTORE,
  CV_ROUND_RECT,
  CV_SAVE,
  CV_SET_TRANSFORM,
  CV_SHADOW_OFFSET,
  CV_STROKE,
  CV_STROKE_STYLE,
  CV_STROKE_TEXT,
  CV_SVG_PATH,
  OP_DRAW,
} from "./protocol.gen";
import type { ImageAsset, LayoutEvent, ViewProps } from "./types";

export type { CanvasContextMethods, CanvasNumber, CanvasPathMethods, CanvasProperties } from "./canvas.gen";


declare function __nsh_measure_text(font: string, text: string): number;

/** A color, a gradient, or a native value interpolating colors. */
export type CanvasPaint = string | CanvasGradient | NativeNode;

/** Path building, shared by the context (its current path) and `Path2D`. */
export interface CanvasPath extends CanvasPathMethods {}
export abstract class CanvasPath {
  /** Where the operations go. */
  protected abstract out: unknown[];

  /** `radii`: one radius, or 2–4 as in CSS `border-radius` (top-left first). */
  roundRect(
    x: CanvasNumber,
    y: CanvasNumber,
    w: CanvasNumber,
    h: CanvasNumber,
    radii: CanvasNumber | CanvasNumber[] = 0,
  ): void {
    const r = Array.isArray(radii) ? radii : [radii];
    const [tl, tr, br, bl] =
      r.length >= 4 ? r : r.length === 3 ? [r[0], r[1], r[2], r[1]] : r.length === 2 ? [r[0], r[1], r[0], r[1]] : [r[0] ?? 0, r[0] ?? 0, r[0] ?? 0, r[0] ?? 0];
    this.out.push(CV_ROUND_RECT, x, y, w, h, tl, tr, br, bl);
  }
}
Object.assign(CanvasPath.prototype, canvasPathMethods);

/** A path to fill, stroke or clip; `new Path2D("M0 0 L10 10")` takes SVG
 * path data (parsed natively). */
export class Path2D extends CanvasPath {
  /** The recorded operations. */
  readonly ops: unknown[] = [];
  protected get out(): unknown[] {
    return this.ops;
  }

  constructor(path?: Path2D | string) {
    super();
    if (typeof path === "string") this.ops.push(CV_SVG_PATH, path);
    else if (path) this.ops.push(...path.ops);
  }

  /** Appends `path`'s subpaths. */
  addPath(path: Path2D): void {
    this.ops.push(...path.ops);
  }
}

/** A gradient from `createLinearGradient` / `createRadialGradient` /
 * `createConicGradient`. */
export class CanvasGradient {
  /** `[kind, ...geometry, stops]`: what the drawing carries. */
  readonly spec: unknown[];
  private readonly stops: unknown[] = [];

  constructor(kind: number, geometry: number[]) {
    this.spec = [kind, ...geometry, this.stops];
  }

  /** `offset` 0..1; any CSS color. */
  addColorStop(offset: number, color: string): void {
    this.stops.push(offset, color);
  }
}

export interface TextMetrics {
  /** The advance width in points. */
  width: number;
}

/** The drawing state: the generated properties' and those written here. */
interface CanvasState extends CanvasProperties {
  fillStyle: CanvasPaint;
  strokeStyle: CanvasPaint;
  lineDash: number[];
  shadowOffsetX: number;
  shadowOffsetY: number;
}

const initialCanvasState: CanvasState = {
  ...initialCanvasProperties,
  fillStyle: "#000",
  strokeStyle: "#000",
  lineDash: [],
  shadowOffsetX: 0,
  shadowOffsetY: 0,
};

const paintValue = (p: CanvasPaint): unknown => (p instanceof CanvasGradient ? p.spec : p);

/**
 * The 2D context `<Canvas draw>` gets (an HTML Canvas 2D subset). Calls only
 * record operations; the platform draws them after `draw` returns (and again
 * whenever a native value they read changes). Coordinates are in points,
 * the origin at the canvas's top left.
 */
export interface CanvasRenderingContext2D extends CanvasContextMethods, CanvasProperties {}
export class CanvasRenderingContext2D extends CanvasPath {
  protected out: unknown[];
  /** The canvas's size in points. */
  readonly canvas: { readonly width: number; readonly height: number };
  private s: CanvasState = { ...initialCanvasState };
  private stack: CanvasState[] = [];

  constructor(out: unknown[], size: { width: number; height: number }) {
    super();
    this.out = out;
    this.canvas = size;
  }

  /** Ends recording: later calls (e.g. from a timer) are ignored. */
  close(): void {
    this.out = [];
  }

  private set<K extends keyof CanvasState>(key: K, value: CanvasState[K], op: number, encoded: unknown): void {
    this.s[key] = value;
    this.out.push(op, encoded);
  }

  get fillStyle(): CanvasPaint {
    return this.s.fillStyle;
  }
  set fillStyle(v: CanvasPaint) {
    this.set("fillStyle", v, CV_FILL_STYLE, paintValue(v));
  }
  get strokeStyle(): CanvasPaint {
    return this.s.strokeStyle;
  }
  set strokeStyle(v: CanvasPaint) {
    this.set("strokeStyle", v, CV_STROKE_STYLE, paintValue(v));
  }
  get shadowOffsetX(): number {
    return this.s.shadowOffsetX;
  }
  set shadowOffsetX(v: number) {
    this.s.shadowOffsetX = v;
    this.out.push(CV_SHADOW_OFFSET, v, this.s.shadowOffsetY);
  }
  get shadowOffsetY(): number {
    return this.s.shadowOffsetY;
  }
  set shadowOffsetY(v: number) {
    this.s.shadowOffsetY = v;
    this.out.push(CV_SHADOW_OFFSET, this.s.shadowOffsetX, v);
  }

  save(): void {
    this.stack.push({ ...this.s });
    this.out.push(CV_SAVE);
  }
  restore(): void {
    const s = this.stack.pop();
    if (!s) return;
    this.s = s;
    this.out.push(CV_RESTORE);
  }
  resetTransform(): void {
    this.out.push(CV_SET_TRANSFORM, 1, 0, 0, 1, 0, 0);
  }
  setLineDash(segments: number[]): void {
    this.s.lineDash = [...segments];
    this.out.push(CV_LINE_DASH, this.s.lineDash);
  }
  getLineDash(): number[] {
    const d = this.s.lineDash;
    return d.length % 2 ? [...d, ...d] : [...d];
  }

  fill(path?: Path2D | "nonzero" | "evenodd", rule?: "nonzero" | "evenodd"): void {
    if (path instanceof Path2D) this.out.push(CV_FILL, path.ops, rule ?? "nonzero");
    else this.out.push(CV_FILL, null, path ?? "nonzero");
  }
  stroke(path?: Path2D): void {
    this.out.push(CV_STROKE, path ? path.ops : null);
  }
  clip(path?: Path2D | "nonzero" | "evenodd", rule?: "nonzero" | "evenodd"): void {
    if (path instanceof Path2D) this.out.push(CV_CLIP, path.ops, rule ?? "nonzero");
    else this.out.push(CV_CLIP, null, path ?? "nonzero");
  }
  fillText(text: string, x: CanvasNumber, y: CanvasNumber, maxWidth?: number): void {
    this.out.push(CV_FILL_TEXT, String(text), x, y, maxWidth ?? null);
  }
  strokeText(text: string, x: CanvasNumber, y: CanvasNumber, maxWidth?: number): void {
    this.out.push(CV_STROKE_TEXT, String(text), x, y, maxWidth ?? null);
  }
  /** Measured by the platform's text engine with the current `font`. */
  measureText(text: string): TextMetrics {
    return { width: __nsh_measure_text(this.s.font, String(text)) };
  }

  createLinearGradient(x0: number, y0: number, x1: number, y1: number): CanvasGradient {
    return new CanvasGradient(0, [x0, y0, x1, y1]);
  }
  createRadialGradient(x0: number, y0: number, r0: number, x1: number, y1: number, r1: number): CanvasGradient {
    return new CanvasGradient(1, [x0, y0, r0, x1, y1, r1]);
  }
  /** `startAngle` in radians, clockwise from the x axis. */
  createConicGradient(startAngle: number, x: number, y: number): CanvasGradient {
    return new CanvasGradient(2, [startAngle, x, y]);
  }

  /**
   * Draws an image (a URL or an imported asset), loaded natively:
   * `(image, dx, dy)` at its natural size, `(image, dx, dy, dw, dh)`, or
   * `(image, sx, sy, sw, sh, dx, dy, dw, dh)` for part of it (source in
   * image pixels). The canvas draws again once it has loaded.
   */
  drawImage(image: string | ImageAsset | { uri: string }, ...args: CanvasNumber[]): void {
    const uri = typeof image === "string" ? image : image.uri;
    const asset = typeof image === "string" ? null : (image as Partial<ImageAsset>);
    if (args.length >= 8) {
      this.out.push(CV_DRAW_IMAGE, uri, args.slice(0, 4), args[4], args[5], args[6], args[7]);
    } else if (args.length >= 4) {
      this.out.push(CV_DRAW_IMAGE, uri, null, args[0], args[1], args[2], args[3]);
    } else {
      this.out.push(CV_DRAW_IMAGE, uri, null, args[0] ?? 0, args[1] ?? 0, asset?.width ?? NaN, asset?.height ?? NaN);
    }
  }
}
Object.assign(CanvasRenderingContext2D.prototype, canvasContextMethods);
Object.defineProperties(CanvasRenderingContext2D.prototype, canvasProperties);

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
export function Canvas(props: CanvasProps): YNode {
  const [local, others] = splitProps(props, ["draw", "onLayout"]);
  const node = createElement("canvas");
  const [size, setSize] = createSignal<CanvasSize | null>(null, {
    equals: (a, b) => a !== null && b !== null && a.width === b.width && a.height === b.height,
  });
  spread(
    node,
    mergeSolidProps(others, {
      onLayout(e: LayoutEvent) {
        setSize({ width: e.width, height: e.height });
        local.onLayout?.(e);
      },
    }),
    false,
  );
  createRenderEffect(() => {
    const s = size();
    if (!s) return;
    // Its own list: native values made while drawing push ops of their own.
    const out: unknown[] = [];
    const ctx = new CanvasRenderingContext2D(out, s);
    try {
      local.draw(ctx, s);
    } finally {
      ctx.close();
      ops.push(OP_DRAW, node.id, out);
      // The native values it reads live as long as the view.
      const used: NativeNode[] = [];
      for (const v of out) {
        if (v instanceof NativeNode) used.push(v);
        else if (Array.isArray(v)) for (const w of v) if (w instanceof NativeNode) used.push(w);
      }
      node.anims = { ...node.anims, draw: used };
      schedule();
    }
  });
  return node;
}
