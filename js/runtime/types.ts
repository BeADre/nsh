// Types of nsh apps: styles, the props of host elements and the JSX
// namespace. Types only: nothing here reaches a bundle.
//
// The styles (`style.gen.ts`) and the host elements' props
// (`elements.gen.ts`) are generated from the core; the event handlers are
// written here, keep them in sync with the payloads of `nsh-js`'s
// `dispatch_event`.

import type { YNode } from "./_core";
import type * as Host from "./elements.gen";

// The style interfaces and the value types they use, generated from the
// core's table of style properties (`nsh_core::js::style`).
import type { ColorValue, ImageStyle, TextStyle, ViewStyle } from "./style.gen";
export type {
  ColorValue,
  DimensionValue,
  ImageStyle,
  LayoutStyle,
  LengthValue,
  TextStyle,
  TransformStep,
  ViewStyle,
} from "./style.gen";
// The props of the host elements and the value types they use, generated
// from the core's `Prop` (`nsh_core::js::host`).
export type {
  AccessibilityRole,
  AccessibilityState,
  AutoCapitalize,
  AutoComplete,
  EllipsizeMode,
  KeyboardDismissMode,
  KeyboardType,
  ListContentStyle,
  LiveRegion,
  ModalAnimation,
  ModalPresentation,
  PointerEvents,
  ResizeMode,
  ReturnKeyType,
  ScreenAnimation,
  SnapAlignment,
  TextContentType,
} from "./elements.gen";
export type {
  AndroidModalHostProps,
  InputHostProps,
  IOSModalHostProps,
  IOSScreenHostProps,
  IOSWebViewHostProps,
  ListHostProps,
  ModalHostProps,
  ScreenHostProps,
  VideoHostProps,
  WebViewHostProps,
} from "./elements.gen";

/** A style, or a list of styles merged left to right (falsy entries are
 * skipped). */
export type StyleProp<T> = T | readonly (T | false | null | undefined)[] | null | undefined;

export interface LayoutEvent {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface PanEvent {
  translationX: number;
  translationY: number;
  velocityX: number;
  velocityY: number;
  /** The `pan.x` value now (with one). */
  valueX?: number;
  /** The `pan.y` value now (with one). */
  valueY?: number;
}

/** A finger of a touch event (points). */
export interface TouchPoint {
  /** Identifies the finger while it is down. */
  identifier: number;
  /** Position in the view the finger went down on (`target`). */
  locationX: number;
  locationY: number;
  /** Position in the surface. */
  pageX: number;
  pageY: number;
  /** Id of the view the finger went down on. */
  target: number;
  /** Milliseconds. */
  timestamp: number;
}

/** A touch or responder event: the finger it is about, the fingers down
 * and those that changed (React Native's `nativeEvent`, which it also is). */
export interface GestureResponderEvent extends TouchPoint {
  touches: TouchPoint[];
  changedTouches: TouchPoint[];
  nativeEvent: GestureResponderEvent;
  /** Touch handlers (`onTouchStart`…): views further out do not see it. */
  stopPropagation(): void;
}

/** Raw touches and the gesture responder system (React Native semantics). */
export interface TouchProps {
  onTouchStart?: (e: GestureResponderEvent) => void;
  onTouchMove?: (e: GestureResponderEvent) => void;
  onTouchEnd?: (e: GestureResponderEvent) => void;
  onTouchCancel?: (e: GestureResponderEvent) => void;
  onTouchStartCapture?: (e: GestureResponderEvent) => void;
  onTouchMoveCapture?: (e: GestureResponderEvent) => void;
  onTouchEndCapture?: (e: GestureResponderEvent) => void;
  onTouchCancelCapture?: (e: GestureResponderEvent) => void;
  /** Become the responder when a finger goes down (asked from the target outwards). */
  onStartShouldSetResponder?: (e: GestureResponderEvent) => boolean;
  /** Become the responder when a finger moves. */
  onMoveShouldSetResponder?: (e: GestureResponderEvent) => boolean;
  /** As above, asked from the outside in, before the bubbling ones. */
  onStartShouldSetResponderCapture?: (e: GestureResponderEvent) => boolean;
  onMoveShouldSetResponderCapture?: (e: GestureResponderEvent) => boolean;
  /** This view is the responder now. Return true to keep native scroll views from taking the touches. */
  onResponderGrant?: (e: GestureResponderEvent) => boolean | void;
  /** Another view is the responder and would not let go. */
  onResponderReject?: (e: GestureResponderEvent) => void;
  onResponderStart?: (e: GestureResponderEvent) => void;
  onResponderMove?: (e: GestureResponderEvent) => void;
  onResponderEnd?: (e: GestureResponderEvent) => void;
  /** The last finger lifted. */
  onResponderRelease?: (e: GestureResponderEvent) => void;
  /** Another view (or a native gesture) took the touches. */
  onResponderTerminate?: (e: GestureResponderEvent) => void;
  /** Another view wants the touches: return false to keep them (default true). */
  onResponderTerminationRequest?: (e: GestureResponderEvent) => boolean;
}

/** Props every host element accepts. */
export interface CommonProps extends TouchProps, Host.HostProps {
  ref?: YNode | ((node: YNode) => void);
  children?: JSX.Element;
  onPress?: () => void;
  onLongPress?: () => void;
  onPressIn?: () => void;
  onPressOut?: () => void;
  onLayout?: (e: LayoutEvent) => void;
  onPanStart?: (e: PanEvent) => void;
  /** With the release velocity, e.g. to start a `decay`. */
  onPanEnd?: (e: PanEvent) => void;
  onPinchStart?: (e: { scale: number; velocity: number }) => void;
  onPinchEnd?: (e: { scale: number; velocity: number }) => void;
  onRotationStart?: (e: { rotation: number; velocity: number }) => void;
  onRotationEnd?: (e: { rotation: number; velocity: number }) => void;
  onAccessibilityAction?: (e: { actionName: string }) => void;
  /** iOS-only props (ignored elsewhere). */
  ios?: IOSProps;
  /** Android-only props (ignored elsewhere). */
  android?: AndroidProps;
  /** Desktop-only props (macOS, Windows, Linux; ignored on mobile). */
  desktop?: DesktopProps;
}

/** iOS-only props of every element (`ios={{…}}`); elements add their own. */
export interface IOSProps {}

/** Desktop-only props of every element (`desktop={{…}}`); elements add their own. */
export interface DesktopProps {}

/** Android-only props of every element (`android={{…}}`). */
export interface AndroidProps extends Host.AndroidHostProps {}

/** `<view>` */
export interface ViewProps extends CommonProps {
  style?: StyleProp<ViewStyle>;
}

/** `<text>`: nested `<text>` elements are spans of the same paragraph. */
export interface TextLayoutLine {
  text: string;
  x: number;
  y: number;
  width: number;
  height: number;
  ascender: number;
  descender: number;
  capHeight: number;
  xHeight: number;
}

export interface TextProps extends CommonProps, Host.TextHostProps {
  style?: StyleProp<TextStyle>;
  /** The laid-out lines (in the text's content box), after each layout. */
  onTextLayout?: (e: { lines: TextLayoutLine[] }) => void;
}

/** An image asset (`import logo from "./logo.png"`), sized in points. */
export interface ImageAsset {
  uri: string;
  width: number;
  height: number;
}

/** `<image>` */
export interface ImageProps extends CommonProps, Host.ImageHostProps {
  style?: StyleProp<ImageStyle>;
  onLoadStart?: () => void;
  /** Download progress in bytes (`total` 0 when unknown); at most every 100 ms. */
  onProgress?: (e: { loaded: number; total: number }) => void;
  /** The source image's size in pixels (not the decoded, downsampled one). */
  onLoad?: (e: { width: number; height: number }) => void;
  onError?: (e: { message: string }) => void;
  /** After `onLoad` or `onError`. */
  onLoadEnd?: () => void;
}

/** Where a scroll view is: the payload of `onScroll`. */
export interface ScrollEvent {
  x: number;
  y: number;
  /** The content's size (zoomed). */
  contentWidth: number;
  contentHeight: number;
  width: number;
  height: number;
  /** The zoom scale (1 unless the scroll view zooms). */
  zoomScale: number;
}

/** How scroll views and lists scroll (React Native's ScrollView props). */
export interface ScrollBehaviorProps extends Host.ScrollingHostProps {
  onScroll?: (e: ScrollEvent) => void;
  onScrollBeginDrag?: (e: { x: number; y: number }) => void;
  onScrollEndDrag?: (e: { x: number; y: number }) => void;
  onMomentumScrollBegin?: (e: { x: number; y: number }) => void;
  onMomentumScrollEnd?: (e: { x: number; y: number }) => void;
}

/** `<scroll>` */
export interface ScrollProps extends CommonProps, ScrollBehaviorProps, Host.ScrollHostProps {
  style?: StyleProp<ViewStyle>;
  onRefresh?: () => void;
  onScrollEnd?: (e: { x: number; y: number }) => void;
}

/** `<input>` */
export interface InputProps extends CommonProps, Host.InputHostProps {
  style?: StyleProp<TextStyle>;
  onChangeText?: (text: string) => void;
  onSubmitEditing?: (e: { text: string }) => void;
  onFocus?: () => void;
  onBlur?: () => void;
  onSelectionChange?: (e: { start: number; end: number }) => void;
  /** Before the text changes: `key` is the typed text, `Backspace` or `Enter`. */
  onKeyPress?: (e: { key: string }) => void;
  /** The size of the text (at the input's width when multiline): grow a multiline input with it. */
  onContentSizeChange?: (e: { width: number; height: number }) => void;
  ios?: IOSInputProps;
}

/** iOS-only props of `<input>`. */
export interface IOSInputProps extends IOSProps, Host.IOSInputHostProps {}

/** Host elements without components of their own take any prop the core
 * knows; their typed forms are the runtime's components (`WebView`, …). */
interface ControlProps extends CommonProps {
  style?: StyleProp<ViewStyle>;
  [prop: string]: any;
}

export declare namespace JSX {
  /** What components return and children accept. */
  type Element = YNode | ArrayElement | (() => Element) | string | number | boolean | null | undefined;
  interface ArrayElement extends Array<Element> {}
  interface ElementChildrenAttribute {
    children: {};
  }
  interface IntrinsicElements {
    view: ViewProps;
    text: TextProps;
    image: ImageProps;
    scroll: ScrollProps;
    input: InputProps;
    inputaccessory: ControlProps;
    canvas: ControlProps;
    webview: ControlProps;
    video: ControlProps;
    stack: ControlProps;
    screen: ControlProps;
    modal: ControlProps;
    headeritem: ControlProps;
  }
  // Used by solid-js's own types.
  interface HTMLAttributes<T> {}
}

/** The state of a pan gesture (React Native's `PanResponderGestureState`). */
export interface PanResponderGestureState {
  stateID: number;
  /** Latest position of the touches' centroid (surface points). */
  moveX: number;
  moveY: number;
  /** Where the centroid was when the responder was granted. */
  x0: number;
  y0: number;
  /** Distance moved since the grant. */
  dx: number;
  dy: number;
  /** Velocity (points per millisecond). */
  vx: number;
  vy: number;
  numberActiveTouches: number;
}

type PanHandler<R = void> = (e: GestureResponderEvent, gestureState: PanResponderGestureState) => R;

/** `PanResponder.create` callbacks. */
export interface PanResponderCallbacks {
  onStartShouldSetPanResponder?: PanHandler<boolean>;
  onStartShouldSetPanResponderCapture?: PanHandler<boolean>;
  onMoveShouldSetPanResponder?: PanHandler<boolean>;
  onMoveShouldSetPanResponderCapture?: PanHandler<boolean>;
  onPanResponderGrant?: PanHandler;
  onPanResponderReject?: PanHandler;
  onPanResponderStart?: PanHandler;
  onPanResponderMove?: PanHandler;
  onPanResponderEnd?: PanHandler;
  onPanResponderRelease?: PanHandler;
  onPanResponderTerminate?: PanHandler;
  onPanResponderTerminationRequest?: PanHandler<boolean>;
  /** Keep native scroll views from taking the touches (default true). */
  onShouldBlockNativeResponder?: PanHandler<boolean>;
}

/** What `PanResponder.create` returns: spread `panHandlers` on a view. */
export interface PanResponderInstance {
  panHandlers: TouchProps;
}
