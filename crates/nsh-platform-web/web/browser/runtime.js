import { createStandard } from "./standard.js";
import { createResource } from "./resource.js";
import { createLists } from "./list.js";
import { Values } from "./values.js";
import { createReactive } from "./reactive.js";
import { createModules } from "./modules.js";
import { createDom } from "./dom.js";
import { createComponents } from "./components.js";
import { createGestures } from "./gesture.js";
import { createCanvas } from "./canvas.js";
import { createAnimation } from "./animation.js";
import { createNavigation } from "./navigation.js";
import { createKeymap } from "./keymap.js";
import { createEnvironment } from "./environment.js";

export function createBrowserHost(player) {
  let current;
  return {
    bind(wasm) {
      return Object.fromEntries(Object.entries(wasm).map(([name, value]) => [name,
        typeof value === "function" && name !== "nsh_browser_trace"
          ? (...args) => {
            const app = current;
            return app ? app.values.boundary(() => value(...args), () => wasm.nsh_browser_trace()) : value(...args);
          } : value,
      ]));
    },
    imports: { value: (...args) => {
      if (!current) throw new Error("the browser host was not initialized");
      return current.imports.value(...args);
    } },
    async run(entry, { appId = "", capabilities = "", imports: moduleMap = {}, chunks = {} } = {}) {
      const [width, height] = player.reset();
      const values = new Values();
      values.trace = () => player.wasm.nsh_browser_trace();
      let running = true, modules, components, animation, lists, frame, rejectLoad;
      const urls = [];
      const active = () => running;
      const call = (op, args) => {
        if (!active()) throw new Error("the app was disposed");
        return values.boundary(() => values.with([args], ([id]) => values.result(player.wasm.nsh_browser_call(op, id))), values.trace);
      };
      const requestPump = () => { if (active()) player.requestPump(); };
      const report = error => { try { player.showError(error?.stack ?? String(error)); } catch { console.error(error); } };
      values.report = report;
      const stopped = [];
      current = {
        values,
        imports: values.imports(() => player.wasm.memory),
        stop() {
          if (!running) return;
          rejectLoad?.(new Error("the app was disposed")); rejectLoad = null;
          // Cleanup is app code: a failure must not strand promises or
          // prevent the engine and the realm from being released.
          for (const stage of stopped) { try { stage(); } catch (error) { report(error); } }
          running = false;
          values.dispose();
          frame?.remove();
          for (const url of urls) URL.revokeObjectURL(url);
        },
      };
      player.wasm.nsh_browser_initialize();
      const reactive = createReactive(call, active, values);
      const dom = createDom(call, values, reactive, active, requestPump);
      const env = createEnvironment(reactive, call, dom);
      const gestures = createGestures(call, values, reactive, dom, active, requestPump);
      dom.setTouches(gestures);
      dom.rt.setup = env.setup;
      dom.rt.font = (uri, family) => call(404, [uri, family]);
      stopped.push(() => dom.dispose(), () => modules?.dispose(), () => animation?.dispose(), () => player.wasm.nsh_stop(), () => player.wasm.nsh_browser_dispose());
      const event = (name, args) => {
        if (!active()) return;
        switch (name) {
          case "bindList": lists?.bind(...args); break;
          case "event": dom.event(...args); break;
          case "reply": modules?.reply(...args); break;
          case "component": components.component(...args); break;
          case "children": components.children(...args); break;
          case "render": components.render(...args); break;
          case "unrender": components.unrender(...args); break;
          case "environment": env.update(args[0]); break;
          case "tokens": env.changedTokens(); break;
          case "animationsFinished": animation?.finished(args[0]); break;
          default: throw new Error(`browser host event is not implemented: ${name}`);
        }
      };
      call(400, [event]);
      call(401, [appId, capabilities]);
      player.begin(width, settings => {
        player.withStrings([settings], (p, l) => player.wasm.nsh_environment(p, l));
        const delay = player.wasm.nsh_browser_start(width, height, player.dark ? 1 : 0, devicePixelRatio || 1);
        if (delay === -2) throw new Error("browser engine startup failed");
        return delay;
      });
      lists = createLists(call, values, reactive, dom, active, requestPump);
      dom.rt.list = lists.render;
      components = createComponents(call, values, reactive, dom);
      modules = createModules(call, values, active, requestPump, components.make);
      dom.setMeasure(node => modules.measure(node));
      animation = createAnimation(call, active, requestPump, env);
      const nsh = modules.exports.get("nsh") ?? {};
      const canvas = createCanvas(call, reactive, dom, active, requestPump);
      Object.assign(nsh, animation.api, canvas, { PanResponder: gestures.PanResponder });
      Object.assign(nsh, reactive, env.api, { List: lists.render, nextTick: (callback, ...args) => { if (typeof callback !== "function") throw new TypeError("nextTick: callback must be a function"); queueMicrotask(() => { if (active()) callback(...args); }); }, mount: dom.mount, resource: createResource(call, reactive, active, requestPump), keymap: createKeymap(call, reactive, dom, env.api.platform, requestPump) });
      const desktop = modules.exports.get("nsh/desktop");
      if (desktop) Object.assign(desktop, { isAvailable: false, titleBar: env.titleBar, Menu: { setApplicationMenu() {}, popup: () => Promise.resolve() } });
      for (const [name, tag] of Object.entries({View:"view",Text:"text",Scroll:"scroll",Overlay:"overlay",Image:"image",WebView:"webview",Video:"video"})) {
        const Component = props => {const node = dom.rt.element(tag);dom.rt.spread(node,props);if ("children" in props) dom.rt.insert(node,()=>props.children);return node;};
        nsh[name] = Object.assign(Component, nsh[name]);
      }
      nsh.nativeComponent = (name, spec = {}) => (props = {}) => {
        const node = dom.rt.element("native");
        dom.rt.prop(node, "component", name);
        const own = new Set(spec.props ?? []), eventNames = new Set((spec.events ?? []).map(name => "on" + name[0].toUpperCase() + name.slice(1)));
        const forwarded = new Proxy(props, { ownKeys: target => Reflect.ownKeys(target).filter(key => key !== "children" && !own.has(key) && !eventNames.has(key)), getOwnPropertyDescriptor: () => ({ configurable: true, enumerable: true }) });
        dom.rt.spread(node, forwarded);
        for (const key of own) dom.rt.bindProp(node, "$" + key, () => props[key]);
        if (eventNames.size) dom.rt.prop(node, "onNativeEvent", event => {
          const handler = props["on" + event.name[0].toUpperCase() + event.name.slice(1)];
          if (typeof handler === "function") handler(event.data);
        });
        return node;
      };
      nsh.nativeModule = specifier => { const namespace = modules.exports.get(specifier); if (!namespace) throw new Error(`unknown native module ${specifier}`); return namespace; };
      const base = modules.exports.get("nsh/base");
      if (base) Object.assign(base, createNavigation(reactive, dom, env, animation.api));
      if (base) base.Input = props => {const node=dom.rt.element("input");dom.rt.spread(node,props);return node;};
      for (const name of ["nsh/mobile", "nsh/ios"]) {
        const namespace = modules.exports.get(name);
        if (namespace) namespace.InputAccessoryView = () => null; // Matches the native host off iOS.
      }
      const namespaces = new Map([...modules.exports, ...createStandard(call, active)]);
      namespaces.set("nsh", nsh);
      namespaces.set("nsh:rt", dom.rt);
      namespaces.set("nsh/list", { List: lists.render });
      namespaces.set("nsh/device", Object.fromEntries(["windowSize", "colorScheme", "setColorScheme", "fontScale", "pixelRatio", "locale", "platform"].map(name => [name, nsh[name]])));
      namespaces.set("nsh/accessibility", { reduceMotion: nsh.reduceMotion, screenReaderEnabled: nsh.screenReaderEnabled });
      for (const [specifier, name] of [["nsh/image", "Image"], ["nsh/webview", "WebView"], ["nsh/video", "Video"], ["nsh/host", "host"]]) namespaces.set(specifier, { [name]: nsh[name] });
      namespaces.set("nsh/native", { nativeComponent: nsh.nativeComponent, nativeModule: nsh.nativeModule });
      namespaces.set("nsh/keymap", { keymap: nsh.keymap });
      namespaces.set("nsh/animated", animation.api);
      namespaces.set("nsh/canvas", canvas);
      namespaces.set("nsh/gesture", { PanResponder: gestures.PanResponder });
      const map = { ...moduleMap };
      const entryURL = new URL(entry, document.baseURI);
      map[new URL("bundle.js", entryURL).href] = entryURL.href;
      for (const [id, url] of Object.entries(chunks)) map[new URL(id, entryURL).href] = new URL(url, document.baseURI).href;
      for (const [specifier, namespace] of namespaces) {
        const declarations = Object.keys(namespace).map((name, index) => {
          if (name === "default") return `export default host.get(${JSON.stringify(specifier)})["default"];`;
          return `const e${index}=host.get(${JSON.stringify(specifier)})[${JSON.stringify(name)}];export{e${index} as ${JSON.stringify(name)}};`;
        }).join("\n");
        const source = `const host=globalThis.__nsh_browser_realm.modules;\n${declarations}`;
        const url = URL.createObjectURL(new Blob([source], { type: "text/javascript" }));
        urls.push(url);
        map[specifier] = url;
      }
      frame = document.createElement("iframe");
      frame.hidden = true;
      frame.setAttribute("aria-hidden", "true");
      player.container.append(frame);
      const realm = frame.contentWindow;
      const doc = realm.document;
      const baseElement = doc.createElement("base");
      baseElement.href = document.baseURI;
      doc.head.append(baseElement);
      const importMap = doc.createElement("script");
      importMap.type = "importmap";
      importMap.textContent = JSON.stringify({ imports: map });
      doc.head.append(importMap);
      const finished = new Promise((resolve, reject) => {
        rejectLoad = reject;
        Object.defineProperty(realm, "__nsh_browser_realm", { value: Object.freeze({
          entry: new URL(entry, document.baseURI).href, modules: namespaces,
          live: active, prepareChunk: id => player.options.prepareChunk?.(id), ready: resolve, failed: reject,
        }) });
      });
      const script = doc.createElement("script");
      script.type = "module";
      script.src = new URL("./realm.js", import.meta.url).href;
      doc.head.append(script);
      try { await finished; if (!active()) throw new Error("the app was disposed"); rejectLoad = null; requestPump(); } catch (error) { if (active()) report(error); throw error; }
    },
    stop() {
      const app = current;
      app?.stop();
      if (current === app) current = null;
    },
  };
}
