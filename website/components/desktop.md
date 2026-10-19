---
title: Desktop UI
description: The nsh/ui/desktop UI set, NSH's design language at a pointer's and a keyboard's density, and its components.
order: 20
---

# Desktop UI

`nsh/ui/desktop` is NSH's UI set for windows on macOS, Windows, Linux and the web. It draws the same design language
as the [mobile UI](./index.md), with the same theme, at a desktop's density: shorter controls, a highlight under the
pointer, and a focus ring when the keyboard moves to a control. Its first components are `Button`, `Popover` and
`Dialog`; more follow.

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
