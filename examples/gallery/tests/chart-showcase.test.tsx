// The Chart showcase demo: charts an app builds from `nsh/base/plot`'s
// parts and its own views. A live price streamed a tick a second that
// pauses and resumes, an order book's depth whose side under the pointer
// is read out and whose range changes, a drawdown whose day under the
// pointer is read out over both of its plots, and capacity rings by
// period.
import { beforeEach, expect, open, test } from "nsh/test";
import type { App } from "nsh/test";

let app: App;
beforeEach(async () => {
  // A mouse to hover with, and a manual clock for the stream's ticks.
  app = await open("../App.tsx", { platform: "macos", width: 1200, height: 1700, clock: {} });
  await app.press("role:button[Chart showcase]");
  await app.waitFor("Live price");
});

/** The live price's figure: the text right after the stream's state. */
const price = () => {
  const texts = app.texts();
  return texts[texts.findIndex((t) => t === "LIVE" || t === "PAUSED") + 1];
};

test("the live price ticks every second until it is paused", async () => {
  const first = price();
  await app.clock.runFor(3000);
  expect(price()).not.toBe(first);
  await app.press("Pause");
  await app.waitFor("PAUSED");
  const paused = price();
  await app.clock.runFor(3000);
  expect(price()).toBe(paused);
  await app.press("Resume");
  await app.waitFor("LIVE");
});

test("the depth reads out the side under the pointer and changes its range", async () => {
  expect(app.texts()).toContain("99.58");
  await app.hover("label:/^Order book depth within 20 ticks/");
  await app.wait(100);
  expect(app.texts().some((t) => t.startsWith("BIDS ≥") || t.startsWith("ASKS ≤"))).toBe(true);
  await app.press("role:tab[40 ticks]");
  await app.waitFor("99.18");
});

test("the drawdown reads out the day under the pointer on either plot", async () => {
  expect(app.texts()).toContain("DAY 120");
  expect(app.texts()).toContain("at a high");
  await app.hover("label:/^Drawdown from the high/");
  await app.waitFor("DAY 60");
  expect(app.texts().some((t) => t.endsWith("% from the high"))).toBe(true);
});

test("the capacity rings show each period's load", async () => {
  expect(app.texts()).toContain("49%");
  await app.press("role:tab[7d peak]");
  await app.waitFor("88%");
  expect(app.texts()).toContain("92%");
});
