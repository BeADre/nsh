---
title: Desktop UI
description: nsh/ui/desktop UI 集：NSH 的设计语言在指针和键盘下的密度，以及它的组件。
order: 20
---

# Desktop UI

`nsh/ui/desktop` 是 NSH 面向 macOS、Windows、Linux 和 Web 窗口的 UI 集。它和 [Mobile UI](./index.md) 使用同一套设计语言、同一个主题，
只是密度适合桌面：控件更矮，指针悬停时高亮，键盘把焦点移到控件上时显示焦点环。

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

## Select

`Select` 在一个字段里显示 `options` 中的某一项；按下它会在下方（窗口放不下时在上方）的浮层里展开列表，从中选择另一项。
列表至少和字段一样宽，当前选中项带有对勾。传入 `value` 时由应用控制选中项，并在 `onChange` 里更新；不传时组件自己保存，初始为 `defaultValue`。

```tsx
import { mount, signal, View } from "nsh";
import { Select } from "nsh/ui/desktop";

function App() {
  const theme = signal<number | null>(null);
  return (
    <View class="w-56 p-6">
      <Select
        options={["Light", "Dark", "System"]}
        value={theme()}
        onChange={(i) => theme.set(i)}
        placeholder="Appearance"
      />
    </View>
  );
}

mount(App);
```

字段的宽度由父容器决定，和普通视图一样。在 macOS 和 Web 上，Tab 可以聚焦到它，按 Return 展开列表，Tab 在选项之间移动，
按 Return 或空格选中一项，焦点随后回到字段上。

| 属性 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `options` | `string[]` | 必填 | 选项。 |
| `value` | `number \| null` | | 应用控制时，选中项的索引。 |
| `defaultValue` | `number \| null` | 无 | 应用不控制时的初始选中项。 |
| `placeholder` | `string` | | 未选中时显示的文字。 |
| `size` | `"sm" \| "md" \| "lg"` | `"md"` | 字段和选项的高度与字号。 |
| `disabled` | `boolean` | `false` | 不能展开，变暗显示。 |

| 事件 | 参数 | 触发时机 |
|---|---|---|
| `onChange` | `number`：选项的索引 | 选中了一项。 |

## Checkbox

`Checkbox` 是一个带勾选标记的小方框，旁边可以有一个 `label`。点击方框或标签，或者在它获得键盘焦点时按 Return 或空格，
会勾选或取消勾选。传入 `value` 时由应用控制状态，并在 `onChange` 里更新 `value`；传入 `defaultValue`（或两者都不传）时，复选框自己保存状态。

```tsx
import { mount, signal, View } from "nsh";
import { Checkbox } from "nsh/ui/desktop";

function App() {
  const remember = signal(false);
  return (
    <View class="gap-3 p-6">
      <Checkbox label="Remember me" value={remember()} onChange={(v) => remember.set(v)} />
      <Checkbox label="Send a copy" defaultValue />
      <Checkbox label="Dense" size="sm" />
    </View>
  );
}

mount(App);
```

`sm`、`md`、`lg` 的方框边长分别是 14、16、20 pt，圆角按控件的形状等比缩小，标签使用控件的字号。勾选时方框填充
`primary.background`，勾选标记为 `primary.foreground`；未勾选时为 `background`，边框为 `input.border`。指针悬停时，
已勾选的方框稍微变淡，未勾选的方框显示 `accent.background`；按下时显示 `primary.active.background` 或
`secondary.active.background`。焦点环围绕方框和标签。屏幕阅读器把它读作复选框，并读出勾选状态。

| 属性 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `value` | `boolean \| null` | | 应用控制状态时，是否勾选。 |
| `defaultValue` | `boolean` | `false` | 应用不控制状态时，初始是否勾选。 |
| `label` | `string \| null` | | 显示在方框旁边；点击它同样会切换方框。 |
| `size` | `"sm" \| "md" \| "lg"` | `"md"` | 方框大小和标签字号。 |
| `disabled` | `boolean` | `false` | 不响应按压，没有反馈，50% 不透明度。 |
| `accessibilityLabel` | `string \| null` | `label` | 当 `label` 不够或没有标签时，屏幕阅读器读出的内容。 |

| 事件 | 参数 | 触发时机 |
|---|---|---|
| `onChange` | `boolean` | 点击，或获得键盘焦点时按 Return 或空格：新的勾选状态。 |

## Switch

`Switch` 是一条轨道和一个圆形滑块，打开或关闭时滑块以弹簧动效移到另一端，旁边可以有一个 `label`。点击开关或标签，
或者在它获得键盘焦点时按 Return 或空格，会切换它。`value`、`defaultValue` 和 `onChange` 的用法与 `Checkbox` 相同。

```tsx
import { mount, signal, View } from "nsh";
import { Switch } from "nsh/ui/desktop";

function App() {
  const wifi = signal(true);
  return (
    <View class="gap-3 p-6">
      <Switch label="Wi-Fi" value={wifi()} onChange={(v) => wifi.set(v)} />
      <Switch label="Bluetooth" />
      <Switch label="Compact" size="sm" defaultValue />
    </View>
  );
}

mount(App);
```

`sm`、`md`、`lg` 的轨道尺寸分别是 32 × 18、36 × 20、44 × 24 pt。关闭时轨道为 `switch.background`，打开时为
`primary.background`，颜色随滑块移动逐渐过渡，使用 NSH 统一的落定动效。滑块为 `switch.thumb.background`，打开时逐渐过渡到
`primary.foreground`，因此在浅色和深色主题下都能在打开的颜色上清晰显示。指针悬停时轨道稍微变淡，打开状态下按住时显示
`primary.active.background`。焦点环围绕开关和标签。屏幕阅读器把它读作开关，并读出开关状态。

| 属性 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `value` | `boolean \| null` | | 应用控制状态时，是否打开。 |
| `defaultValue` | `boolean` | `false` | 应用不控制状态时，初始是否打开。 |
| `label` | `string \| null` | | 显示在开关旁边；点击它同样会切换开关。 |
| `size` | `"sm" \| "md" \| "lg"` | `"md"` | 轨道大小和标签字号。 |
| `disabled` | `boolean` | `false` | 不响应按压，没有反馈，50% 不透明度。 |
| `accessibilityLabel` | `string \| null` | `label` | 当 `label` 不够或没有标签时，屏幕阅读器读出的内容。 |

| 事件 | 参数 | 触发时机 |
|---|---|---|
| `onChange` | `boolean` | 点击，或获得键盘焦点时按 Return 或空格：新的开关状态。 |

## TextField

`TextField` 是单行文本输入框，可以在上方显示 `label`。边框使用主题的 `input.border`，指针悬停时稍微加深；输入焦点在它上面时
（无论是点击还是 Tab 聚焦）显示焦点环（主题的 `ring`，宽 2 pt，间距 2 pt）。按 Return 会触发 `onSubmit`，焦点留在输入框里。
传入 `value` 时由应用控制文字，并在 `onChange` 里更新它；传入 `defaultValue`（或都不传）时由输入框自己保存文字。

```tsx
import { mount, signal, Text, View } from "nsh";
import { TextField } from "nsh/ui/desktop";

function App() {
  const name = signal("");
  return (
    <View class="w-72 gap-3 p-6">
      <TextField label="Name" placeholder="Your name" value={name()} onChange={(t) => name.set(t)} />
      <TextField label="Password" secure size="sm" />
      <Text>{`Hello, ${name()}`}</Text>
    </View>
  );
}

mount(App);
```

输入框的高度由 `size` 决定（24、32 或 40 pt），宽度由父视图决定（用 `class="w-64"` 设置宽度）。标签使用主题的
`muted.foreground`，文字使用 `foreground`，占位文字使用 `muted.foreground`，输入框背景使用 `background`。

| 属性 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `label` | `string \| null` | | 显示在输入框上方。 |
| `placeholder` | `string \| null` | | 内容为空时显示。 |
| `value` | `string \| null` | | 由应用控制时的文字。 |
| `defaultValue` | `string` | `""` | 由输入框自己保存时的初始文字。 |
| `secure` | `boolean` | `false` | 隐藏输入的内容（密码）。 |
| `size` | `"sm" \| "md" \| "lg"` | `"md"` | 高度、内边距、字号和圆角。 |
| `disabled` | `boolean` | `false` | 不可编辑，50% 不透明度。 |
| `class`、`classList`、`style`、`testID`、`onLayout`、`ref` | | | 作用于输入框的根视图（标签和输入框）。 |

| 事件 | 参数 | 触发时机 |
|---|---|---|
| `onChange` | `string`：新的文字 | 文字改变。 |
| `onSubmit` | `string`：当前文字 | 按下 Return。 |

## NumberInput

`NumberInput` 是一个数字输入框，两侧是 − 和 + 按钮。点击按钮按 `step` 增减数值，按住按钮会持续步进，直到松开或数值到达
`min` 或 `max`；到达边界的按钮显示为淡化状态。输入的文字在按 Return 或输入框失去焦点时读取：按 `step` 的小数位取整，限制在
`min` 和 `max` 之间，并以同样的小数位显示（步长为 0.25 时显示 `0.50`）。不是数字的文字会恢复为原来的数值。Tab 停在输入框上，
两侧按钮供指针使用。

```tsx
import { mount, signal, Text, View } from "nsh";
import { NumberInput } from "nsh/ui/desktop";

function App() {
  const quantity = signal(1);
  return (
    <View class="items-start gap-3 p-6">
      <NumberInput value={quantity()} onChange={(v) => quantity.set(v)} min={1} max={99} />
      <NumberInput defaultValue={0.5} step={0.25} max={2} size="sm" />
      <Text>{`Quantity: ${quantity()}`}</Text>
    </View>
  );
}

mount(App);
```

它的外观和 `TextField` 一致（边框、悬停、焦点环）；按钮在指针悬停时显示主题的 `accent.background`，按下时显示
`secondary.active.background`。

| 属性 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `value` | `number \| null` | | 由应用控制时的数值。 |
| `defaultValue` | `number` | `0` | 由输入框自己保存时的初始数值。 |
| `min` | `number` | `0` | 最小值。 |
| `max` | `number` | `100` | 最大值。 |
| `step` | `number` | `1` | 每次按下按钮改变的量；数值使用它的小数位。 |
| `size` | `"sm" \| "md" \| "lg"` | `"md"` | 高度、字号和圆角。 |
| `disabled` | `boolean` | `false` | 不可编辑，不响应按压，50% 不透明度。 |
| `class`、`classList`、`style`、`testID`、`onLayout`、`ref` | | | 作用于输入框的根视图。 |

| 事件 | 参数 | 触发时机 |
|---|---|---|
| `onChange` | `number`：新的数值 | 按钮步进了数值，或读取的输入与当前数值不同。 |

## Tabs

`Tabs` 是一行标签页，其中一个处于选中状态。`underline` 变体在选中的标签下画一条 2 pt 的 `primary.background` 线，
压在横贯整行的 `border` 分隔线上；`segmented` 变体把标签放进 `tab_bar.segmented.background` 的轨道里，所有标签和最宽的一样宽，
选中的标签浮起在 `tab.active.background` 上。下划线或浮起的背景会以 NSH 的落定弹簧滑到新选中的标签，并在滑动中变成它的宽度。

```tsx
import { mount, signal, Text, View } from "nsh";
import { Tabs } from "nsh/ui/desktop";

function App() {
  const tab = signal(0);
  return (
    <View class="gap-4 p-6">
      <Tabs options={["Overview", "Activity", "Settings"]} value={tab()} onChange={(i) => tab.set(i)} />
      <Text>{["Summary", "Recent events", "Preferences"][tab()]}</Text>
      <Tabs variant="segmented" options={["Day", "Week", "Month"]} defaultValue={1} />
    </View>
  );
}

mount(App);
```

标签文字使用 `tab.foreground`，选中的标签和指针下的标签使用 `tab.active.foreground`；其他标签在指针下显示 `accent.background`。
在 macOS 和 Web 上，每个标签都是键盘焦点的一站：Tab 在标签之间移动，按 Return 或空格选中获得焦点的标签，它显示焦点环。
屏幕阅读器把这一行读作标签列表，并读出每个标签的选中状态。分段样式的一行和它的标签一样宽；加上 `class="self-stretch"`
会铺满容器，标签平分宽度。

| 属性 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `options` | `string[]` | 必填 | 各标签的文字。 |
| `value` | `number \| null` | 无（非受控） | 由应用控制时，选中标签的索引（在 `onChange` 里更新它）。 |
| `defaultValue` | `number` | `0` | 由组件自己保存选中状态时，最初选中标签的索引。 |
| `variant` | `"underline" \| "segmented"` | `"underline"` | 选中标签下画一条线，或把标签放进轨道并浮起选中的那个。 |
| `size` | `"sm" \| "md" \| "lg"` | `"md"` | 高度、内边距和字号。 |
| `disabled` | `boolean` | `false` | 不响应按压，50% 不透明度。 |
| `class`、`classList`、`style`、`testID`、`onLayout`、`ref` | | | 作用于这一行。 |

| 事件 | 参数 | 触发时机 |
|---|---|---|
| `onChange` | `number`：标签的索引 | 点击另一个标签，或另一个标签获得键盘焦点时按 Return 或空格。 |

## Badge

`Badge` 是一个小的圆角标签，用于状态、分类或计数，高 20 pt，字号比主题字号小 4 pt。

```tsx
import { View } from "nsh";
import { Badge } from "nsh/ui/desktop";

<View class="flex-row items-center gap-2">
  <Badge label="Draft" />
  <Badge label="New" variant="primary" />
  <Badge label="Failed" variant="danger" />
  <Badge label="v2.1" variant="outline" />
</View>;
```

| 变体 | 背景 | 文字 |
|---|---|---|
| `secondary`（默认） | `secondary.background` | `secondary.foreground` |
| `primary` | `primary.background` | `primary.foreground` |
| `danger` | `danger.background` | `danger.foreground` |
| `outline` | 透明，带 `border` 颜色的边框 | `foreground` |

| 属性 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `label` | `string` | 必填 | 文字。 |
| `variant` | `"secondary" \| "primary" \| "danger" \| "outline"` | `"secondary"` | 外观。 |
| `class`、`classList`、`style`、`testID`、`onLayout`、`ref` | | | 作用于徽标。 |

## Kbd

`Kbd` 在一个按键形状的小块里显示键盘快捷键，写法和应用所在平台的菜单一致：`keys="mod+k"` 在 macOS 和 Mac 上的浏览器里显示为 `⌘K`，
在 Windows、Linux 和其他浏览器里显示为 `Ctrl+K`。它接受 `nsh/keymap` 的按键写法（`"mod+k"`、`"shift+mod+p"`、`"escape"`），
所以提示显示的正是 `keymap.bind` 绑定的快捷键；其他文字按原样显示。它高 20 pt，背景是 `muted.background`，带 `border` 颜色的边框，
文字使用 `muted.foreground`。

```tsx
import { Text, View } from "nsh";
import { Kbd } from "nsh/ui/desktop";

<View class="flex-row items-center gap-2">
  <Text>Search</Text>
  <Kbd keys="mod+k" />
</View>;
```

| 属性 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `keys` | `string` | 必填 | 按键，按 `nsh/keymap` 的写法（`mod` 在 Apple 平台上是 Command，在其他平台上是 Control）。 |
| `class`、`classList`、`style`、`testID`、`onLayout`、`ref` | | | 作用于这个小块。 |

## DropdownMenu

`open` 为 true 时，`DropdownMenu` 在 `trigger` 下方的浮层里列出操作。`items` 与 `nsh/desktop` 的 `Menu.popup` 的条目写法相同：
带 `label` 的菜单项，`shortcut` 按平台的菜单写法显示在末尾（`"mod+r"` 在 macOS 上显示为 `⌘R`，其他平台为 `Ctrl+R`），`enabled`，
`checked` 显示对勾，`danger` 用危险色表示破坏性操作；或者 `{ type: "separator" }` 分隔线。选中一项会先报告 `onSelect`（参数为它的索引），
再报告 `onClose`；点击菜单外部或按 Escape 会报告 `onClose`。在回调里把 `open` 设为 false。

```tsx
import { mount, signal, View } from "nsh";
import { Button, DropdownMenu } from "nsh/ui/desktop";

function App() {
  const open = signal(false);
  const items = [
    { label: "Rename", shortcut: "mod+r" },
    { label: "Duplicate", shortcut: "shift+mod+d" },
    { type: "separator" as const },
    { label: "Delete", shortcut: "mod+backspace", danger: true },
  ];
  return (
    <View class="flex-row p-6">
      <DropdownMenu
        open={open()}
        onClose={() => open.set(false)}
        onSelect={(i) => console.log(items[i])}
        items={items}
        trigger={<Button title="Actions" variant="secondary" onPress={() => open.set(true)} />}
      />
    </View>
  );
}

mount(App);
```

快捷键只负责显示，绑定请用 `nsh/keymap`。在 macOS 和 Web 上，Tab 在可用的菜单项之间移动，按 Return 或空格选中一项，焦点随后回到触发按钮。

| 属性 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `open` | `boolean` | `false` | 是否显示。 |
| `trigger` | JSX | | 打开它的元素，菜单放在它下方。 |
| `items` | `MenuEntry[]` | 必填 | 条目：`{ label, shortcut?, enabled?, checked?, danger? }` 或 `{ type: "separator" }`。 |
| `size` | `"sm" \| "md" \| "lg"` | `"md"` | 菜单项的高度和字号。 |

| 事件 | 参数 | 触发时机 |
|---|---|---|
| `onSelect` | `number`：菜单项在 `items` 中的索引 | 选中了一项。 |
| `onClose` | 无 | 选中了一项、点击菜单外部，或按 Escape。 |
