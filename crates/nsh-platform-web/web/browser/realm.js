// Loaded as an external ES module in the app's own same-origin realm.
// Native import(), including top-level await and module cycles, remains
// the browser's module loader. No eval or Function compilation is used.
const host = globalThis.__nsh_browser_realm;
try {
  await import(host.entry);
  host.ready();
} catch (error) {
  host.failed(error);
}
