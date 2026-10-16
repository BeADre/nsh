import type { PanResponderCallbacks, PanResponderInstance } from "./types";
/** Pan gestures on top of the responder system (React Native's
* `PanResponder`). */
export declare const PanResponder: {
	create(config: PanResponderCallbacks): PanResponderInstance;
};
