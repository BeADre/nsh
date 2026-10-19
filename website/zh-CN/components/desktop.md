---
title: Desktop UI
description: nsh/ui/desktop UI 集：NSH 的设计语言在指针和键盘下的密度，以及它的组件。
order: 20
---

# Desktop UI

`nsh/ui/desktop` 是 NSH 面向 macOS、Windows、Linux 和 Web 窗口的 UI 集。它和 [Mobile UI](./index.md) 使用同一套设计语言、同一个主题，
只是密度适合桌面：控件更矮，指针悬停时高亮，键盘把焦点移到控件上时显示焦点环。第一批组件是 `Button`、`Popover` 和 `Dialog`，后续组件陆续加入。

```tsx
import { Button } from "nsh/ui/desktop";
```

这些组件的工作方式和移动端组件一样：用 Rust 编写，由原生核心用宿主元素渲染，类型从源码生成（`.nsh/types/nsh/ui/desktop.d.ts`），
主题来自 `nsh.json` 的 `"theme"`。一个应用可以同时使用两套组件，用不同的名字导入：

```tsx
import { Button } from "nsh/ui/desktop";
import { Button as MobileButton } from "nsh/ui/mobile";
```

## 尺寸

每个控件都有 `size`：`sm`（高 24 pt，用于工具栏和紧凑表格）、`md`（32 pt，默认）和 `lg`（40 pt，用于窗口的主要操作）。
`lg` 的文字使用主题字号，较紧凑的尺寸略小。圆角跟随主题的 `radius`，并按控件高度等比缩放，所以桌面控件和移动端控件形状一致。

## 指针和键盘

- **悬停**：有底色的控件在指针下稍微变淡；`plain` 控件显示主题的 `accent.background`。
- **按下**：点击会按下控件；按下期间显示主题的 `*.active.background`。
- **焦点**：在 macOS 和 Web 上，Tab 在控件之间移动键盘焦点，按 Return 或空格会按下获得焦点的控件，它显示主题 `ring` 颜色的焦点环，
  宽 2 pt，间距 2 pt。点击会按下控件但不让它获得焦点，所以焦点环只在键盘聚焦时显示。

## Button

```tsx
import { mount, signal, Text, View } from "nsh";
import { Button } from "nsh/ui/desktop";

function App() {
  const saved = signal(0);
  return (
    <View class="flex-row items-center gap-3 p-6">
      <Button title="Save" onPress={() => saved.update((n) => n + 1)} />
      <Button title="Cancel" variant="secondary" />
      <Button title="Delete" variant="danger" size="sm" />
      <Text>{`Saved ${saved()} times`}</Text>
    </View>
  );
}

mount(App);
```

| 变体 | 背景 | 悬停 | 按下 | 文字 |
|---|---|---|---|---|
| `primary`（默认） | `primary.background` | `primary.background`，变淡 | `primary.active.background` | `primary.foreground` |
| `secondary` | `secondary.background` | `secondary.background`，变淡 | `secondary.active.background` | `secondary.foreground` |
| `danger` | `danger.background` | `danger.background`，变淡 | `danger.active.background` | `danger.foreground` |
| `plain` | 透明 | `accent.background` | `secondary.active.background` | `foreground` |

| 属性 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `title` | `string` | 必填 | 文字标签。 |
| `variant` | `"primary" \| "secondary" \| "danger" \| "plain"` | `"primary"` | 外观。 |
| `size` | `"sm" \| "md" \| "lg"` | `"md"` | 高度、内边距和字号。 |
| `disabled` | `boolean` | `false` | 不响应按压，没有反馈，50% 不透明度。 |
| `accessibilityLabel` | `string \| null` | `title` | 当 `title` 不够时，屏幕阅读器读出的内容。 |
| `class`、`classList`、`style`、`testID`、`onLayout`、`ref` | | | 作用于按钮的根视图。`class` 和 `style` 会覆盖按钮自身的样式。 |

| 事件 | 参数 | 触发时机 |
|---|---|---|
| `onPress` | 无 | 点击，或按钮获得键盘焦点时按 Return 或空格（macOS、Web）。 |

## Popover

`open` 为 true 时，`Popover` 在 `trigger` 旁边显示一个浮层：默认在下方；下方放不下时放到上方（或另一侧），并且始终保持在窗口内。
点击浮层外部或按 Escape 会通过 `onClose` 关闭它，在回调里把 `open` 设为 false。每次打开时都会重新创建内容。

```tsx
import { mount, signal, View } from "nsh";
import { Button, Popover } from "nsh/ui/desktop";

function App() {
  const open = signal(false);
  return (
    <View class="p-6">
      <Popover
        open={open()}
        onClose={() => open.set(false)}
        trigger={<Button title="Options" onPress={() => open.set(true)} />}
      >
        <View class="w-40 gap-1">
          <Button title="Rename" variant="plain" onPress={() => open.set(false)} />
          <Button title="Delete" variant="danger" onPress={() => open.set(false)} />
        </View>
      </Popover>
    </View>
  );
}

mount(App);
```

浮层使用主题的 `popover.background`、`popover.foreground` 和 `border` 颜色，淡入和淡出分别与 NSH 其他浮层的进入、离开动效一致。
在 macOS 和 Web 上，Tab 会从触发按钮进入浮层内容，浮层关闭后键盘焦点回到触发按钮。

| 属性 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `open` | `boolean` | `false` | 是否显示。 |
| `trigger` | JSX | | 打开它的元素，浮层放在它旁边。 |
| `side` | `"top" \| "bottom" \| "left" \| "right"` | `"bottom"` | 放在触发元素的哪一侧。 |
| `align` | `"start" \| "center" \| "end"` | `"start"` | 沿该侧与触发元素如何对齐。 |
| `children` | JSX | | 内容，浮层打开时创建。 |

| 事件 | 参数 | 触发时机 |
|---|---|---|
| `onClose` | 无 | 点击浮层外部，或按 Escape。 |

## Dialog

`open` 为 true 时，`Dialog` 在窗口中央、变暗的背景之上显示一个面板：标题、说明、你的内容，以及 `footer` 里的操作按钮。
点击背景或按 Escape 会通过 `onClose` 关闭它，在回调里把 `open` 设为 false。打开时它会轻微上浮并淡入，关闭时下沉并淡出。每次打开时都会重新创建内容。

```tsx
import { mount, signal, View } from "nsh";
import { Button, Dialog } from "nsh/ui/desktop";

function App() {
  const open = signal(false);
  const close = () => open.set(false);
  return (
    <View class="p-6">
      <Button title="Delete project…" variant="danger" onPress={() => open.set(true)} />
      <Dialog
        open={open()}
        onClose={close}
        title="Delete “Website”?"
        description="The project, its files and its history are deleted for everyone."
        footer={
          <View class="flex-row gap-2">
            <Button title="Cancel" variant="secondary" onPress={close} />
            <Button title="Delete" variant="danger" onPress={close} />
          </View>
        }
      />
    </View>
  );
}

mount(App);
```

在 macOS 和 Web 上，Tab 只在对话框内的控件之间移动，对话框关闭后键盘焦点回到打开它之前所在的位置。

| 属性 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `open` | `boolean` | `false` | 是否显示。 |
| `title` | `string` | | 顶部的粗体标题。 |
| `description` | `string` | | 标题下方的弱化说明文字。 |
| `footer` | JSX | | 操作按钮，放在最后一行的末端（主要操作放最后）。 |
| `width` | `number` | `440` | 面板宽度（点）；窗口更窄时随之变窄。 |
| `children` | JSX | | 说明和操作按钮之间的内容，对话框打开时创建。 |

| 事件 | 参数 | 触发时机 |
|---|---|---|
| `onClose` | 无 | 点击背景，或按 Escape。 |
