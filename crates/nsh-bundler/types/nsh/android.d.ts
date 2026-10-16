import { BackHandler as NativeBackHandler } from "./android.gen";
export * from "./android.gen";
/** True on Android. */
export declare const isAvailable: boolean;
/** The back button's listeners. */
export interface BackListeners {
	/** `listener` returns true when it handled the press; the most recently
	* added listener runs first. Without a listener that handles it, the app
	* exits. */
	addEventListener(event: "hardwareBackPress", listener: () => boolean | void): {
		remove(): void;
	};
}
/** The back button, after modals and stacks had their turn. */
export declare const BackHandler: typeof NativeBackHandler & BackListeners;
