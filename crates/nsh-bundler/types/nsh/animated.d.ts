import { type Easing, type EasingCurves, type LayoutAnimationConfig, type LayoutAnimationPresets, type LayoutAnimationProperties, type LayoutAnimationProperty, type LayoutAnimationType, type LayoutAnimationTypes } from "./anim.gen";
export type { Easing, EasingCurve, EasingCurveName, EasingCurves, LayoutAnimationAnim, LayoutAnimationConfig, LayoutAnimationPresets, LayoutAnimationProperties, LayoutAnimationProperty, LayoutAnimationType, LayoutAnimationTypes } from "./anim.gen";
type Operand = NativeNode | number;
/** A node of the native animation graph: evaluated on the UI thread. */
export declare class NativeNode {
	readonly __nv: number;
	/** Inputs, kept alive as long as this node is. */
	private readonly inputs;
	constructor(inputs?: NativeNode[]);
	/** Maps this node through piecewise ranges: numbers, colors or angles
	* (`"45deg"`, `"1rad"`). Outside the input range it clamps by default;
	* `easing` shapes each segment. */
	interpolate(config: {
		inputRange: number[];
		outputRange: number[] | string[];
		extrapolate?: Extrapolation;
		extrapolateLeft?: Extrapolation;
		extrapolateRight?: Extrapolation;
		easing?: Easing;
	}): NativeNode;
	/** Follows this node's changes, kept within `min..max` (a header that
	* hides while content scrolls up and comes back as it scrolls down). */
	diffClamp(min: number, max: number): NativeNode;
	add(b: Operand): NativeNode;
	sub(b: Operand): NativeNode;
	mul(b: Operand): NativeNode;
	div(b: Operand): NativeNode;
	/** Modulo, never negative for a positive `b` (e.g. angles in 0..360). */
	mod(b: Operand): NativeNode;
}
export type Extrapolation = "extend" | "clamp" | "identity";
/** Easing functions (React Native's `Easing`). */
export interface EasingModule extends EasingCurves {
	linear: Easing;
	ease: Easing;
	bezier(x1: number, y1: number, x2: number, y2: number): Easing;
	in(e: Easing): Easing;
	out(e: Easing): Easing;
	inOut(e: Easing): Easing;
}
/** Easing functions (React Native's `Easing`): curves in their "in" form,
* turned around with `out` / `inOut`. */
export declare const Easing: EasingModule;
/** A settable, animatable native value. */
export declare class NativeValue extends NativeNode {
	constructor(value: number);
	/** Jumps to `value`, stopping any animation. */
	set(value: number): void;
	/** Stops the running animation where it is. */
	stop(): void;
	/** Animates to `to` over `duration` ms. Resolves with `finished`. */
	timing(to: number, options?: {
		duration?: number;
		easing?: Easing;
		delay?: number;
	}): Promise<boolean>;
	/** Springs to `to`. Resolves with `finished`. */
	spring(to: number, options?: {
		stiffness?: number;
		damping?: number;
		mass?: number;
		velocity?: number;
	}): Promise<boolean>;
	/** Coasts from `velocity` (units per second) and slows down. */
	decay(options?: {
		velocity?: number;
		deceleration?: number;
	}): Promise<boolean>;
	private start;
}
/** Creates a native value. Bind it (or nodes derived from it) to style
* properties such as `opacity`, `transform` and `backgroundColor`, to a
* scroll position (`scrollValue`) or to drags (`pan`). */
export declare function createNativeValue(value?: number): NativeValue;
/** An animation built with `Animated`: runs natively, steps and all. */
export interface CompositeAnimation {
	/** Runs it (again). Resolves (and calls `callback`) with whether it
	* finished rather than being stopped. */
	start(callback?: (result: {
		finished: boolean;
	}) => void): Promise<boolean>;
	/** Stops it where it is. */
	stop(): void;
	/** Stops it and puts its values back where they were when it started. */
	reset(): void;
}
/** Animations composed natively (React Native's `Animated`). */
export interface AnimatedModule {
	timing(value: NativeValue, config: {
		toValue: number;
		duration?: number;
		easing?: Easing;
		delay?: number;
	}): CompositeAnimation;
	spring(value: NativeValue, config: {
		toValue: number;
		stiffness?: number;
		damping?: number;
		mass?: number;
		velocity?: number;
	}): CompositeAnimation;
	decay(value: NativeValue, config?: {
		velocity?: number;
		deceleration?: number;
	}): CompositeAnimation;
	sequence(list: CompositeAnimation[]): CompositeAnimation;
	parallel(list: CompositeAnimation[], config?: {
		stopTogether?: boolean;
	}): CompositeAnimation;
	stagger(ms: number, list: CompositeAnimation[]): CompositeAnimation;
	delay(ms: number): CompositeAnimation;
	loop(animation: CompositeAnimation, config?: {
		iterations?: number;
		resetBeforeIteration?: boolean;
	}): CompositeAnimation;
	add(a: NativeNode, b: NativeNode | number): NativeNode;
	subtract(a: NativeNode, b: NativeNode | number): NativeNode;
	multiply(a: NativeNode, b: NativeNode | number): NativeNode;
	divide(a: NativeNode, b: NativeNode | number): NativeNode;
	modulo(a: NativeNode, b: NativeNode | number): NativeNode;
	diffClamp(a: NativeNode, min: number, max: number): NativeNode;
}
/** Animations composed natively (React Native's `Animated`): each step
* starts on the UI thread the moment the previous one ends. */
export declare const Animated: AnimatedModule;
/** Animates the layout changes of the next render natively: frames move,
* new views fade in, removed views fade out. */
export declare const LayoutAnimation: {
	/** Animates the changes of the next commit. `onEnd` runs when done. */
	configureNext(config: LayoutAnimationConfig, onEnd?: () => void): void;
	/** A config with `duration`, the same curve for all three phases. */
	create: (duration: number, type?: LayoutAnimationType, property?: LayoutAnimationProperty) => LayoutAnimationConfig;
	Types: LayoutAnimationTypes;
	Properties: LayoutAnimationProperties;
	Presets: LayoutAnimationPresets;
	easeInEaseOut(onEnd?: () => void): void;
	linear(onEnd?: () => void): void;
	spring(onEnd?: () => void): void;
};
