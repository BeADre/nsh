---
title: nsh/base
description: 应用界面的无样式基础组件：按钮、导航栈与模态框、安全区、系统对话框、软键盘和状态栏。
order: 2.5
demo: ui-navigation
---

# nsh/base

`nsh/base` 是应用界面的基础组件层：不带样式、负责行为、状态、焦点和无障碍的组件，各平台通用。它位于基础元素（`nsh` 和各个 `nsh/<module>` 模块）与应用自己的组件库之间，就像 GPUI Kit 中 `gpui-base` 位于 `gpui` 与带样式的组件库之间（见[模块](./index.md#three-layers)）。它提供控件（`Button`）、导航（由 `createNavigator` 创建的导航器驱动的、由多个 `Screen` 组成的 `Stack`，以及在所有内容之上展示内容的 `Modal`）、安全区（`SafeAreaView`、`safeArea()` 和 `safeAreaInsets()`）、系统对话框（`Alert` 和 `ActionSheet`）、软键盘（`Keyboard`、`keyboard()` 和 `KeyboardAvoidingView`）以及状态栏（`StatusBar`）。

外观由应用决定：页面、模态框或标题栏里的内容都是你自己的 JSX，用基础元素配合样式、[Tailwind](../docs/tailwind.md) 类和主题绘制；或者是[你自己的 crate](../docs/rust.md) 中的 Rust 组件；也可以是社区基于 `nsh/base` 构建的组件库。每个宿主都提供 `nsh/base`；Rust 应用通过 `nsh-base` crate 使用同一套 API（`nsh_base::{SafeAreaView, …}`）。

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

本页是 API 参考。完整的上手讲解见[导航指南](../docs/navigation.md)和[表单与键盘](../docs/forms-and-keyboard.md)。

<a id="controls"></a>

## 控件

<a id="button"></a>

### Button

一个只负责行为、没有外观的按压目标：处理按压、禁用和选中状态，并向屏幕阅读器报告按钮角色。它没有自己的颜色、尺寸或文字。用 `class` 或 `style` 设置样式，用 `active:`、`hover:`、`focus:`、`disabled:` 变体设置各状态的样式，内容放在它里面。这样同一个 `Button` 可以做成你的主按钮、图标按钮或列表的一行。

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

| 属性 | 类型 | |
|---|---|---|
| `disabled` | `boolean` | 不响应按压，也没有按压反馈；`disabled:` 样式生效，屏幕阅读器会报告它不可用。 |
| `selected` | `boolean` | 选中状态（切换中的触发器、当前标签页），屏幕阅读器会报告。值由应用持有，样式也由应用决定。 |
| `accessibilityLabel` | `string` | 内容不足以说明时（比如只有图标），屏幕阅读器读出的文字。 |
| `onPress` | `() => void` | 被按下。 |
| `onPressIn`、`onPressOut` | `() => void` | 触摸在它上面按下，以及结束或移开：用来做你自己的按压反馈（弹簧缩放、高亮）。 |
| `class`、`style` | | 它的全部外观。 |

<a id="navigation"></a>

## 导航

在 iOS 上，栈就是 `UINavigationController`，带原生标题栏和侧滑返回；在 Android 上，它是带工具栏和预测性返回的页面栈；桌面平台显示自己的标题栏和返回按钮。

页面的内容就是你自己的 JSX。这里用一个小小的 `Action` 组件绘制按钮：

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

<a id="how-a-stack-works"></a>

### 栈的工作方式

- 导航器保存**路由**：由你自行决定的普通值，从栈底到栈顶排列。路由通常是一个对象，标明页面并携带参数。
- `Stack` 为每个路由调用一次子函数，并把结果显示为一个页面。对同一个路由它只调用一次该函数，因此页面在栈中期间（包括被其他页面覆盖时）会保留状态：signal、滚动位置、输入内容。
- 当用户以原生方式离开页面（返回按钮、侧滑返回、Android 返回键）时，栈会从导航器中移除对应路由，因此 `nav.routes()` 始终与用户看到的一致。
- 路由按**引用**（`===`）比较。栈中的每个路由都必须是不同的值：每次 push 一个新对象（`{ name: "detail", id }`）；可能出现两次的页面使用对象路由。
- 如果子函数返回的不是 `<Screen>`，栈会用一个默认选项的页面把它包起来。要设置标题和其他标题栏选项，请返回 `<Screen>`。

### createNavigator

```ts
function createNavigator<R>(initial: R[]): Navigator<R>
```

创建一个导航器，其栈从 `initial` 中的路由开始（从栈底到栈顶）。每次 push 都是一个独立的页面，同一个路由可以 push 两次。只需创建一次（例如在模块顶层），然后传给 `<Stack navigator>`。任何代码都可以导入它来进行导航。

| 成员 | 说明 |
|---|---|
| `routes: Accessor<readonly R[]>` | 路由，从栈底到栈顶。响应式。 |
| `push(route: R): void` | 压入一个页面。 |
| `pop(count?: number): void` | 弹出 `count` 个页面（默认 1）。至少保留一个页面。 |
| `popToTop(): void` | 返回到第一个页面。 |
| `replace(route: R): void` | 替换栈顶页面。 |
| `reset(routes: R[]): void` | 替换所有页面。 |
| `remove(route: R): void` | 移除 `route` 最上面的那个页面，无论它在栈中的什么位置。 |

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

| 属性 | 类型 | 说明 |
|---|---|---|
| `navigator` | `Navigator<R>` | 保存路由的导航器。必填。 |
| `children` | `(route: R, index: Accessor<number>) => JSX.Element` | 渲染一个路由。返回 `<Screen>` 以设置标题栏选项。必填。 |
| `style` | `StyleProp<ViewStyle>` | 栈的样式，通常为 `{ flex: 1 }`。 |

子函数接收路由值本身，以及它在栈中位置的访问器。

### Screen

`Screen` 是栈中的一个页面，并配置其标题栏：

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

| 属性 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `title` | `string` | 无 | 标题栏的标题。 |
| `headerShown` | `boolean` | `true` | 是否显示标题栏。 |
| `headerTransparent` | `boolean` | `false` | 内容延伸到透明标题栏下方。 |
| `headerTintColor` | `ColorValue` | 平台默认 | 返回按钮和栏按钮的颜色。 |
| `headerBackgroundColor` | `ColorValue` | 平台默认 | 标题栏背景色。 |
| `headerTitleColor` | `ColorValue` | 平台默认 | 标题颜色。 |
| `headerBackVisible` | `boolean` | `true` | 是否显示返回按钮。 |
| `headerLeft` | `JSX.Element` | 无 | 标题栏左侧的内容，替代返回按钮。 |
| `headerRight` | `JSX.Element` | 无 | 标题栏右侧的内容。 |
| `headerTitle` | `JSX.Element` | 无 | 替代标题的内容。 |
| `gestureEnabled` | `boolean` | `true` | 交互式返回手势：iOS 侧滑返回、Android 跟随手指的预测性返回。无论如何，返回按钮都可用。 |
| `preventBack` | `boolean` | `false` | 用户停留在该页面。见[离开页面](#leaving-a-screen)。 |
| `animation` | `"default" \| "fade" \| "none"` | `"default"` | 页面进入和离开的方式。 |
| `ios` | `{ headerLargeTitle?: boolean; headerBackTitle?: string }` | 无 | 仅 iOS：随内容滚动收缩进栏内的大标题；返回到本页面的返回按钮文字。 |
| `desktop` | `{ headerLargeTitle?: boolean; headerBackTitle?: string }` | 无 | 仅桌面（macOS、Windows、Linux）：桌面标题栏上的同样选项。 |
| `style` | `StyleProp<ViewStyle>` | 无 | 页面样式。 |
| `children` | `JSX.Element` | 无 | 内容。 |
| `onBackRequest` | `() => void` | 无 | 用户试图离开设置了 `preventBack` 的页面。 |
| `onDismiss` | `() => void` | 无 | 用户以原生方式离开了页面，其路由已被移除。 |

标题栏项（`headerLeft`、`headerRight`、`headerTitle`）就是普通的 JSX：它们可以读取 signal、处理点击，并在页面显示期间更新。

页面内容从不透明标题栏的下方开始。设置 `headerTransparent` 或 `headerShown={false}` 时，内容从屏幕顶部开始；可以用 [SafeAreaView](#safeareaview) 或 `safeAreaInsets()` 避开标题栏和状态栏。

```tsx
<Screen title="Large title" ios={{ headerLargeTitle: true }}>
  <Scroll style={{ flex: 1 }}>…</Scroll>
</Screen>
```

<a id="leaving-a-screen"></a>

#### 离开页面

`preventBack` 让用户停留在当前页面，例如表单中有未保存的修改时。它为 true 时，侧滑返回被关闭，标题栏的返回按钮和 Android 返回键会调用 `onBackRequest` 而不是弹出页面。询问用户后，再由你自己弹出：

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
> 系统会在用户操作的那一刻，在主线程上询问是否允许返回，而主线程从不等待 JS。因此 NSH 依据已经挂载的 `preventBack` 值作答。一旦有可能丢失的内容就设置 `preventBack`，赶在用户返回之前。

如果只想关闭侧滑手势而保留返回按钮，使用 `gestureEnabled={false}`。

### Modal

`Modal` 在所有内容之上（包括栈的标题栏）展示内容，位于它自己的原生窗口层中。`visible` 为 true（默认）时显示，并且它是受控的：用户请求关闭时会调用 `onRequestClose`，由你把 `visible` 设为 false。

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

| 属性 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `visible` | `boolean` | `true` | 为 true 时显示。 |
| `animationType` | `"slide" \| "fade" \| "none"` | `"slide"` | Modal 出现的方式。 |
| `transparent` | `boolean` | `false` | 背景透出后面的内容，用于对话框和浮层。 |
| `dismissible` | `boolean` | `false` | 用户以原生方式关闭它（iOS 下滑关闭 sheet、Android 返回键），随后调用 `onRequestClose` 和 `onDismiss`。否则这些手势只是请求关闭。 |
| `ios` | `{ presentationStyle?: "fullScreen" \| "pageSheet" \| "formSheet"; supportedOrientations?: (…)[] }` | 无 | 仅 iOS：`pageSheet` 和 `formSheet` 会让后面的内容保持可见；Modal 允许的屏幕方向（默认与呈现它的界面一致）。 |
| `desktop` | `{ presentationStyle?: "fullScreen" \| "pageSheet" \| "formSheet" }` | 无 | 仅桌面（macOS、Windows、Linux）：`pageSheet` 和 `formSheet` 是窗口中居中的卡片，背后的内容变暗；点击背景或按 Escape 请求关闭。 |
| `android` | `{ statusBarTranslucent?: boolean }` | 无 | 仅 Android：内容延伸到状态栏下方（默认从状态栏下方开始）。 |
| `style` | `StyleProp<ViewStyle>` | 无 | Modal 的样式。 |
| `children` | `JSX.Element` | 无 | 内容。 |
| `onRequestClose` | `() => void` | 无 | 用户请求关闭（Android 返回键、iOS 下滑 sheet）。请把 `visible` 设为 false。 |
| `onShow` | `() => void` | 无 | Modal 已显示。 |
| `onDismiss` | `() => void` | 无 | Modal 已关闭。 |

Modal 会把自己的安全区边距提供给其内容，因此其中的 [SafeAreaView](#safeareaview) 和 `safeAreaInsets()` 使用的是 Modal 的边距。设置 `transparent` 和 `animationType="fade"` 的 Modal 可以用作你自己设计的对话框或底部面板：

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

关于该元素的更多内容见 [Modal](../elements/modal.md)。

<a id="safe-areas"></a>

## 安全区

### SafeAreaView

`SafeAreaView` 通过添加内边距，让内容避开刘海、状态栏、Home 指示条、导航标题栏或 Mac Catalyst 的窗口标题栏。内边距取的是**视图所在位置**的 inset：在 [Screen](#screen) 内，是该页面标题栏下方的 inset；在 [Modal](#modal) 内，是模态框的 inset；其他情况下，是整个 surface 的安全区域。

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

inset 会**叠加**在视图自身的内边距上。在上面的例子里，内容从顶部 inset 再往下 16 pt 处开始，而背景色铺满整个区域，包括状态栏后面的部分。

默认四条边都会加内边距。传入 `edges` 可以只处理其中几条，比如底部 inset 已经由标签栏或你自己的底栏处理时：

```tsx
<SafeAreaView edges={["top", "left", "right"]} style={{ flex: 1 }}>
  …
</SafeAreaView>
```

标题栏可见且不透明的 [Screen](#screen) 已经把内容放在标题栏下方，所以那里的顶部 inset 为 0。SafeAreaView 用于：

- 没有导航栈的应用的根视图；
- 设置了 `headerShown={false}` 或 `headerTransparent` 的页面；
- 全屏 [Modal](#modal) 的内容。

| 属性 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `edges` | `Edge[] \| null`（`"top" \| "right" \| "bottom" \| "left"`） | 全部四条边 | 要加内边距的边。 |
| `children` | `JSX.Element` | 无 | 内容。 |
| `class`、`classList`、`style`、`testID`、`onLayout`、`ref` | | | 作用于该视图。其内边距会与 inset 相加。 |

<a id="safearea-and-safeareainsets"></a>

### safeArea 与 safeAreaInsets

需要具体数值时（比如自己给底栏加内边距），使用下面两个 accessor 之一。它们都返回 `Insets` 对象（`{ top, right, bottom, left }`，单位 pt），并且是响应式的：读取它们的代码会在值变化时（例如旋转屏幕）重新运行。

| 导出 | 返回值 |
|---|---|
| `safeArea()` | surface 的安全区域：避开系统 UI（刘海、Home 指示条、标题栏）的部分。 |
| `safeAreaInsets()` | 在组件中调用。返回该组件所在位置的 inset 的 accessor：最近的页面（标题栏下方）的、最近的模态框的，否则为 surface 的安全区域。与 SafeAreaView 使用的 inset 相同。 |

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

<a id="navigation-and-safe-areas-on-each-platform"></a>

### 各平台上的导航与安全区

- **iOS**：使用 `UINavigationController`，带原生标题栏、返回按钮和侧滑返回。`ios={{ headerLargeTitle }}` 和 `headerBackTitle` 仅在 iOS 上有效。在转场进行中到达的栈变更会在转场结束后应用。inset 来自 UIKit 的安全区域（状态栏、刘海或灵动岛、Home 指示条）。
- **Android**：带工具栏的页面栈。系统返回键会弹出栈顶页面；在支持的 Android 版本上使用预测性返回。如需自行处理返回键，见 [nsh/android](./nsh-android.md) 中的 `BackHandler`。NSH 报告系统栏和屏幕挖孔中与 NSH 视图重叠的部分；当 NSH [嵌入](../docs/embedding-android.md)在原生工具栏下方时，视图的顶部 inset 为 0。
- **Mac Catalyst**：与 iOS 相同。page sheet 和 form sheet 以与窗口同尺寸的 macOS sheet 形式呈现。顶部 inset 为窗口的标题栏。

<a id="dialogs"></a>

## 对话框

`Alert` 和 `ActionSheet` 显示平台自带的对话框：`Alert` 在 iOS 上是 `UIAlertController`，在 Android 上是 `AlertDialog`；`ActionSheet` 在 iOS 上是操作表，在 Android 上是列表对话框。它们都返回一个 Promise，解析为用户所选按钮或操作的序号。这些对话框使用平台自身的外观。

### Alert

```ts
Alert.alert(
  title: string,
  message?: string | null,
  buttons?: AlertButton[] | null,
  options?: AlertOptions | null,
): Promise<number>
```

Promise 解析为被按按钮在 `buttons` 中的序号；如果对话框未通过按钮而被关闭（Android，且设置了 `cancelable`），则解析为 `-1`。它总会被解析。

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

不传按钮时，警告框只有一个“OK”按钮：

```tsx
await Alert.alert("Saved", "Your changes were saved.");
```

#### AlertButton

| 字段 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `text` | `string \| null` | `"OK"` | 按钮文字。 |
| `style` | `"default" \| "cancel" \| "destructive"` | `"default"` | `cancel` 表示取消按钮；`destructive` 表示破坏性操作（iOS 上显示为红色）。 |

#### AlertOptions

| 字段 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `cancelable` | `boolean` | `false` | Android：点击对话框外部或按返回键会关闭对话框，Promise 解析为 `-1`。 |

> [!TIP]
> 主线程从不等待 JS，因此对话框以原生方式显示和应答，Promise 解析后你的代码继续执行。你可以在任何事件处理函数中 `await` 一个警告框，例如配合 [`preventBack`](#leaving-a-screen) 在离开页面前请用户确认。

### ActionSheet

```ts
ActionSheet.show(options: ActionSheetOptions): Promise<number>
```

Promise 解析为所选操作在 `options.options` 中的序号。如果用户没有选择就关闭了操作表（点击外部、按返回键），则解析为 `cancelButtonIndex`；操作表没有取消操作时解析为 `-1`。它总会被解析。

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

| 字段 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `options` | `string[]` | 必填 | 各操作的文字，按顺序排列。 |
| `cancelButtonIndex` | `number \| null` | 无 | 取消操作的序号。 |
| `destructiveButtonIndexes` | `number[] \| null` | 无 | 破坏性操作的序号（iOS 上显示为红色）。 |
| `title` | `string \| null` | 无 | 显示在操作上方。 |
| `message` | `string \| null` | 无 | 显示在标题下方。 |

<a id="dialogs-on-each-platform"></a>

### 各平台上的对话框

- **iOS**：警告框的按钮由 `UIAlertController` 布局。`style: "cancel"` 的按钮会放在 iOS 放置取消按钮的位置，可能与它在数组中的位置不同；返回的序号始终是它在 `buttons` 中的序号。操作表把取消操作单独显示在底部。在 iPad 和 Mac Catalyst 上操作表以 popover 形式出现：点击 popover 外部即可关闭，Promise 解析为 `cancelButtonIndex`（或 `-1`）。
- **Android**：`AlertDialog` 最多显示三个按钮，按位置映射：最后一个按钮是 positive 按钮，倒数第二个是 negative 按钮，倒数第三个是 neutral 按钮。操作表是对话框中的操作列表，点击外部或按返回键即为取消。列表对话框显示 `title`，没有标题时显示 `message`。按钮与操作的样式保持平台外观。

<a id="keyboard"></a>

## 键盘

`Keyboard` 用于收起软键盘，`keyboard()` 访问器告诉你键盘遮挡了多少屏幕，`KeyboardAvoidingView` 让内容保持在键盘上方并随键盘移动。

### Keyboard.dismiss

```ts
Keyboard.dismiss(): void
```

让当前获得焦点的输入框失去焦点，从而收起键盘。没有输入框获得焦点时，一切保持原样。

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
> 如果想在用户拖动滚动视图时收起键盘，在[滚动视图](../elements/scroll.md)或 [List](../elements/list.md) 上设置 `keyboardDismissMode="on-drag"` 或 `"interactive"`。

<a id="keyboard-height"></a>

### 键盘高度

```ts
keyboard: Accessor<{ height: number; duration: number }>
```

`keyboard()` 返回软键盘覆盖在 NSH 界面上的高度（单位为 pt，键盘隐藏时为 0），以及键盘弹出或收起动画的时长（单位为秒）。它是响应式的：读取它的 effect 或 JSX 表达式会在键盘弹出、收起或高度变化时重新运行。

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

这个高度是键盘与 NSH 界面重叠的部分。如果窗口本身会为键盘缩小（会调整窗口大小的 Android Activity），或者 NSH 嵌入的视图本来就在键盘上方结束，高度为 0。

> [!NOTE]
> 要让内容保持在键盘上方，使用 [KeyboardAvoidingView](#keyboardavoidingview) 通常比自己读取 `keyboard()` 更简单：它会以原生方式随键盘同步动画。

### KeyboardAvoidingView

`KeyboardAvoidingView` 让其内容保持在软键盘上方。键盘弹出或收起时，视图会让出与键盘等高的空间，并与键盘同步执行动画。它适用于滚动视图之外的内容，例如聊天界面底部的消息输入栏。[滚动视图](../elements/scroll.md)或 [List](../elements/list.md) 中的输入框会以原生方式滚动到可见区域。

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

该视图假定自己延伸到屏幕底部（屏幕内容通常如此），因此会让出完整的键盘高度。如果它下方还有其他内容（例如标签栏），可以用 `keyboardVerticalOffset` 传入负值减去这部分高度，或传入正值增加额外空间。

<a id="behaviors"></a>

#### 让位方式

| `behavior` | 如何让出空间 |
|---|---|
| `"padding"`（默认） | 把键盘高度加到视图的底部内边距上。上方内容随之缩小，背景仍延伸到屏幕底部。 |
| `"height"` | 把键盘高度作为底部外边距。整个视图（包括背景）在键盘顶部截止。 |
| `"position"` | 把内部的内容视图向上移动键盘的高度，并保持其尺寸。用 `contentContainerStyle` 设置这个内部视图的样式。 |

```tsx
<KeyboardAvoidingView behavior="position" contentContainerStyle={{ padding: 16, gap: 12 }} style={{ flex: 1 }}>
  <Input placeholder="E-mail" keyboardType="email-address" class="rounded-lg border border-neutral-300 px-3 py-2" />
  <View onPress={signIn} accessibilityRole="button" class="rounded-lg bg-blue-600 px-4 py-3">
    <Text class="text-center font-semibold text-white">Sign in</Text>
  </View>
</KeyboardAvoidingView>
```

这里的内边距或外边距会叠加在视图自身样式设置的值之上，你自己的 padding 保持不变。

<a id="keyboardavoidingview-props"></a>

#### KeyboardAvoidingView 属性

| 属性 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `enabled` | `boolean` | `true` | 是否避让键盘。为 false 时视图保持自身布局。 |
| `behavior` | `"padding" \| "height" \| "position"` | `"padding"` | 视图如何让出空间（见[让位方式](#behaviors)）。 |
| `keyboardVerticalOffset` | `number` | `0` | 键盘显示时加到键盘高度上的值（例如视图下方某个栏的高度）。总和至少为 0。 |
| `contentContainerStyle` | `StyleProp<ViewStyle>` | 无 | `behavior="position"` 时内部视图的样式。 |
| `children` | `JSX.Element` | 无 | 内容。 |
| `class`、`classList`、`style`、`testID`、`onLayout`、`ref` | | | 作用于外层视图。 |

<a id="the-keyboard-on-each-platform"></a>

### 各平台上的键盘

- **iOS**：让位动画使用键盘自身的时长和 ease-in-ease-out 曲线。
- **Android**：如果窗口会为键盘缩小（Activity 会调整窗口大小），内容本来就在键盘上方结束，KeyboardAvoidingView 保持原有布局。键盘遮挡视图时，它的表现与 iOS 相同。
- **桌面平台与 Mac Catalyst** 使用硬件键盘输入：`keyboard().height` 保持为 0，视图保持原有布局。

<a id="status-bar"></a>

## 状态栏

`StatusBar` 决定系统状态栏的外观：内容（时间、电量、信号）的颜色，以及状态栏是否隐藏。你可以把它当作组件渲染，让设置只在这部分 UI 显示期间生效；也可以调用它的静态函数。

### StatusBar

渲染出来的 `StatusBar` 不占任何空间，它的设置在挂载期间生效：

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

可以同时渲染多个 `StatusBar`，比如每个页面一个。**最后挂载**的那个生效，并且是**逐个属性**生效：只设置了 `hidden` 的 `StatusBar` 会保留下层那个的 `barStyle`。某个 `StatusBar` 卸载时（例如它所在的页面被弹出），之前的设置会重新生效。

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

<a id="statusbar-props"></a>

#### StatusBar 属性

| 属性 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `barStyle` | `"default" \| "light-content" \| "dark-content" \| null` | 不设置 | 内容的颜色。 |
| `hidden` | `boolean \| null` | 不设置 | 隐藏状态栏。 |
| `animated` | `boolean \| null` | 不设置 | 以动画呈现 `barStyle` 和 `hidden` 的变化。 |
| `android` | `{ backgroundColor?: string \| null }` | 不设置 | 仅 Android：状态栏的背景色。 |
| `web` | `{ themeColor?: string \| null }` | 不设置 | 仅浏览器：页面的主题色（`<meta name="theme-color">`）。 |

未设置的属性，会把状态栏的这一部分交给下层的 `StatusBar` 或函数决定。

<a id="bar-styles"></a>

#### 状态栏样式

| `barStyle` | 内容 |
|---|---|
| `"default"` | 平台针对当前外观（浅色或深色模式）的默认样式。 |
| `"light-content"` | 浅色内容，用于深色背景。 |
| `"dark-content"` | 深色内容，用于浅色背景。 |

<a id="setting-it-with-functions"></a>

#### 用函数设置

静态函数设置的是基础设置，任何渲染出来的 `StatusBar` 都会覆盖它。

```tsx
StatusBar.setBarStyle("dark-content", true);
StatusBar.setHidden(true, "fade");
StatusBar.setHidden(false);
```

| 函数 | 说明 |
|---|---|
| `StatusBar.setBarStyle(barStyle: BarStyle, animated?: boolean \| null): void` | 设置内容的颜色。`animated` 默认为 `false`。 |
| `StatusBar.setHidden(hidden: boolean, animation?: "none" \| "fade" \| "slide" \| null): void` | 隐藏或显示状态栏。除 `"none"` 以外的任何动画值都会让变化以动画呈现。 |

<a id="in-a-browser"></a>

#### 在浏览器中

浏览器会给页面周围的栏着色（手机上的地址栏、已安装 Web 应用的标题栏）。`web={{ themeColor }}` 以页面的 `<meta name="theme-color">` 设置它，同样是最近挂载的优先。

```tsx
<StatusBar barStyle="light-content" web={{ themeColor: "#0b0b0f" }} />
```

<a id="the-status-bar-on-each-platform"></a>

### 各平台上的状态栏

- **iOS**：隐藏和显示以淡入淡出呈现，`"slide"` 同样是淡入淡出。在[嵌入](../docs/embedding-ios.md)原生应用的场景下，状态栏的外观由宿主应用的视图控制器决定。宿主可以在其视图控制器中返回 `NshAppView.preferredStatusBarStyle`、`NshAppView.prefersStatusBarHidden` 和 `NshAppView.preferredStatusBarUpdateAnimation`，以应用 NSH 的设置。
- **Android**：`android={{ backgroundColor }}` 在 edge-to-edge 之前为状态栏着色（Android 15，即 API 35 起，应用会绘制到状态栏之下）。
- **桌面平台、Mac Catalyst 与 Web**：`barStyle` 和 `hidden` 作用于平台的状态栏；Web 使用 `web={{ themeColor }}`。

> [!IMPORTANT]
> 如果你的 `nsh.json` 声明了 `"capabilities"`，只有其中包含 `"statusBar": true` 时状态栏才会改变，其他更改会被拒绝并记录日志。参见[沙箱与权限](../docs/sandbox-and-capabilities.md)。

```json
{
  "capabilities": {
    "statusBar": true
  }
}
```

<a id="related"></a>

## 相关内容

- [导航指南](../docs/navigation.md)。
- [表单与键盘](../docs/forms-and-keyboard.md)。
- [Modal](../elements/modal.md) 与 [Input](../elements/input.md)。
- [设备与环境](../docs/device-and-environment.md)。
- [nsh/android](./nsh-android.md)：`BackHandler`、用于简短消息的 `Toast.show`、`setStatusBarBackgroundColor` 及其他 Android 专用 API。
