import type { Accessor, AndroidModalHostProps, IOSModalHostProps, IOSScreenHostProps, JSX, ModalHostProps, ScreenHostProps, StyleProp, ViewStyle, YNode } from "nsh";
/** A stack of routes for `<Stack>`: plain values, compared by identity. */
export interface Navigator<R> {
	/** The routes, bottom to top (reactive). */
	routes: Accessor<readonly R[]>;
	/** Pushes a screen. */
	push(route: R): void;
	/** Pops `count` screens (at least one stays). */
	pop(count?: number): void;
	/** Pops back to the first screen. */
	popToTop(): void;
	/** Replaces the top screen. */
	replace(route: R): void;
	/** Replaces all screens. */
	reset(routes: R[]): void;
	/** Removes a route wherever it is (the user popped its screen). */
	remove(route: R): void;
}
/** Creates the route stack of a `<Stack>`. */
export declare function createNavigator<R>(initial: R | R[]): Navigator<R>;
export interface StackProps<R> {
	navigator: Navigator<R>;
	style?: StyleProp<ViewStyle>;
	/** Renders a route; return a `<Screen>` for header options. */
	children: (route: R, index: Accessor<number>) => JSX.Element;
}
/**
* A native navigation stack: UINavigationController on iOS (swipe back,
* native header), a stack with a toolbar and predictive back on Android.
* Screens stay mounted (with their state) while they are in the stack.
*/
export declare function Stack<R>(props: StackProps<R>): YNode;
export interface ScreenProps extends ScreenHostProps {
	headerLeft?: JSX.Element;
	headerRight?: JSX.Element;
	/** Replaces the title. */
	headerTitle?: JSX.Element;
	/** The user tried to leave a screen with `preventBack`. */
	onBackRequest?: () => void;
	style?: StyleProp<ViewStyle>;
	onDismiss?: () => void;
	children?: JSX.Element;
	ios?: IOSScreenHostProps;
}
/** One screen of a `<Stack>`, with its header options. */
export declare function Screen(props: ScreenProps): YNode;
export interface ModalProps extends ModalHostProps {
	/** Shown while true (default). */
	visible?: boolean;
	/** The user asked to close (Android back button, iOS sheet swipe): set
	* `visible` to false to close it. */
	onRequestClose?: () => void;
	onShow?: () => void;
	onDismiss?: () => void;
	style?: StyleProp<ViewStyle>;
	children?: JSX.Element;
	ios?: IOSModalHostProps;
	android?: AndroidModalHostProps;
}
/** Content presented over everything else, in its own native window layer. */
export declare function Modal(props: ModalProps): any;
