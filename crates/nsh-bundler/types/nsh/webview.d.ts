import { type YNode } from "./_core";
import type { IOSWebViewHostProps, ViewProps, WebViewHostProps } from "./types";
export interface WebViewNavigation {
	url: string;
	title: string;
	loading: boolean;
	canGoBack: boolean;
	canGoForward: boolean;
}
export interface WebViewProps extends Omit<ViewProps, "children" | "onLoad" | "onError" | "ios">, WebViewHostProps {
	source: string | {
		uri: string;
	} | {
		html: string;
		baseUrl?: string;
	};
	onLoadStart?: (e: WebViewNavigation) => void;
	onLoad?: (e: WebViewNavigation) => void;
	onError?: (e: {
		message: string;
	}) => void;
	/** The page called `window.ReactNativeWebView.postMessage(data)`. */
	onMessage?: (e: {
		data: string;
	}) => void;
	onNavigationStateChange?: (e: WebViewNavigation) => void;
	ios?: IOSWebViewHostProps;
}
/** A web page (WKWebView / android.webkit.WebView); its `ref` has
* `reload()`, `goBack()`, `goForward()`, `stopLoading()`,
* `injectJavaScript(script)` and `postMessage(data)`. */
export declare const WebView: (props: WebViewProps) => YNode;
