// `nsh/ui/mobile`'s Rust components laying out the app's content: a
// `ListView` recycles rows the app renders, a `Group` wraps each child in a
// row. The app writes the content; the components only place it.
import { createSignal } from "solid-js";
import { Button, Group, ListView } from "nsh/ui/mobile";

const stocks = Array.from({ length: 300 }, (_, i) => ({
  id: i,
  symbol: `SYM${String(i).padStart(3, "0")}`,
  price: (50 + ((i * 37) % 400) + (i % 7) * 0.13).toFixed(2),
  change: (((i * 13) % 21) - 10) / 4,
}));

function Header() {
  const [taps, setTaps] = createSignal(0);
  return (
    <view style={{ padding: 16, gap: 16 }}>
      <Group header={<text>Group: each child is a row</text>} footer={<text>The rows are JSX; the group adds the box and the dividers.</text>}>
        <text>Notifications</text>
        <text>Privacy</text>
        <view style={{ flexDirection: "row", justifyContent: "space-between" }}>
          <text>Taps</text>
          <text>{taps()}</text>
        </view>
      </Group>
      <Button title="Tap" variant="secondary" onPress={() => setTaps(taps() + 1)} />
      <text style={{ fontSize: 13, color: "#57606a" }}>ListView below: 300 rows the app renders, recycled natively.</text>
    </view>
  );
}

export function MobileUi() {
  return (
    <ListView each={stocks} keyExtractor={(s) => String(s.id)} ListHeaderComponent={Header}>
      {(stock) => (
        <view style={{ flexDirection: "row", alignItems: "center" }}>
          <text style={{ flex: 1, fontWeight: "600" }}>{stock().symbol}</text>
          <text style={{ width: 80, textAlign: "right" }}>{stock().price}</text>
          <text style={{ width: 64, textAlign: "right", color: stock().change >= 0 ? "#1a7f37" : "#cf222e" }}>
            {(stock().change >= 0 ? "+" : "") + stock().change.toFixed(2)}%
          </text>
        </view>
      )}
    </ListView>
  );
}
