// Globals of the nsh JS environment (QuickJS-NG plus nsh's own): what an app
// can use besides the ECMAScript library. There is no DOM.
//
// Keep in sync with `crates/nsh-js/src/console.rs`, `host.rs` (timers,
// frames) and `web/` (fetch and friends).

declare var console: {
  log(...data: unknown[]): void;
  info(...data: unknown[]): void;
  debug(...data: unknown[]): void;
  warn(...data: unknown[]): void;
  error(...data: unknown[]): void;
};

declare var performance: {
  /** Milliseconds since the JS context started, with sub-millisecond precision. */
  now(): number;
};

declare function setTimeout(handler: (...args: any[]) => void, timeout?: number, ...args: any[]): number;
declare function setInterval(handler: (...args: any[]) => void, timeout?: number, ...args: any[]): number;
declare function clearTimeout(id: number | undefined): void;
declare function clearInterval(id: number | undefined): void;
declare function queueMicrotask(callback: () => void): void;
/** Runs `callback` at the next display frame with its time in milliseconds. */
declare function requestAnimationFrame(callback: (time: number) => void): number;
declare function cancelAnimationFrame(id: number): void;

// --- Web APIs (`crates/nsh-js/src/web`): as in browsers, for TS apps ----

declare function atob(data: string): string;
declare function btoa(data: string): string;

declare class DOMException extends Error {
  constructor(message?: string, name?: string);
  readonly name: string;
  readonly message: string;
  readonly code: number;
}

type BufferSource = ArrayBuffer | ArrayBufferView;

declare class TextEncoder {
  readonly encoding: "utf-8";
  encode(input?: string): Uint8Array;
  encodeInto(source: string, destination: Uint8Array): { read: number; written: number };
}

/** UTF-8, UTF-16LE / BE and windows-1252 (with the labels of latin1 and ASCII). */
declare class TextDecoder {
  constructor(label?: string, options?: { fatal?: boolean; ignoreBOM?: boolean });
  readonly encoding: string;
  readonly fatal: boolean;
  readonly ignoreBOM: boolean;
  decode(input?: BufferSource, options?: { stream?: boolean }): string;
}

declare class TextEncoderStream {
  readonly encoding: "utf-8";
  readonly readable: ReadableStream<Uint8Array>;
  readonly writable: WritableStream<string>;
}

declare class TextDecoderStream {
  constructor(label?: string, options?: { fatal?: boolean; ignoreBOM?: boolean });
  readonly encoding: string;
  readonly fatal: boolean;
  readonly ignoreBOM: boolean;
  readonly readable: ReadableStream<string>;
  readonly writable: WritableStream<BufferSource>;
}

declare class URLSearchParams {
  constructor(init?: string | Record<string, string> | Iterable<[string, string]> | URLSearchParams);
  readonly size: number;
  append(name: string, value: string): void;
  delete(name: string, value?: string): void;
  get(name: string): string | null;
  getAll(name: string): string[];
  has(name: string, value?: string): boolean;
  set(name: string, value: string): void;
  sort(): void;
  forEach(callback: (value: string, key: string, parent: URLSearchParams) => void, thisArg?: unknown): void;
  entries(): IterableIterator<[string, string]>;
  keys(): IterableIterator<string>;
  values(): IterableIterator<string>;
  [Symbol.iterator](): IterableIterator<[string, string]>;
  toString(): string;
}

declare class URL {
  constructor(url: string | URL, base?: string | URL);
  static canParse(url: string | URL, base?: string | URL): boolean;
  static parse(url: string | URL, base?: string | URL): URL | null;
  /** A `blob:` URL `fetch` reads `blob` from, until revoked. */
  static createObjectURL(blob: Blob): string;
  static revokeObjectURL(url: string): void;
  href: string;
  readonly origin: string;
  protocol: string;
  username: string;
  password: string;
  host: string;
  hostname: string;
  port: string;
  pathname: string;
  search: string;
  hash: string;
  readonly searchParams: URLSearchParams;
  toString(): string;
  toJSON(): string;
}

interface EventInit {
  bubbles?: boolean;
  cancelable?: boolean;
  composed?: boolean;
}

declare class Event {
  constructor(type: string, init?: EventInit);
  static readonly NONE: 0;
  static readonly CAPTURING_PHASE: 1;
  static readonly AT_TARGET: 2;
  static readonly BUBBLING_PHASE: 3;
  readonly type: string;
  readonly target: EventTarget | null;
  readonly currentTarget: EventTarget | null;
  readonly eventPhase: number;
  readonly bubbles: boolean;
  readonly cancelable: boolean;
  readonly defaultPrevented: boolean;
  readonly composed: boolean;
  readonly isTrusted: boolean;
  readonly timeStamp: number;
  returnValue: boolean;
  cancelBubble: boolean;
  composedPath(): EventTarget[];
  preventDefault(): void;
  stopPropagation(): void;
  stopImmediatePropagation(): void;
}

declare class CustomEvent<T = any> extends Event {
  constructor(type: string, init?: EventInit & { detail?: T });
  readonly detail: T;
}

type EventListenerOrEventListenerObject = ((event: Event) => void) | { handleEvent(event: Event): void };

interface AddEventListenerOptions {
  capture?: boolean;
  once?: boolean;
  passive?: boolean;
  signal?: AbortSignal;
}

declare class EventTarget {
  constructor();
  addEventListener(
    type: string,
    listener: EventListenerOrEventListenerObject | null,
    options?: AddEventListenerOptions | boolean,
  ): void;
  removeEventListener(
    type: string,
    listener: EventListenerOrEventListenerObject | null,
    options?: { capture?: boolean } | boolean,
  ): void;
  dispatchEvent(event: Event): boolean;
}

declare class AbortSignal extends EventTarget {
  readonly aborted: boolean;
  readonly reason: any;
  onabort: ((event: Event) => void) | null;
  throwIfAborted(): void;
  static abort(reason?: any): AbortSignal;
  static timeout(milliseconds: number): AbortSignal;
  static any(signals: Iterable<AbortSignal>): AbortSignal;
}

declare class AbortController {
  readonly signal: AbortSignal;
  abort(reason?: any): void;
}

type BlobPart = BufferSource | Blob | string;

interface BlobPropertyBag {
  type?: string;
  endings?: "transparent" | "native";
}

declare class Blob {
  constructor(blobParts?: Iterable<BlobPart>, options?: BlobPropertyBag);
  readonly size: number;
  readonly type: string;
  slice(start?: number, end?: number, contentType?: string): Blob;
  arrayBuffer(): Promise<ArrayBuffer>;
  bytes(): Promise<Uint8Array>;
  text(): Promise<string>;
  stream(): ReadableStream<Uint8Array>;
}

declare class File extends Blob {
  constructor(fileBits: Iterable<BlobPart>, fileName: string, options?: BlobPropertyBag & { lastModified?: number });
  readonly name: string;
  readonly lastModified: number;
  readonly webkitRelativePath: string;
}

type FormDataEntryValue = File | string;

declare class FormData {
  constructor();
  append(name: string, value: string | Blob, fileName?: string): void;
  delete(name: string): void;
  get(name: string): FormDataEntryValue | null;
  getAll(name: string): FormDataEntryValue[];
  has(name: string): boolean;
  set(name: string, value: string | Blob, fileName?: string): void;
  forEach(callback: (value: FormDataEntryValue, key: string, parent: FormData) => void, thisArg?: unknown): void;
  entries(): IterableIterator<[string, FormDataEntryValue]>;
  keys(): IterableIterator<string>;
  values(): IterableIterator<FormDataEntryValue>;
  [Symbol.iterator](): IterableIterator<[string, FormDataEntryValue]>;
}

// Streams.

interface QueuingStrategy<T = any> {
  highWaterMark?: number;
  size?: (chunk: T) => number;
}

declare class ByteLengthQueuingStrategy implements QueuingStrategy<ArrayBufferView> {
  constructor(init: { highWaterMark: number });
  readonly highWaterMark: number;
  readonly size: (chunk: ArrayBufferView) => number;
}

declare class CountQueuingStrategy implements QueuingStrategy {
  constructor(init: { highWaterMark: number });
  readonly highWaterMark: number;
  readonly size: (chunk: any) => 1;
}

interface UnderlyingDefaultSource<R = any> {
  start?(controller: ReadableStreamDefaultController<R>): any;
  pull?(controller: ReadableStreamDefaultController<R>): void | PromiseLike<void>;
  cancel?(reason?: any): void | PromiseLike<void>;
  type?: undefined;
}

interface UnderlyingByteSource {
  type: "bytes";
  autoAllocateChunkSize?: number;
  start?(controller: ReadableByteStreamController): any;
  pull?(controller: ReadableByteStreamController): void | PromiseLike<void>;
  cancel?(reason?: any): void | PromiseLike<void>;
}

type ReadableStreamReadResult<T> = { done: false; value: T } | { done: true; value?: T };

interface StreamPipeOptions {
  preventAbort?: boolean;
  preventCancel?: boolean;
  preventClose?: boolean;
  signal?: AbortSignal;
}

declare class ReadableStream<R = any> {
  constructor(underlyingSource: UnderlyingByteSource, strategy?: { highWaterMark?: number });
  constructor(underlyingSource?: UnderlyingDefaultSource<R>, strategy?: QueuingStrategy<R>);
  static from<R>(iterable: Iterable<R | PromiseLike<R>> | AsyncIterable<R>): ReadableStream<R>;
  readonly locked: boolean;
  cancel(reason?: any): Promise<void>;
  getReader(options: { mode: "byob" }): ReadableStreamBYOBReader;
  getReader(): ReadableStreamDefaultReader<R>;
  pipeThrough<T>(transform: { writable: WritableStream<R>; readable: ReadableStream<T> }, options?: StreamPipeOptions): ReadableStream<T>;
  pipeTo(destination: WritableStream<R>, options?: StreamPipeOptions): Promise<void>;
  tee(): [ReadableStream<R>, ReadableStream<R>];
  values(options?: { preventCancel?: boolean }): AsyncIterableIterator<R>;
  [Symbol.asyncIterator](options?: { preventCancel?: boolean }): AsyncIterableIterator<R>;
}

declare class ReadableStreamDefaultReader<R = any> {
  constructor(stream: ReadableStream<R>);
  readonly closed: Promise<void>;
  read(): Promise<ReadableStreamReadResult<R>>;
  releaseLock(): void;
  cancel(reason?: any): Promise<void>;
}

declare class ReadableStreamBYOBReader {
  constructor(stream: ReadableStream<Uint8Array>);
  readonly closed: Promise<void>;
  read<T extends ArrayBufferView>(view: T, options?: { min?: number }): Promise<ReadableStreamReadResult<T>>;
  releaseLock(): void;
  cancel(reason?: any): Promise<void>;
}

declare class ReadableStreamDefaultController<R = any> {
  readonly desiredSize: number | null;
  close(): void;
  enqueue(chunk?: R): void;
  error(e?: any): void;
}

declare class ReadableByteStreamController {
  readonly byobRequest: ReadableStreamBYOBRequest | null;
  readonly desiredSize: number | null;
  close(): void;
  enqueue(chunk: ArrayBufferView): void;
  error(e?: any): void;
}

declare class ReadableStreamBYOBRequest {
  readonly view: ArrayBufferView | null;
  respond(bytesWritten: number): void;
  respondWithNewView(view: ArrayBufferView): void;
}

interface UnderlyingSink<W = any> {
  start?(controller: WritableStreamDefaultController): any;
  write?(chunk: W, controller: WritableStreamDefaultController): void | PromiseLike<void>;
  close?(): void | PromiseLike<void>;
  abort?(reason?: any): void | PromiseLike<void>;
  type?: undefined;
}

declare class WritableStream<W = any> {
  constructor(underlyingSink?: UnderlyingSink<W>, strategy?: QueuingStrategy<W>);
  readonly locked: boolean;
  abort(reason?: any): Promise<void>;
  close(): Promise<void>;
  getWriter(): WritableStreamDefaultWriter<W>;
}

declare class WritableStreamDefaultWriter<W = any> {
  constructor(stream: WritableStream<W>);
  readonly closed: Promise<void>;
  readonly desiredSize: number | null;
  readonly ready: Promise<void>;
  abort(reason?: any): Promise<void>;
  close(): Promise<void>;
  releaseLock(): void;
  write(chunk?: W): Promise<void>;
}

declare class WritableStreamDefaultController {
  readonly signal: AbortSignal;
  error(e?: any): void;
}

interface Transformer<I = any, O = any> {
  start?(controller: TransformStreamDefaultController<O>): any;
  transform?(chunk: I, controller: TransformStreamDefaultController<O>): void | PromiseLike<void>;
  flush?(controller: TransformStreamDefaultController<O>): void | PromiseLike<void>;
  cancel?(reason?: any): void | PromiseLike<void>;
}

declare class TransformStream<I = any, O = any> {
  constructor(transformer?: Transformer<I, O>, writableStrategy?: QueuingStrategy<I>, readableStrategy?: QueuingStrategy<O>);
  readonly readable: ReadableStream<O>;
  readonly writable: WritableStream<I>;
}

declare class TransformStreamDefaultController<O = any> {
  readonly desiredSize: number | null;
  enqueue(chunk?: O): void;
  error(reason?: any): void;
  terminate(): void;
}

// fetch.

type HeadersInit = Headers | Record<string, string> | Iterable<[string, string]>;

declare class Headers {
  constructor(init?: HeadersInit);
  append(name: string, value: string): void;
  delete(name: string): void;
  get(name: string): string | null;
  getSetCookie(): string[];
  has(name: string): boolean;
  set(name: string, value: string): void;
  forEach(callback: (value: string, key: string, parent: Headers) => void, thisArg?: unknown): void;
  entries(): IterableIterator<[string, string]>;
  keys(): IterableIterator<string>;
  values(): IterableIterator<string>;
  [Symbol.iterator](): IterableIterator<[string, string]>;
}

type BodyInit = ReadableStream<Uint8Array> | Blob | BufferSource | FormData | URLSearchParams | string;

interface Body {
  /** The body as it arrives (a byte stream), or null. */
  readonly body: ReadableStream<Uint8Array> | null;
  readonly bodyUsed: boolean;
  arrayBuffer(): Promise<ArrayBuffer>;
  blob(): Promise<Blob>;
  bytes(): Promise<Uint8Array>;
  formData(): Promise<FormData>;
  json(): Promise<any>;
  text(): Promise<string>;
}

interface RequestInit {
  method?: string;
  headers?: HeadersInit;
  /** A ReadableStream is sent as it is read (chunked), with `duplex: "half"`. */
  body?: BodyInit | null;
  duplex?: "half";
  signal?: AbortSignal | null;
  /** "manual" gives the 3xx response itself. */
  redirect?: "follow" | "manual" | "error";
  credentials?: "omit" | "same-origin" | "include";
  cache?: "default" | "no-store" | "reload" | "no-cache" | "force-cache" | "only-if-cached";
  mode?: "same-origin" | "no-cors" | "cors";
  referrer?: string;
  referrerPolicy?: string;
  integrity?: string;
  keepalive?: boolean;
  priority?: "high" | "low" | "auto";
}

declare class Request implements Body {
  constructor(input: string | URL | Request, init?: RequestInit);
  readonly method: string;
  readonly url: string;
  readonly headers: Headers;
  readonly destination: string;
  readonly referrer: string;
  readonly referrerPolicy: string;
  readonly mode: string;
  readonly credentials: string;
  readonly cache: string;
  readonly redirect: "follow" | "manual" | "error";
  readonly integrity: string;
  readonly keepalive: boolean;
  readonly isReloadNavigation: boolean;
  readonly isHistoryNavigation: boolean;
  readonly signal: AbortSignal;
  readonly duplex: "half";
  readonly body: ReadableStream<Uint8Array> | null;
  readonly bodyUsed: boolean;
  arrayBuffer(): Promise<ArrayBuffer>;
  blob(): Promise<Blob>;
  bytes(): Promise<Uint8Array>;
  formData(): Promise<FormData>;
  json(): Promise<any>;
  text(): Promise<string>;
  clone(): Request;
}

interface ResponseInit {
  status?: number;
  statusText?: string;
  headers?: HeadersInit;
}

declare class Response implements Body {
  constructor(body?: BodyInit | null, init?: ResponseInit);
  static json(data: unknown, init?: ResponseInit): Response;
  static error(): Response;
  static redirect(url: string | URL, status?: number): Response;
  readonly type: "basic" | "default" | "error";
  readonly url: string;
  readonly redirected: boolean;
  readonly status: number;
  readonly ok: boolean;
  readonly statusText: string;
  readonly headers: Headers;
  readonly body: ReadableStream<Uint8Array> | null;
  readonly bodyUsed: boolean;
  arrayBuffer(): Promise<ArrayBuffer>;
  blob(): Promise<Blob>;
  bytes(): Promise<Uint8Array>;
  formData(): Promise<FormData>;
  json(): Promise<any>;
  text(): Promise<string>;
  clone(): Response;
}

/**
 * HTTP through the platform's network stack (NSURLSession,
 * HttpURLConnection), streaming both ways; `data:` and `blob:` URLs are
 * answered locally.
 */
declare function fetch(input: string | URL | Request, init?: RequestInit): Promise<Response>;

// Imported files: images are asset descriptors (sized in points, `@2x`
// files at half their pixel size), fonts the name of their family.
// The app's Tailwind theme from `nsh.json` (JSON), for the runtime.
declare module "nsh:tailwind-theme" {
  const theme: string | null;
  export default theme;
}
// The theme of the Rust components from `nsh.json` (JSON), for the runtime.
declare module "nsh:theme" {
  const theme: string | null;
  export default theme;
}
declare module "*.png" {
  const asset: { uri: string; width: number; height: number };
  export default asset;
}
declare module "*.jpg" {
  const asset: { uri: string; width: number; height: number };
  export default asset;
}
declare module "*.jpeg" {
  const asset: { uri: string; width: number; height: number };
  export default asset;
}
declare module "*.gif" {
  const asset: { uri: string; width: number; height: number };
  export default asset;
}
declare module "*.webp" {
  const asset: { uri: string; width: number; height: number };
  export default asset;
}
declare module "*.heic" {
  const asset: { uri: string; width: number; height: number };
  export default asset;
}
declare module "*.ttf" {
  const family: string;
  export default family;
}
declare module "*.otf" {
  const family: string;
  export default family;
}
