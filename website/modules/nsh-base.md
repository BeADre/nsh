---
title: nsh/base
description: "Unstyled base components for an app's UI: buttons, navigation stacks and modals, safe areas, system dialogs, the software keyboard and the status bar."
order: 2.5
demo: ui-navigation
---

# nsh/base

`nsh/base` is the base component layer of an app's UI: unstyled components that own behavior, state, focus and
accessibility, on every platform. It sits between the primitives (`nsh` and the `nsh/<module>` modules) and the
app's own component library, as `gpui-base` sits between `gpui` and a styled library in GPUI Kit (see
[Modules](./index.md#three-layers)). It provides controls (`Button`), navigation (a `Stack` of `Screen`s driven by a navigator you create
with `createNavigator`, and `Modal` for content presented over everything else), safe areas (`SafeAreaView`,
`safeArea()` and `safeAreaInsets()`), the system's dialogs (`Alert` and `ActionSheet`), the software keyboard
(`Keyboard`, `keyboard()` and `KeyboardAvoidingView`) and the status bar (`StatusBar`).

The look is the app's: what goes inside a screen, a modal or a header is your own JSX, drawn from the primitives with
styles, [Tailwind](../docs/tailwind.md) classes and your theme, Rust components from [your own crate](../docs/rust.md),
or a component library the community builds on `nsh/base`. Every host serves `nsh/base`; Rust apps use the same API
from the `nsh-base` crate (`nsh_base::{SafeAreaView, …}`).

```tsx
import {
  ActionSheet, Alert, Button, createNavigator, Keyboard, keyboard, KeyboardAvoidingView, Modal, SafeAreaView, safeArea,
  Screen, Stack, StatusBar, safeAreaInsets,
} from "nsh/base";
import type {
  ActionSheetOptions, AlertButton, AlertButtonStyle, AlertOptions, AvoidBehavior, ButtonProps, BarAnimation, BarStyle, Edge,
  Insets, Navigator,
} from "nsh/base";
```

This page is the API reference. For walkthroughs, see the [Navigation guide](../docs/navigation.md) and
[Forms and keyboard](../docs/forms-and-keyboard.md).

## Controls

### Button

A press target that owns its behavior and has no look: presses, the disabled and selected states and the button role
for screen readers. It has no colors, sizes or label of its own. Style it with `class` or `style`, its states with the
`active:`, `hover:`, `focus:` and `disabled:` variants, and put its content inside, so one `Button` becomes your
primary button, an icon button or a row of a list.

```tsx
import { mount, signal, Text } from "nsh";
import { Button } from "nsh/base";

function App() {
  const count = signal(0);
  return (
    <Button
      onPress={() => count.update((n) => n + 1)}
      class="self-center px-5 py-3 rounded-xl bg-blue-600 active:bg-blue-700 disabled:opacity-40"
    >
      <Text class="text-white text-base font-semibold">{`Pressed ${count()}`}</Text>
    </Button>
  );
}

mount(App);
```

| Prop | Type | |
|---|---|---|
| `disabled` | `boolean` | No presses and no press feedback; `disabled:` styles apply and screen readers say it is dimmed. |
| `selected` | `boolean` | Selected (a toggled trigger, the current tab); screen readers say so. The app owns the value and styles it. |
| `accessibilityLabel` | `string` | What screen readers say, when the content is not enough (an icon). |
| `onPress` | `() => void` | It was pressed. |
| `onPressIn`, `onPressOut` | `() => void` | A touch went down on it, and ended or left: for your own press feedback (a spring, a highlight). |
| `class`, `style` | | Its look, all of it. |

## Navigation

A stack is `UINavigationController` on iOS, with the native header and swipe back. On Android it is a stack of
screens with a toolbar and predictive back. The desktops show a header of their own with a back button.

The screens' content is your own JSX. Here a small `Action` component draws the buttons:

```tsx
import { mount, Text, View } from "nsh";
import { createNavigator, Screen, Stack } from "nsh/base";

type Route = { name: "home" } | { name: "detail"; id: number };

const nav = createNavigator<Route>([{ name: "home" }]);

function Action(props: { title: string; onPress: () => void }) {
  return (
    <View onPress={props.onPress} accessibilityRole="button" class="rounded-lg bg-blue-600 px-4 py-3 active:opacity-70">
      <Text class="text-center text-base font-semibold text-white">{props.title}</Text>
    </View>
  );
}

function Home() {
  return (
    <Screen title="Inbox">
      <View style={{ flex: 1, padding: 16 }}>
        <Action title="Open message 1" onPress={() => nav.push({ name: "detail", id: 1 })} />
      </View>
    </Screen>
  );
}

function Detail(props: { id: number }) {
  return (
    <Screen title={`Message ${props.id}`}>
      <View style={{ flex: 1, padding: 16 }}>
        <Action title="Back" onPress={() => nav.pop()} />
      </View>
    </Screen>
  );
}

function App() {
  return (
    <Stack navigator={nav} style={{ flex: 1 }}>
      {(route) => (route.name === "home" ? <Home /> : <Detail id={route.id} />)}
    </Stack>
  );
}

mount(App);
```

### How a stack works

- The navigator holds the **routes**: plain values of your choosing, from the bottom of the stack to the top. A route
  is usually an object that names the screen and carries its parameters.
- `Stack` calls its child function once for each route and shows the result as a screen. It does not call the
  function again for the same route, so a screen keeps its state (signals, scroll position, inputs) while it is in the
  stack, also while other screens cover it.
- When the user leaves a screen natively (the back button, swipe back, Android back), the stack removes its route from
  the navigator, so `nav.routes()` always matches what the user sees.
- Routes are compared by **identity** (`===`). Each route in a stack must be a distinct value: push a new object
  (`{ name: "detail", id }`) rather than reusing one, and use object routes for screens that can appear twice.
- If the child function returns something other than a `<Screen>`, the stack wraps it in a screen with default
  options. Return a `<Screen>` to set a title and other header options.

### createNavigator

```ts
function createNavigator<R>(initial: R[]): Navigator<R>
```

Creates a navigator whose stack starts with the routes of `initial`, bottom to top. Each push is a screen of its own, so the
same route can be pushed twice. Create it once, for example at module
level, and pass it to `<Stack navigator>`. Any code can import it to navigate.

| Member | Description |
|---|---|
| `routes: Accessor<readonly R[]>` | The routes, bottom to top. Reactive. |
| `push(route: R): void` | Pushes a screen. |
| `pop(count?: number): void` | Pops `count` screens (default 1). At least one screen always stays. |
| `popToTop(): void` | Pops back to the first screen. |
| `replace(route: R): void` | Replaces the top screen. |
| `reset(routes: R[]): void` | Replaces all screens. |
| `remove(route: R): void` | Removes the topmost screen of `route`, wherever it is in the stack. |

```tsx
nav.push({ name: "detail", id: 7 });
nav.pop(2);
nav.reset([{ name: "home" }, { name: "detail", id: 1 }]);
const depth = () => nav.routes().length;
```

### Stack

```tsx
<Stack navigator={nav} style={{ flex: 1 }}>
  {(route, index) => …}
</Stack>
```

| Prop | Type | Description |
|---|---|---|
| `navigator` | `Navigator<R>` | The navigator that holds the routes. Required. |
| `children` | `(route: R, index: Accessor<number>) => JSX.Element` | Renders a route. Return a `<Screen>` for header options. Required. |
| `style` | `StyleProp<ViewStyle>` | The stack's style; usually `{ flex: 1 }`. |

The child function receives the route value itself and an accessor of its position in the stack.

### Screen

`Screen` is one screen of a stack and configures its header:

```tsx
<Screen
  title="Profile"
  headerTintColor="#ffffff"
  headerBackgroundColor="#1a7f37"
  headerTitleColor="#ffffff"
  headerRight={<Text onPress={edit} style={{ color: "#ffffff", fontSize: 17 }}>Edit</Text>}
>
  <Scroll style={{ flex: 1 }}>…</Scroll>
</Screen>
```

| Prop | Type | Default | Description |
|---|---|---|---|
| `title` | `string` | none | The header's title. |
| `headerShown` | `boolean` | `true` | Whether the header is shown. |
| `headerTransparent` | `boolean` | `false` | The content extends under a transparent header. |
| `headerTintColor` | `ColorValue` | platform | The color of the back button and the bar buttons. |
| `headerBackgroundColor` | `ColorValue` | platform | The header's background color. |
| `headerTitleColor` | `ColorValue` | platform | The title's color. |
| `headerBackVisible` | `boolean` | `true` | Whether the back button is shown. |
| `headerLeft` | `JSX.Element` | none | Content at the left of the header, in place of the back button. |
| `headerRight` | `JSX.Element` | none | Content at the right of the header. |
| `headerTitle` | `JSX.Element` | none | Content that replaces the title. |
| `gestureEnabled` | `boolean` | `true` | The interactive back gesture: iOS swipe back, Android predictive back that follows the finger. The back buttons work either way. |
| `preventBack` | `boolean` | `false` | The user stays on the screen. See [Leaving a screen](#leaving-a-screen). |
| `animation` | `"default" \| "fade" \| "none"` | `"default"` | How the screen enters and leaves. |
| `ios` | `{ headerLargeTitle?: boolean; headerBackTitle?: string }` | none | iOS only: a large title that shrinks into the bar as the content scrolls; the label of the back button that leads back to this screen. |
| `desktop` | `{ headerLargeTitle?: boolean; headerBackTitle?: string }` | none | Desktop only (macOS, Windows, Linux): the same options on a desktop header. |
| `style` | `StyleProp<ViewStyle>` | none | The screen's style. |
| `children` | `JSX.Element` | none | The content. |
| `onBackRequest` | `() => void` | none | The user tried to leave a screen that has `preventBack`. |
| `onDismiss` | `() => void` | none | The user left the screen natively and its route was removed. |

The header items (`headerLeft`, `headerRight`, `headerTitle`) are ordinary JSX: they can read signals, handle presses
and update while the screen is shown.

The screen's content starts below an opaque header. With `headerTransparent` or `headerShown={false}`, it starts at
the top of the screen; use [SafeAreaView](#safeareaview) or `safeAreaInsets()` to keep it clear of the header and
the status bar.

```tsx
<Screen title="Large title" ios={{ headerLargeTitle: true }}>
  <Scroll style={{ flex: 1 }}>…</Scroll>
</Screen>
```

#### Leaving a screen

`preventBack` keeps the user on a screen, for example while a form has unsaved changes. While it is true, swipe back is
off, and the header's back button and the Android back button call `onBackRequest` instead of popping. Ask the user,
then pop yourself:

```tsx
import { Input, signal } from "nsh";
import { Alert, Screen } from "nsh/base";

function EditNote() {
  const text = signal("");
  const confirm = async () => {
    const i = await Alert.alert("Discard your changes?", undefined, [
      { text: "Keep editing", style: "cancel" },
      { text: "Discard", style: "destructive" },
    ]);
    if (i === 1) nav.pop();
  };
  return (
    <Screen title="Edit" preventBack={text().length > 0} onBackRequest={confirm}>
      <Input value={text()} onChangeText={(v) => text.set(v)} placeholder="A note" />
    </Screen>
  );
}
```

> [!IMPORTANT]
> The system asks whether the user may go back on the main thread, at the moment the user acts, and the main thread
> never waits for JS. So NSH answers from the `preventBack` value already mounted. Set `preventBack` as soon as there
> is something to lose, before the user goes back.

To turn off only the swipe gesture and keep the back buttons, use `gestureEnabled={false}`.

### Modal

`Modal` presents content over everything else, including the stack's header, in its own native window layer. It is
shown while `visible` is true (the default), and it is controlled: when the user asks to close it, `onRequestClose` is
called and you set `visible` to false.

```tsx
import { signal, Text, View } from "nsh";
import { Modal, SafeAreaView } from "nsh/base";

function Settings() {
  const open = signal(false);
  return (
    <View>
      <Text onPress={() => open.set(true)} accessibilityRole="button" class="text-blue-600">About</Text>
      <Modal visible={open()} ios={{ presentationStyle: "pageSheet" }} onRequestClose={() => open.set(false)}>
        <SafeAreaView style={{ flex: 1, padding: 20, gap: 12 }}>
          <Text style={{ fontSize: 24, fontWeight: "700" }}>About</Text>
          <Text onPress={() => open.set(false)} accessibilityRole="button" class="text-blue-600">Close</Text>
        </SafeAreaView>
      </Modal>
    </View>
  );
}
```

| Prop | Type | Default | Description |
|---|---|---|---|
| `visible` | `boolean` | `true` | Shown while true. |
| `animationType` | `"slide" \| "fade" \| "none"` | `"slide"` | How the modal appears. |
| `transparent` | `boolean` | `false` | The background shows what is behind, for dialogs and overlays. |
| `dismissible` | `boolean` | `false` | The user closes it natively (iOS sheet swiped down, Android back button); then `onRequestClose` and `onDismiss` are called. Otherwise those gestures ask to close. |
| `ios` | `{ presentationStyle?: "fullScreen" \| "pageSheet" \| "formSheet"; supportedOrientations?: (…)[] }` | none | iOS only: `pageSheet` and `formSheet` keep what is behind visible; the orientations the modal allows (default: the presenter's). |
| `desktop` | `{ presentationStyle?: "fullScreen" \| "pageSheet" \| "formSheet" }` | none | Desktop only (macOS, Windows, Linux): `pageSheet` and `formSheet` are cards centered in the window over a dimmed backdrop; clicking the backdrop or pressing Escape asks to close. |
| `android` | `{ statusBarTranslucent?: boolean }` | none | Android only: the content reaches under the status bar (by default it starts below it). |
| `style` | `StyleProp<ViewStyle>` | none | The modal's style. |
| `children` | `JSX.Element` | none | The content. |
| `onRequestClose` | `() => void` | none | The user asked to close it (Android back button, iOS sheet swipe). Set `visible` to false. |
| `onShow` | `() => void` | none | The modal was shown. |
| `onDismiss` | `() => void` | none | The modal was dismissed. |

A modal provides its own insets to its content, so [SafeAreaView](#safeareaview) and `safeAreaInsets()` inside it
use the modal's insets. A modal with `transparent` and `animationType="fade"` works as a dialog or a bottom sheet of
your own design:

```tsx
<Modal visible={open()} transparent animationType="fade" onRequestClose={close}>
  <View style={{ flex: 1, backgroundColor: "rgba(0,0,0,0.4)", alignItems: "center", justifyContent: "center", padding: 32 }}>
    <View style={{ alignSelf: "stretch", backgroundColor: "#ffffff", borderRadius: 14, padding: 20, gap: 12 }}>
      <Text style={{ fontSize: 20, fontWeight: "700" }}>A dialog</Text>
      <Text onPress={close} accessibilityRole="button" class="self-end font-semibold text-blue-600">OK</Text>
    </View>
  </View>
</Modal>
```

See [Modal](../elements/modal.md) for more about the element.

## Safe areas

### SafeAreaView

`SafeAreaView` adds padding so that its content stays clear of the notch, the status bar, the home indicator, a
navigation header or the Mac Catalyst title bar. The padding is the insets **where the view renders**. Inside a
[Screen](#screen) these are the insets below the screen's header. Inside a [Modal](#modal) they are the modal's
insets. Elsewhere they are the surface's safe area.

```tsx
import { mount, Text } from "nsh";
import { SafeAreaView } from "nsh/base";

function App() {
  return (
    <SafeAreaView style={{ flex: 1, backgroundColor: "#fff8c5", padding: 16 }}>
      <Text style={{ fontSize: 24, fontWeight: "700" }}>Clear of the notch</Text>
    </SafeAreaView>
  );
}

mount(App);
```

The insets are **added** to the view's own padding. In the example, the content starts 16 pt below the top inset, and
the background color fills the whole area, including the area behind the status bar.

By default all four edges are padded. Pass `edges` to pad only some of them, for example when a tab bar or a bottom
bar of your own handles the bottom inset:

```tsx
<SafeAreaView edges={["top", "left", "right"]} style={{ flex: 1 }}>
  …
</SafeAreaView>
```

A [Screen](#screen) with a visible, opaque header already places its content below the header, so the top inset there
is 0. SafeAreaView is for:

- the root of an app without a navigation stack,
- a screen with `headerShown={false}` or `headerTransparent`,
- the content of a full-screen [Modal](#modal).

| Prop | Type | Default | Description |
|---|---|---|---|
| `edges` | `Edge[] \| null` (`"top" \| "right" \| "bottom" \| "left"`) | all four | The edges to pad. |
| `children` | `JSX.Element` | none | The content. |
| `class`, `classList`, `style`, `testID`, `onLayout`, `ref` | | | Applied to the view. Its padding is added to the insets. |

### safeArea and safeAreaInsets

When you need the numbers, for example to pad a bottom bar yourself, use one of the two accessors. Both return an
`Insets` object (`{ top, right, bottom, left }` in points) and are reactive: code that reads them runs again when they
change, for example on rotation.

| Export | Returns |
|---|---|
| `safeArea()` | The surface's safe area: the parts clear of system UI (notch, home indicator, title bar). |
| `safeAreaInsets()` | Call it in a component. It returns an accessor of the insets where that component renders: the nearest screen's (below its header), the nearest modal's, or else the surface's safe area. These are the insets SafeAreaView uses. |

```tsx
import { Text, View } from "nsh";
import { safeAreaInsets } from "nsh/base";

function BottomBar() {
  const insets = safeAreaInsets();
  return (
    <View style={{ flexDirection: "row", padding: 8, paddingBottom: 8 + insets().bottom }}>
      <Text>Bottom bar</Text>
    </View>
  );
}
```

### Navigation and safe areas on each platform

- **iOS**: `UINavigationController` with the native header, back button and swipe back. `ios={{ headerLargeTitle }}`
  and `headerBackTitle` are iOS only. Stack changes that arrive during a running transition are applied when it ends.
  The insets come from UIKit's safe area (status bar, notch or Dynamic Island, home indicator).
- **Android**: a stack with a toolbar. The system back button pops the top screen, with predictive back on Android
  versions that have it. See `BackHandler` in [nsh/android](./nsh-android.md) to handle the back button yourself.
  NSH reports the parts of the system bars and the display cutout that overlap the NSH view; when NSH is
  [embedded](../docs/embedding-android.md) below a native toolbar, the view's top inset is 0.
- **Mac Catalyst**: as on iOS. Page and form sheets are presented as macOS sheets the size of the window. The top
  inset is the window's title bar.

## Dialogs

`Alert` and `ActionSheet` show the platform's own dialogs: `Alert` is `UIAlertController` on iOS and `AlertDialog` on
Android, `ActionSheet` an action sheet on iOS and a list dialog on Android. Each returns a promise that resolves with
the index of the button or action the user chose. The dialogs have the platform's own look.

### Alert

```ts
Alert.alert(
  title: string,
  message?: string | null,
  buttons?: AlertButton[] | null,
  options?: AlertOptions | null,
): Promise<number>
```

The promise resolves with the index of the pressed button in `buttons`, or `-1` when the alert was dismissed without
a button (Android, with `cancelable`). It always resolves.

```tsx
import { mount, signal, Text, View } from "nsh";
import { Alert } from "nsh/base";

function App() {
  const answer = signal("–");
  const confirm = async () => {
    const i = await Alert.alert("Delete the photo?", "This cannot be undone.", [
      { text: "Cancel", style: "cancel" },
      { text: "Delete", style: "destructive" },
    ]);
    answer.set(i === 1 ? "deleted" : "kept");
  };
  return (
    <View style={{ flex: 1, padding: 16, paddingTop: 48, gap: 12 }}>
      <View onPress={confirm} accessibilityRole="button" class="rounded-lg bg-red-600 px-4 py-3 active:opacity-70">
        <Text class="text-center font-semibold text-white">Delete…</Text>
      </View>
      <Text>{answer()}</Text>
    </View>
  );
}

mount(App);
```

With no buttons, the alert has one "OK" button:

```tsx
await Alert.alert("Saved", "Your changes were saved.");
```

#### AlertButton

| Field | Type | Default | Description |
|---|---|---|---|
| `text` | `string \| null` | `"OK"` | The button's label. |
| `style` | `"default" \| "cancel" \| "destructive"` | `"default"` | `cancel` marks the cancel button; `destructive` marks a destructive action (red on iOS). |

#### AlertOptions

| Field | Type | Default | Description |
|---|---|---|---|
| `cancelable` | `boolean` | `false` | Android: a tap outside the dialog or the back button dismisses it, and the promise resolves with `-1`. |

> [!TIP]
> Because the main thread never waits for JS, the dialog is shown and answered natively and your code continues when
> the promise resolves. You can `await` an alert inside any event handler, for example to confirm leaving a screen
> with [`preventBack`](#leaving-a-screen).

### ActionSheet

```ts
ActionSheet.show(options: ActionSheetOptions): Promise<number>
```

The promise resolves with the index of the chosen action in `options.options`. When the user dismisses the sheet
without choosing (a tap outside, the back button), it resolves with `cancelButtonIndex`, or with `-1` when the sheet
has no cancel action. It always resolves.

```tsx
import { mount, signal, Text, View } from "nsh";
import { ActionSheet } from "nsh/base";

function App() {
  const chosen = signal("–");
  const share = async () => {
    const options = ["Cancel", "Copy link", "Save", "Delete"];
    const i = await ActionSheet.show({
      title: "Share the photo",
      options,
      cancelButtonIndex: 0,
      destructiveButtonIndexes: [3],
    });
    chosen.set(options[i] ?? "dismissed");
  };
  return (
    <View style={{ flex: 1, padding: 16, paddingTop: 48, gap: 12 }}>
      <View onPress={share} accessibilityRole="button" class="rounded-lg bg-blue-600 px-4 py-3 active:opacity-70">
        <Text class="text-center font-semibold text-white">Share…</Text>
      </View>
      <Text>{chosen()}</Text>
    </View>
  );
}

mount(App);
```

#### ActionSheetOptions

| Field | Type | Default | Description |
|---|---|---|---|
| `options` | `string[]` | required | The actions' labels, in order. |
| `cancelButtonIndex` | `number \| null` | none | The index of the cancel action. |
| `destructiveButtonIndexes` | `number[] \| null` | none | The indexes of destructive actions (red on iOS). |
| `title` | `string \| null` | none | Shown above the actions. |
| `message` | `string \| null` | none | Shown below the title. |

### Dialogs on each platform

- **iOS**: the buttons of an alert are laid out by `UIAlertController`. A button with `style: "cancel"` is placed where
  iOS puts cancel buttons, which may differ from its position in your array; the index you get back is always its
  index in `buttons`. An action sheet shows the cancel action separately at its bottom. On iPad and Mac Catalyst the
  action sheet is a popover: a tap outside it dismisses it, and the promise resolves with `cancelButtonIndex` (or
  `-1`).
- **Android**: an `AlertDialog` shows up to three buttons, mapped by position: the last button is the positive
  button, the one before it the negative button, and the one before that the neutral button. An action sheet is a
  list of the actions in a dialog, with the cancel action as a tap outside or the back button. The list dialog shows
  the `title`, or the `message` when the sheet has no title. Button and action styles keep the platform's look.

## Keyboard

`Keyboard` hides the software keyboard, the `keyboard()` accessor tells you how much of the screen it covers, and
`KeyboardAvoidingView` keeps content above it, moving with it.

### Keyboard.dismiss

```ts
Keyboard.dismiss(): void
```

Unfocuses the focused input, which hides the keyboard. With no input focused, it leaves everything as it is.

```tsx
import { Input, mount, signal, Text, View } from "nsh";
import { Keyboard } from "nsh/base";

function App() {
  const name = signal("");
  return (
    <View style={{ flex: 1, padding: 16, paddingTop: 48, gap: 12 }}>
      <Input
        value={name()}
        onChangeText={(v) => name.set(v)}
        placeholder="Name"
        class="rounded-lg border border-neutral-300 px-3 py-2 text-base"
      />
      <View onPress={() => Keyboard.dismiss()} accessibilityRole="button" class="rounded-lg bg-blue-600 px-4 py-3 active:opacity-70">
        <Text class="text-center font-semibold text-white">Done</Text>
      </View>
    </View>
  );
}

mount(App);
```

> [!TIP]
> To hide the keyboard when the user drags a scroll view, set `keyboardDismissMode="on-drag"` or `"interactive"` on
> the [scroll view](../elements/scroll.md) or [List](../elements/list.md).

### Keyboard height

```ts
keyboard: Accessor<{ height: number; duration: number }>
```

`keyboard()` returns the height of the software keyboard over the NSH surface, in points (0 while it is hidden), and
the duration of the keyboard's show or hide animation, in seconds. It is reactive: an effect or JSX expression that
reads it runs again when the keyboard shows, hides or changes height.

```tsx
import { Input, View } from "nsh";
import { keyboard, safeAreaInsets } from "nsh/base";

function MessageBar() {
  const insets = safeAreaInsets();
  return (
    <View
      style={{
        flexDirection: "row",
        padding: 8,
        // The home indicator's inset only matters while the keyboard is hidden.
        paddingBottom: 8 + (keyboard().height > 0 ? 0 : insets().bottom),
      }}
    >
      <Input placeholder="Message" style={{ flex: 1 }} />
    </View>
  );
}
```

The height is the part of the keyboard that overlaps the NSH surface. When the window itself shrinks for the keyboard
(an Android activity that resizes), or when NSH is embedded in a view that ends above the keyboard, the height is 0.

> [!NOTE]
> To keep content above the keyboard, [KeyboardAvoidingView](#keyboardavoidingview) is usually simpler than reading
> `keyboard()` yourself: it animates with the keyboard natively.

### KeyboardAvoidingView

`KeyboardAvoidingView` keeps its content above the software keyboard. When the keyboard shows or hides, the view makes
room by the keyboard's height, and the change animates with the keyboard. Use it for content outside scroll views,
such as a message bar at the bottom of a chat screen. Inputs inside a [scroll view](../elements/scroll.md) or a
[List](../elements/list.md) are scrolled into view natively.

```tsx
import { Input, mount, Scroll, signal, Text, View } from "nsh";
import { KeyboardAvoidingView } from "nsh/base";

function App() {
  const draft = signal("");
  return (
    <KeyboardAvoidingView style={{ flex: 1, backgroundColor: "#ffffff" }}>
      <Scroll style={{ flex: 1 }} keyboardDismissMode="interactive">
        <Text style={{ padding: 16, paddingTop: 48 }}>Messages…</Text>
      </Scroll>
      <View style={{ flexDirection: "row", gap: 8, padding: 8, borderTopWidth: 1, borderTopColor: "#e5e5e5" }}>
        <Input
          value={draft()}
          onChangeText={(v) => draft.set(v)}
          placeholder="Message"
          style={{ flex: 1, fontSize: 16, padding: 8 }}
        />
      </View>
    </KeyboardAvoidingView>
  );
}

mount(App);
```

The view assumes that it reaches the bottom of the screen, as a screen's content does: it makes room by the full
keyboard height. When something is below it, such as a tab bar, subtract that with `keyboardVerticalOffset` (a
negative value) or add to it with a positive value.

#### Behaviors

| `behavior` | How it makes room |
|---|---|
| `"padding"` (default) | Adds the keyboard's height to the view's bottom padding. The content above shrinks; the background still reaches the bottom of the screen. |
| `"height"` | Adds the keyboard's height as a bottom margin. The whole view, including its background, ends at the top of the keyboard. |
| `"position"` | Moves an inner content view up by the keyboard's height and keeps its size. Style that inner view with `contentContainerStyle`. |

```tsx
<KeyboardAvoidingView behavior="position" contentContainerStyle={{ padding: 16, gap: 12 }} style={{ flex: 1 }}>
  <Input placeholder="E-mail" keyboardType="email-address" class="rounded-lg border border-neutral-300 px-3 py-2" />
  <View onPress={signIn} accessibilityRole="button" class="rounded-lg bg-blue-600 px-4 py-3">
    <Text class="text-center font-semibold text-white">Sign in</Text>
  </View>
</KeyboardAvoidingView>
```

The padding or margin is added to what the view's own style sets, so your padding stays.

#### KeyboardAvoidingView props

| Prop | Type | Default | Description |
|---|---|---|---|
| `enabled` | `boolean` | `true` | Avoid the keyboard. While false, the view keeps its own layout. |
| `behavior` | `"padding" \| "height" \| "position"` | `"padding"` | How the view makes room (see [Behaviors](#behaviors)). |
| `keyboardVerticalOffset` | `number` | `0` | Added to the keyboard's height while it shows (for example, the height of a bar below the view). The total is at least 0. |
| `contentContainerStyle` | `StyleProp<ViewStyle>` | none | The inner view's style with `behavior="position"`. |
| `children` | `JSX.Element` | none | The content. |
| `class`, `classList`, `style`, `testID`, `onLayout`, `ref` | | | Applied to the outer view. |

### The keyboard on each platform

- **iOS**: the room animates with the keyboard's own duration and an ease-in-ease-out curve.
- **Android**: where the window shrinks for the keyboard (an activity that resizes), the content already ends above
  the keyboard, and KeyboardAvoidingView keeps its layout. Where the keyboard overlaps the view, it makes room as on
  iOS.
- **Desktops and Mac Catalyst** type with a hardware keyboard: `keyboard().height` stays 0 and the view keeps its
  layout.

## Status bar

`StatusBar` sets how the system status bar looks: the color of its content (time, battery, signal) and whether it is
hidden. Render it as a component, so that the setting applies while that part of the UI is shown, or call its static
functions.

### StatusBar

A rendered `StatusBar` takes up no space. Its setting applies while it is mounted:

```tsx
import { mount, Text, View } from "nsh";
import { StatusBar } from "nsh/base";

function App() {
  return (
    <View style={{ flex: 1, backgroundColor: "#0b0b0f" }}>
      <StatusBar barStyle="light-content" />
      <Text style={{ color: "#ffffff", padding: 16, paddingTop: 64 }}>Dark screen, light status bar</Text>
    </View>
  );
}

mount(App);
```

You can render several `StatusBar`s, for example one per screen. The **most recently mounted** one wins, **prop by
prop**: a `StatusBar` that only sets `hidden` keeps the `barStyle` of the one below it. When a `StatusBar` unmounts,
for example when its screen is popped, the previous setting applies again.

```tsx
function PhotoViewer() {
  return (
    <Screen headerShown={false}>
      <StatusBar hidden animated />
      …
    </Screen>
  );
}
```

#### StatusBar props

| Prop | Type | Default | Description |
|---|---|---|---|
| `barStyle` | `"default" \| "light-content" \| "dark-content" \| null` | not set | The color of the content. |
| `hidden` | `boolean \| null` | not set | Hides the status bar. |
| `animated` | `boolean \| null` | not set | Animates changes of `barStyle` and `hidden`. |
| `android` | `{ backgroundColor?: string \| null }` | not set | Android only: the bar's background color. |
| `web` | `{ themeColor?: string \| null }` | not set | Browsers only: the page's theme color (`<meta name="theme-color">`). |

A prop left unset leaves that part of the status bar to the `StatusBar` below, or to the functions.

#### Bar styles

| `barStyle` | Content |
|---|---|
| `"default"` | The platform's default for the current appearance (light or dark mode). |
| `"light-content"` | Light content, for dark backgrounds. |
| `"dark-content"` | Dark content, for light backgrounds. |

#### Setting it with functions

The static functions set a base setting. Every rendered `StatusBar` overrides it.

```tsx
StatusBar.setBarStyle("dark-content", true);
StatusBar.setHidden(true, "fade");
StatusBar.setHidden(false);
```

| Function | Description |
|---|---|
| `StatusBar.setBarStyle(barStyle: BarStyle, animated?: boolean \| null): void` | Sets the color of the content. `animated` defaults to `false`. |
| `StatusBar.setHidden(hidden: boolean, animation?: "none" \| "fade" \| "slide" \| null): void` | Hides or shows the bar. Any animation other than `"none"` animates the change. |

#### In a browser

Browsers give their own bars around the page a color (the address bar on phones, the title bar of an installed web
app). `web={{ themeColor }}` sets it, as the page's `<meta name="theme-color">`, with the same most-recent-wins rule.

```tsx
<StatusBar barStyle="light-content" web={{ themeColor: "#0b0b0f" }} />
```

### The status bar on each platform

- **iOS**: hiding and showing animate with a fade; `"slide"` also fades. In an app
  [embedded](../docs/embedding-ios.md) in a native one, the host app's view controller decides how the status bar
  looks. The host applies NSH's setting by returning `NshAppView.preferredStatusBarStyle`,
  `NshAppView.prefersStatusBarHidden` and `NshAppView.preferredStatusBarUpdateAnimation` from its view controller.
- **Android**: `android={{ backgroundColor }}` colors the bar before edge-to-edge (Android 15, API 35, draws apps
  behind the status bar).
- **Desktops, Mac Catalyst and the web**: `barStyle` and `hidden` apply where the platform has a status bar; the web
  takes `web={{ themeColor }}`.

> [!IMPORTANT]
> When your `nsh.json` declares `"capabilities"`, the status bar changes only when they include
> `"statusBar": true`; other changes are refused and logged. See
> [Sandbox and capabilities](../docs/sandbox-and-capabilities.md).

```json
{
  "capabilities": {
    "statusBar": true
  }
}
```

## Related

- [Navigation guide](../docs/navigation.md).
- [Forms and keyboard](../docs/forms-and-keyboard.md).
- [Modal](../elements/modal.md) and [Input](../elements/input.md).
- [Device and environment](../docs/device-and-environment.md).
- [nsh/android](./nsh-android.md): `BackHandler`, `Toast.show` for short messages, `setStatusBarBackgroundColor` and
  other Android-only APIs.
