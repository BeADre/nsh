---
title: Desktop UI
description: The nsh/ui/desktop UI set, NSH's design language at a pointer's and a keyboard's density, and its Button.
order: 20
---

# Desktop UI

`nsh/ui/desktop` is NSH's UI set for windows on macOS, Windows, Linux and the web. It draws the same design language
as the [mobile UI](./index.md), with the same theme, at a desktop's density: shorter controls, a highlight under the
pointer, and a focus ring when the keyboard moves to a control. Its first component is `Button`; more follow.

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
