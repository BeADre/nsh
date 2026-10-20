// Navigation: a stack of screens with header options, pushed and popped.
import { mount, signal, Text, View } from "nsh";
import { createNavigator, Screen, Stack } from "nsh/base";
import { ui } from "./palette";

function Button(props: { title: string; variant?: "primary" | "secondary"; onPress: () => void }) {
  const primary = () => props.variant !== "secondary";
  return (
    <View accessibilityRole="button" onPress={props.onPress} style={primary() ? ui().primary : ui().raised} class="items-center rounded-xl py-3 active:opacity-75">
      <Text style={{ fontWeight: "600", color: primary() ? ui().onPrimary : ui().text }}>{props.title}</Text>
    </View>
  );
}

type Route = { name: "home" } | { name: "detail"; id: number };

const nav = createNavigator<Route>({ name: "home" });

function Home() {
  const opened = signal(0);
  return (
    <Screen title="Inbox">
      <View style={{ flex: 1, backgroundColor: ui().page, padding: 16, paddingTop: 24, gap: 12 }}>
        <Text style={{ color: ui().muted }}>{`Opened ${opened()} messages`}</Text>
        <Button
          title="Open message 1"
          onPress={() => {
            opened.set(opened() + 1);
            nav.push({ name: "detail", id: 1 });
          }}
        />
        <Button title="Open message 2" variant="secondary" onPress={() => nav.push({ name: "detail", id: 2 })} />
      </View>
    </Screen>
  );
}

function Detail(props: { id: number }) {
  const starred = signal(false);
  return (
    <Screen
      title={`Message ${props.id}`}
      headerRight={
        <Text onPress={() => starred.set(!starred())} style={{ fontSize: 17, color: ui().accent }}>
          {starred() ? "★" : "☆"}
        </Text>
      }
    >
      <View style={{ flex: 1, backgroundColor: ui().page, padding: 16, gap: 12 }}>
        <Text style={{ color: ui().text }}>{`The body of message ${props.id}.`}</Text>
        <Button title="Next message" onPress={() => nav.push({ name: "detail", id: props.id + 1 })} />
        <Button title="Back to the inbox" variant="secondary" onPress={() => nav.popToTop()} />
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
