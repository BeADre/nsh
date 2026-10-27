import { Values } from "./values.js";
import { createReactive } from "./reactive.js";
import { createModules } from "./modules.js";
import { createDom } from "./dom.js";
import { createComponents } from "./components.js";
import { createEnvironment } from "./environment.js";

export function createBrowserHost(player) {
  let current;
  return {
    imports: { value: (...args) => {
      if (!current) throw new Error("the browser host was not initialized");
      return current.imports.value(...args);
    } },
    async run(entry, { appId = "", capabilities = "", imports: moduleMap = {} } = {}) {
      const [width, height] = player.reset();
      const values = new Values();
      let running = true, modules, components, frame, rejectLoad;
      const urls = [];
      const active = () => running;
      const call = (op, args) => {
        if (!active()) throw new Error("the app was disposed");
        return values.with([args], ([id]) => values.result(player.wasm.nsh_browser_call(op, id)));
      };
      const requestPump = () => { if (active()) player.requestPump(); };
      const report = error => { try { player.showError(error?.stack ?? String(error)); } catch { console.error(error); } };
      values.report = report;
      const stopped = [];
      current = {
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
      const reactive = createReactive(call, active);
      const dom = createDom(call, values, reactive, active, requestPump);
      const env = createEnvironment(reactive, call, dom);
      dom.rt.setup = env.setup;
      dom.rt.font = (uri, family) => call(404, [uri, family]);
      stopped.push(() => dom.dispose(), () => modules?.dispose(), () => player.wasm.nsh_stop(), () => player.wasm.nsh_browser_dispose());
      const event = (name, args) => {
        if (!active()) return;
        switch (name) {
          case "event": dom.event(...args); break;
          case "reply": modules?.reply(...args); break;
          case "component": components.component(...args); break;
          case "children": components.children(...args); break;
          case "render": components.render(...args); break;
          case "unrender": components.unrender(...args); break;
          case "environment": env.update(args[0]); break;
          case "tokens": env.changedTokens(); break;
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
      components = createComponents(call, values, reactive, dom);
      modules = createModules(call, values, active, requestPump, components.make);
      const nsh = modules.exports.get("nsh") ?? {};
      Object.assign(nsh, reactive, env.api, { mount: dom.mount });
      const desktop = modules.exports.get("nsh/desktop");
      if (desktop) Object.assign(desktop, { isAvailable: false, titleBar: env.titleBar });
      for (const [name, tag] of Object.entries({View:"view",Text:"text",Scroll:"scroll",Overlay:"overlay",Image:"image",WebView:"webview",Video:"video"})) {
        const Component = props => {const node = dom.rt.element(tag);dom.rt.spread(node,props);if ("children" in props) dom.rt.insert(node,()=>props.children);return node;};
        nsh[name] = Object.assign(Component, nsh[name]);
      }
      const base = modules.exports.get("nsh/base");
      if (base) base.Input = props => {const node=dom.rt.element("input");dom.rt.spread(node,props);return node;};
      const namespaces = new Map(modules.exports);
      namespaces.set("nsh", nsh);
      namespaces.set("nsh:rt", dom.rt);
      const map = { ...moduleMap };
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
          ready: resolve, failed: reject,
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
