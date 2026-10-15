// `nsh/ios`: what only iOS has. Importable everywhere (one bundle runs on
// both platforms); elsewhere it does nothing.

import { hostComponent, os, type YNode } from "./_core";
import type { JSX, StyleProp, ViewStyle } from "./types";

/** True on iOS (and Mac Catalyst). */
export const isAvailable: boolean = os === "ios";

// --- Input accessories ------------------------------------------------------------

export interface InputAccessoryViewProps {
  /** Inputs with this `inputAccessoryViewID` show the view over the keyboard. */
  nativeID: string;
  style?: StyleProp<ViewStyle>;
  class?: string;
  children?: JSX.Element;
}

/**
 * A bar over the keyboard of the inputs whose `inputAccessoryViewID` is its
 * `nativeID` (iOS's `inputAccessoryView`): as wide as the window and as tall
 * as its content, moving with the keyboard natively. It renders nothing where
 * it is placed in the tree, and nothing at all on Android.
 */
export const InputAccessoryView: (props: InputAccessoryViewProps) => YNode | null = isAvailable
  ? hostComponent<InputAccessoryViewProps>("inputaccessory", true)
  : () => null;
