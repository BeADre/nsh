// A case of the API's test suite (docs/dev/compatibility.md): `StatusBar`
// of `nsh/base` run as in a browser (`platform web`, where `web={{…}}` is
// read): settings with every prop, changed through signals, stacked and
// removed. It renders nothing and takes no room: the views around it lay
// out as without it. What the status bar shows reaches only the platform,
// which the headless runtime has none of. Kept with
// `cargo xtask api --keep api/cases/base-status-bar.tsx`; its script checks
// each.
import { Text, View, mount, signal } from "nsh";
import { Button, StatusBar } from "nsh/base";

function App() {
  const dark = signal(false);
  const second = signal(true);
  return (
    <View style={{ flex: 1 }}>
      <StatusBar
        foreground={dark() ? "light" : "dark"}
        hidden={false}
        animated
        web={{ themeColor: dark() ? "#000000" : "#ffffff" }}
      />
      <View testID="first" style={{ height: 20 }} />
      <If when={second()}>
        <StatusBar foreground="auto" hidden animated={false} web={{ themeColor: null }} />
      </If>
      <View testID="second" style={{ height: 20 }} />
      <StatusBar foreground={null} hidden={null} animated={null} web={null} />
      <Button onPress={() => dark.set(!dark())}>
        <Text>{`Dark: ${dark()}`}</Text>
      </Button>
      <Button onPress={() => second.set(!second())}>
        <Text>{`Second: ${second()}`}</Text>
      </Button>
    </View>
  );
}

mount(App);
