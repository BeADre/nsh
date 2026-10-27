import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { Values } from "./values.js";
import { createReactive } from "./reactive.js";
import { createModules } from "./modules.js";
import { createDom } from "./dom.js";
import { createComponents } from "./components.js";

const path = process.env.NSH_BROWSER_WASM;
async function fixture() {
  const module = await WebAssembly.compile(await readFile(path));
  const values = new Values();
  let wasm;
  const imports = {};
  for (const entry of WebAssembly.Module.imports(module)) {
    assert.equal(entry.kind, "function");
    (imports[entry.module] ??= {})[entry.name] = () => 0;
    assert.doesNotMatch(entry.name, /rquickjs/);
  }
  imports.nsh_browser = values.imports(() => wasm.memory);
  wasm = (await WebAssembly.instantiate(module, imports)).exports;
  wasm.nsh_browser_initialize();
  let live = true;
  const invoke = (op, args) => values.with([args], ([id]) => values.result(wasm.nsh_browser_call(op, id)));
  const api = createReactive(invoke, () => live);
  const dom = createDom(invoke, values, api, () => live, () => {});
  const components = createComponents(invoke, values, api, dom);
  return { api, values, modules: () => createModules(invoke, values, () => live, () => {}, components.make), close() { wasm.nsh_browser_dispose(); live = false; values.dispose(); } };
}

test("browser closures run on the shared Wasm reactive scheduler", { skip: !path }, async () => {
  const { api, close } = await fixture();
  const seen = [];
  let dispose;
  let count;
  api.root(stop => {
    dispose = stop;
    count = api.signal(1);
    const doubled = api.computed(() => count() * 2);
    api.effect(() => seen.push(doubled()));
    api.batch(() => { count.set(2); count.set(3); });
    api.cleanup(() => seen.push("disposed"));
  });
  assert.deepEqual(seen, [6]);
  dispose();
  count.set(4);
  assert.deepEqual(seen, [6, "disposed"]);
  close();
  assert.throws(() => count(), /disposed/);
});

test("contexts and untracked reads keep their owner across callbacks", { skip: !path }, async () => {
  const { api, close } = await fixture();
  const Context = api.context("default");
  let dispose;
  let read;
  let owner;
  api.root(stop => {
    dispose = stop;
    Context.Provider({ value: "provided", get children() { owner = api.getOwner(); return null; } });
    read = () => api.runWithOwner(owner, Context);
  });
  assert.equal(read(), "provided");
  assert.equal(Context(), "default");
  dispose();
  close();
});

test("exceptions restore the graph and comparison callbacks preserve identity", { skip: !path }, async () => {
  const { api, close } = await fixture();
  const signal = api.signal({ n: 1 }, { equals: (a, b) => a.n === b.n });
  let runs = 0;
  let dispose;
  api.root(stop => { dispose = stop; api.effect(() => { signal(); runs++; }); });
  signal.set({ n: 1 });
  assert.equal(runs, 1);
  signal.set({ n: 2 });
  assert.equal(runs, 2);
  assert.throws(() => api.batch(() => { throw new Error("callback failed"); }), /callback failed/);
  signal.set({ n: 3 });
  assert.equal(runs, 3);
  dispose();
  close();
});


test("registry functions use Rust conversions and preserve error classes", { skip: !path }, async () => {
  const fixture = await fixtureForModules();
  try {
    const os = fixture.modules.exports.get("os");
    assert.equal(typeof os.platform(), "string");
    assert.equal(os.arch(), "wasm32");
    assert.equal(os.default, os);
    assert.equal(os.EOL, "\n");
    assert.throws(() => os.homedir(), error => error instanceof Error && /home directory/.test(error.message));
    const process = fixture.modules.exports.get("process");
    assert.throws(() => process.run(42), TypeError);
    assert.equal(process.platform, os.platform());
  } finally { fixture.modules.dispose(); fixture.close(); }
});
async function fixtureForModules() { const f = await fixture(); return { ...f, modules: f.modules() }; }


test("null owner clears context and root disposal is idempotent", { skip: !path }, async () => {
  const { api, close } = await fixture();
  const Context = api.context("default");
  let dispose;
  api.root(stop => {
    dispose = stop;
    Context.Provider({ value: "provided", get children() {
      assert.equal(api.runWithOwner(null, Context), "default");
      assert.equal(api.runWithOwner(null, api.getOwner), null);
      return null;
    }});
  });
  dispose();
  assert.doesNotThrow(dispose);
  close();
});

test("hostile thrown message getters preserve the exception without recursive decoding", { skip: !path }, async () => {
  const { api, close } = await fixture();
  const error = { get message() { throw error; } };
  assert.throws(() => api.batch(() => { throw error; }), caught => caught === error);
  assert.equal(api.signal(3)(), 3);
  close();
});


test("computed options and returned effect cleanups match the public API", { skip: !path }, async () => {
  const { api, close } = await fixture();
  const signal = api.signal(1);
  let effects = 0, cleanups = 0, dispose;
  api.root(stop => {
    dispose = stop;
    const derived = api.computed(() => ({ value: signal() % 2 }), { equals: (a, b) => a?.value === b.value });
    api.effect(() => { derived(); effects++; return () => cleanups++; });
  });
  signal.set(3);
  assert.equal(effects, 1);
  signal.set(4);
  assert.equal(effects, 2);
  assert.equal(cleanups, 1);
  dispose();
  assert.equal(cleanups, 2);
  close();
});

test("throwing null and undefined preserves their identity", { skip: !path }, async () => {
  const { api, close } = await fixture();
  for (const error of [null, undefined]) {
    let caught = false;
    try { api.batch(() => { throw error; }); }
    catch (value) { caught = true; assert.equal(value, error); }
    assert.ok(caught);
  }
  close();
});
