---
title: Desktop UI
description: The nsh/ui/desktop UI set, NSH's design language at a pointer's and a keyboard's density, and its components.
order: 20
---

# Desktop UI

`nsh/ui/desktop` is NSH's UI set for windows on macOS, Windows, Linux and the web. It draws the same design language
as the [mobile UI](./index.md), with the same theme, at a desktop's density: shorter controls, a highlight under the
pointer, and a focus ring when the keyboard moves to a control.

```tsx
import { Button } from "nsh/ui/desktop";
```

The components work as the mobile ones do: they are written in Rust, rendered by the native core from host elements,
typed from their source (`.nsh/types/nsh/ui/desktop.d.ts`), and themed from `nsh.json`'s `"theme"`. An app may use
both sets, under different names:

```tsx
import { Button } from "nsh/ui/desktop";
import { Button as MobileButton } from "nsh/ui/mobile";
```

## Sizes

Every control takes a `size`: `sm` (24 pt tall, for toolbars and dense tables), `md` (32 pt, the default) or `lg`
(40 pt, for a window's main action). Text is the theme's font size for `lg` and slightly smaller for the denser
sizes. Corners follow the theme's `radius`, scaled with the control's height, so a desktop control has the shape of a
mobile one.

## Pointer and keyboard

- **Hover**: a filled control fades a little under the pointer; a `plain` one gets the theme's `accent.background`.
- **Press**: a click presses a control; while pressed it shows the theme's `*.active.background`.
- **Focus**: on macOS and the web, Tab moves the keyboard focus from control to control, Return or Space presses the
  focused one, and it shows a ring in the theme's `ring` color, 2 pt wide, 2 pt away. A click presses a control
  without focusing it, so the ring shows for keyboard focus only.

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

| Variant | Background | Hovered | Pressed | Label |
|---|---|---|---|---|
| `primary` (default) | `primary.background` | `primary.background`, faded | `primary.active.background` | `primary.foreground` |
| `secondary` | `secondary.background` | `secondary.background`, faded | `secondary.active.background` | `secondary.foreground` |
| `danger` | `danger.background` | `danger.background`, faded | `danger.active.background` | `danger.foreground` |
| `plain` | transparent | `accent.background` | `secondary.active.background` | `foreground` |

| Prop | Type | Default | Description |
|---|---|---|---|
| `title` | `string` | required | The label. |
| `variant` | `"primary" \| "secondary" \| "danger" \| "plain"` | `"primary"` | The look. |
| `size` | `"sm" \| "md" \| "lg"` | `"md"` | The height, padding and text size. |
| `disabled` | `boolean` | `false` | No presses, no feedback, 50% opacity. |
| `accessibilityLabel` | `string \| null` | `title` | What screen readers read, when `title` is not enough. |
| `class`, `classList`, `style`, `testID`, `onLayout`, `ref` | | | Applied to the button's root view. `class` and `style` override the button's own styles. |

| Event | Payload | When |
|---|---|---|
| `onPress` | none | A click, or Return or Space while the button has the keyboard focus (macOS, the web). |

## Popover

`Popover` shows a floating surface next to its `trigger` while `open` is true: below it by default, above it (or on
the other side) when the window has no room there, and always inside the window. A click outside the surface or
Escape closes it through `onClose`; set `open` to false there. The content is made each time the popover opens.

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

The surface uses the theme's `popover.background`, `popover.foreground` and `border` colors, fades in as NSH's other
surfaces move in and fades out as they leave. On macOS and the web, Tab goes from the trigger into the content and the keyboard focus
returns to the trigger when the popover closes.

| Prop | Type | Default | Description |
|---|---|---|---|
| `open` | `boolean` | `false` | Shown. |
| `trigger` | JSX | | What opens it: the surface goes next to it. |
| `side` | `"top" \| "bottom" \| "left" \| "right"` | `"bottom"` | Which side of the trigger. |
| `align` | `"start" \| "center" \| "end"` | `"start"` | How it lines up with the trigger along that side. |
| `children` | JSX | | The content, made when the popover opens. |

| Event | Payload | When |
|---|---|---|
| `onClose` | none | A click outside the surface, or Escape. |

## Dialog

`Dialog` shows a surface in the middle of the window over a dimmed backdrop while `open` is true: a title, a
description, your content and the actions in `footer`. A click on the backdrop or Escape closes it through `onClose`;
set `open` to false there. It rises a little and fades in as it opens, and sinks and fades out as it closes. The
content is made each time it opens.

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

On macOS and the web, Tab moves between the dialog's controls only, and the keyboard focus returns to what had it
when the dialog closes.

| Prop | Type | Default | Description |
|---|---|---|---|
| `open` | `boolean` | `false` | Shown. |
| `title` | `string` | | At the top, in bold. |
| `description` | `string` | | Below the title, muted. |
| `footer` | JSX | | The actions, at the end of the last row (the main one last). |
| `width` | `number` | `440` | The surface's width in points; a narrower window narrows it. |
| `children` | JSX | | The content between the description and the actions, made when the dialog opens. |

| Event | Payload | When |
|---|---|---|
| `onClose` | none | A click on the backdrop, or Escape. |

## Select

`Select` shows one of `options` in a field; pressing it opens the list in a popover below it (above, when the window
has no room) to pick another. The list is at least as wide as the field, with a check mark on the current choice.
With `value` the app controls the choice and updates it from `onChange`; without, the select keeps its own, starting
at `defaultValue`.

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

The field takes the width its parent gives it, as a view does. On macOS and the web, Tab reaches it and Return opens
the list, Tab moves through the options, Return or Space picks one, and the focus returns to the field.

| Prop | Type | Default | Description |
|---|---|---|---|
| `options` | `string[]` | required | The options. |
| `value` | `number \| null` | | The index of the chosen option, when the app controls it. |
| `defaultValue` | `number \| null` | none | The first choice, when the app does not control it. |
| `placeholder` | `string` | | Shown while nothing is chosen. |
| `size` | `"sm" \| "md" \| "lg"` | `"md"` | The field's and the options' height and text size. |
| `disabled` | `boolean` | `false` | Not opened, dimmed. |

| Event | Payload | When |
|---|---|---|
| `onChange` | `number`: the option's index | An option was picked. |

## Checkbox

`Checkbox` is a small square box with a check mark, and an optional `label` beside it. A click on the box or the label,
or Return or Space while it has the keyboard focus, checks or unchecks it. With `value` the app controls it and updates
`value` from `onChange`; with `defaultValue` (or neither) the checkbox keeps its own state.

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

The box is 14, 16 or 20 pt for `sm`, `md` and `lg`, with the control's corner shape scaled down to it, and the label
uses the control's text size. Checked, it is filled with `primary.background` and the mark is `primary.foreground`;
unchecked, it is `background` with an `input.border` outline. Under the pointer a checked box fades a little and an
unchecked one shows `accent.background`; pressed, it shows `primary.active.background` or
`secondary.active.background`. The focus ring goes around the box and its label. Screen readers read it as a checkbox
with its checked state.

| Prop | Type | Default | Description |
|---|---|---|---|
| `value` | `boolean \| null` | | Checked, when the app controls it. |
| `defaultValue` | `boolean` | `false` | Checked at first, when the app does not control it. |
| `label` | `string \| null` | | Beside the box; a click on it toggles the box too. |
| `size` | `"sm" \| "md" \| "lg"` | `"md"` | The box and the label's text size. |
| `disabled` | `boolean` | `false` | No presses, no feedback, 50% opacity. |
| `accessibilityLabel` | `string \| null` | `label` | What screen readers read, when `label` is not enough or there is none. |

| Event | Payload | When |
|---|---|---|
| `onChange` | `boolean` | A click, or Return or Space with the keyboard focus: the new checked state. |

## Switch

`Switch` is a track with a round thumb that springs across when it turns on or off, with an optional `label` beside
it. A click on the switch or the label, or Return or Space while it has the keyboard focus, flips it. `value`,
`defaultValue` and `onChange` work as `Checkbox`'s do.

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

The track is 32 × 18, 36 × 20 or 44 × 24 pt for `sm`, `md` and `lg`. Off, it is `switch.background`; on, it is
`primary.background`, and the color fades from one to the other as the thumb moves, with NSH's settling motion. The
thumb is `switch.thumb.background`, blending to `primary.foreground` as it turns on, so it stands out on the on color
in light and dark themes alike. The track fades a little under the pointer and shows `primary.active.background`
while pressed when on. The focus ring goes around the switch and its label. Screen readers read it as a switch with
its state.

| Prop | Type | Default | Description |
|---|---|---|---|
| `value` | `boolean \| null` | | On, when the app controls it. |
| `defaultValue` | `boolean` | `false` | On at first, when the app does not control it. |
| `label` | `string \| null` | | Beside the switch; a click on it flips the switch too. |
| `size` | `"sm" \| "md" \| "lg"` | `"md"` | The track and the label's text size. |
| `disabled` | `boolean` | `false` | No presses, no feedback, 50% opacity. |
| `accessibilityLabel` | `string \| null` | `label` | What screen readers read, when `label` is not enough or there is none. |

| Event | Payload | When |
|---|---|---|
| `onChange` | `boolean` | A click, or Return or Space with the keyboard focus: the new state. |

## TextField

`TextField` is a one-line text input with an optional `label` above it. Its border is the theme's `input.border`,
darker under the pointer, and it shows the focus ring (the theme's `ring`, 2 pt wide, 2 pt away) while typing goes
there, whether the field was focused by a click or by Tab. Return reports `onSubmit` and keeps the focus in the field.
With `value` the app controls the text and updates it from `onChange`; with `defaultValue` (or neither) the field
keeps its own.

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

The field is as tall as its `size` (24, 32 or 40 pt) and as wide as its parent gives it (`class="w-64"` sets a
width). The label uses the theme's `muted.foreground`, the text `foreground`, the placeholder `muted.foreground` and
the field's background `background`.

| Prop | Type | Default | Description |
|---|---|---|---|
| `label` | `string \| null` | | Shown above the field. |
| `placeholder` | `string \| null` | | Shown while it is empty. |
| `value` | `string \| null` | | The text, when the app controls it. |
| `defaultValue` | `string` | `""` | The first text, when the field keeps its own. |
| `secure` | `boolean` | `false` | Hides what is typed (passwords). |
| `size` | `"sm" \| "md" \| "lg"` | `"md"` | The height, padding, text size and corners. |
| `disabled` | `boolean` | `false` | Not editable, 50% opacity. |
| `class`, `classList`, `style`, `testID`, `onLayout`, `ref` | | | Applied to the field's root view (the label and the box). |

| Event | Payload | When |
|---|---|---|
| `onChange` | `string`: the new text | The text changed. |
| `onSubmit` | `string`: the text | Return was pressed. |

## NumberInput

`NumberInput` is a number field between − and + buttons. A click on a button steps the value by `step`, and holding
it keeps stepping until the button is released or the value reaches `min` or `max`; a button at its bound is dimmed.
What is typed is read when Return is pressed or the field loses focus: rounded to the decimals of `step`, kept between
`min` and `max`, and shown with those decimals (`0.50` for a step of 0.25). Text that is not a number gives the value
back. Tab stops at the field; the buttons are for the pointer.

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

It has the field look of `TextField` (border, hover, focus ring); the buttons get the theme's `accent.background`
under the pointer and `secondary.active.background` while pressed.

| Prop | Type | Default | Description |
|---|---|---|---|
| `value` | `number \| null` | | The value, when the app controls it. |
| `defaultValue` | `number` | `0` | The first value, when the field keeps its own. |
| `min` | `number` | `0` | The smallest value. |
| `max` | `number` | `100` | The largest value. |
| `step` | `number` | `1` | How much a button press changes the value; its decimals are the value's. |
| `size` | `"sm" \| "md" \| "lg"` | `"md"` | The height, text size and corners. |
| `disabled` | `boolean` | `false` | Not editable, no presses, 50% opacity. |
| `class`, `classList`, `style`, `testID`, `onLayout`, `ref` | | | Applied to the field's root view. |

| Event | Payload | When |
|---|---|---|
| `onChange` | `number`: the new value | A button stepped it, or what was typed was read and differs from it. |
