// A tiny ledger whose logic is Rust: the component `Gauge`, the module
// `Ledger` and the class `Account` all come from the app's crate
// (`native/`, nsh.json "rust"), imported by its specifier. Their types are
// generated from the Rust code into `.nsh/types/rust-exports/native.d.ts`.
import { type JSX, mount, Scroll, signal, Text, View } from "nsh";
import { Button, SafeAreaView } from "nsh/base";
import { Account, Gauge, Ledger } from "rust-exports/native";

// Each object owns an instance kept in Rust.
const savings = new Account("Savings", 120);
const checking = Account.parse("Checking: 40");

/** A statement of a throwaway account: `using` frees it at the end of the block. */
function preview(): string {
  using draft = Account.parse("Draft: 10");
  draft.deposit(5);
  return draft.statement();
}

/** A card: a heading over its content. */
function Card(props: { header: JSX.Element; children: JSX.Element | JSX.Element[] }) {
  return (
    <View style={{ gap: 8, padding: 16, borderRadius: 12, backgroundColor: "#f4f4f5" }}>
      {props.header}
      {props.children}
    </View>
  );
}

const ACTION_COLORS = { primary: "#2563eb", secondary: "#e4e4e7", danger: "#dc2626" };

/** A button of `nsh/base` with this app's look. */
function Action(props: { title: string; variant?: "primary" | "secondary" | "danger"; onPress: () => void }) {
  const variant = () => props.variant ?? "primary";
  return (
    <Button
      onPress={props.onPress}
      accessibilityLabel={props.title}
      style={{ paddingHorizontal: 14, paddingVertical: 8, borderRadius: 8, backgroundColor: ACTION_COLORS[variant()] }}
      activeStyle={{ opacity: 0.7 }}
    >
      <Text style={{ fontWeight: "600", color: variant() === "secondary" ? "#18181b" : "#ffffff" }}>{props.title}</Text>
    </Button>
  );
}

/** A line of text in the cards. */
function Line(props: { children: string; muted?: boolean }) {
  return <Text style={{ fontSize: 15, opacity: props.muted ? 0.6 : 1 }}>{props.children}</Text>;
}

function App() {
  // The module: a function returning its value at once, and two Promises.
  const copied = signal("Nothing copied yet");
  const rate = signal(0);

  // The class: a getter, a setter, methods, an async member, a callback.
  const name = signal(savings.name);
  const balance = signal(savings.balance);
  const toChecking = signal<number | null>(checking.balance);
  const heard = signal<number[]>([]);
  const status = signal(preview());
  savings.onChange((b) => {
    balance.set(b);
    heard.update((h) => [...h, b]);
  });
  const attempt = (f: () => string) => {
    try {
      status.set(f());
    } catch (e) {
      status.set((e as Error).message);
    }
  };

  // The component: controlled by `spent`, which its event updates.
  const spent = signal(25);
  const changes = signal(0);

  return (
    <SafeAreaView style={{ flex: 1 }}>
      <Scroll contentStyle={{ padding: 16, gap: 16 }}>
        <Card header={<Text>Ledger · a module</Text>}>
          <Line>{`Formatted at once: ${Ledger.format(1234.5)}`}</Line>
          <Line muted>{copied()}</Line>
          <Line muted>{rate() ? `Rate: ${rate() * 100}%` : "Rate not fetched"}</Line>
          <View style={{ flexDirection: "row", gap: 8 }}>
            <Action
              title="Copy statement"
              variant="secondary"
              onPress={async () => copied.set(`Copied ${await Ledger.copy(savings.statement())} characters`)}
            />
            <Action title="Fetch rate" variant="secondary" onPress={async () => rate.set(await Ledger.fetchRate())} />
          </View>
        </Card>

        <Card header={<Text>Account · a class</Text>}>
          <Line>{`${name()}: ${Ledger.format(balance())}`}</Line>
          <Line>{`Checking: ${toChecking() === null ? "closed" : Ledger.format(toChecking()!)}`}</Line>
          <Line muted>{`Heard: ${heard().join(", ") || "nothing yet"}`}</Line>
          <Line muted>{status()}</Line>
          <View style={{ flexDirection: "row", flexWrap: "wrap", gap: 8 }}>
            <Action title="Deposit $25" onPress={() => attempt(() => `Deposited: ${Ledger.format(savings.deposit(25))}`)} />
            <Action
              title="Move $30"
              onPress={() =>
                attempt(() => {
                  const left = savings.transfer(checking, 30);
                  toChecking.set(checking.balance);
                  return `Moved $30, ${Ledger.format(left)} left`;
                })
              }
            />
            <Action
              title="Accrue"
              onPress={async () => status.set(`Accrued: ${Ledger.format(await savings.accrue(rate() || 0.01))}`)}
            />
            <Action
              title="Rename"
              variant="secondary"
              onPress={() => {
                savings.name = "Rainy day";
                name.set(savings.name);
              }}
            />
            <Action
              title="Close checking"
              variant="danger"
              onPress={() => {
                checking.dispose();
                toChecking.set(null);
                attempt(() => `Still open: ${checking.balance}`);
              }}
            />
          </View>
        </Card>

        <Card header={<Text>Gauge · a component</Text>}>
          <Gauge
            value={spent()}
            step={25}
            label="Groceries"
            onChange={(v) => {
              spent.set(v);
              changes.update((n) => n + 1);
            }}
          >
            <Line muted>{`${Ledger.format(spent())} of $100.00`}</Line>
          </Gauge>
          <Line muted>{`Pressed ${changes()}×, onChange gave ${spent()}`}</Line>
        </Card>
      </Scroll>
    </SafeAreaView>
  );
}

mount(App);
