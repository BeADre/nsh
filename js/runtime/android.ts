// `nsh/android`: what only Android has. Importable everywhere (one bundle
// runs on both platforms); elsewhere it does nothing. `Toast`,
// `setStatusBarBackgroundColor` and `BackHandler.exitApp` are Rust
// (`nsh_core::runtime`); the back button's listeners are the app's JS.

import { os, root, setProp } from "./_core";
import { BackHandler as NativeBackHandler } from "./android.gen";

export * from "./android.gen";

/** True on Android. */
export const isAvailable: boolean = os === "android";

const backListeners: (() => boolean | void)[] = [];

function onBackPress() {
  for (let i = backListeners.length - 1; i >= 0; i--) {
    if (backListeners[i]()) return;
  }
  NativeBackHandler.exitApp();
}

/** The back button's listeners. */
export interface BackListeners {
  /** `listener` returns true when it handled the press; the most recently
   * added listener runs first. Without a listener that handles it, the app
   * exits. */
  addEventListener(event: "hardwareBackPress", listener: () => boolean | void): { remove(): void };
}

/** The back button, after modals and stacks had their turn. */
export const BackHandler: typeof NativeBackHandler & BackListeners = Object.assign(NativeBackHandler, {
  addEventListener(_event: "hardwareBackPress", listener: () => boolean | void): { remove(): void } {
    backListeners.push(listener);
    if (backListeners.length === 1) setProp(root, "onBackPress", onBackPress);
    return {
      remove(): void {
        const i = backListeners.indexOf(listener);
        if (i >= 0) backListeners.splice(i, 1);
        if (backListeners.length === 0) setProp(root, "onBackPress", undefined);
      },
    };
  },
});
