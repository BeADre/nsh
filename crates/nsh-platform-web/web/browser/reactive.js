// Browser functions and values use Rust's shared reactive graph. A facade
// belongs to one app instance; it never consults a process-global player.
export function createReactive(invoke, live) {
  const call = (op, ...args) => {
    if (!live()) throw new Error("the app was disposed");
    return invoke(op, args);
  };
  const finalizers = new FinalizationRegistry(handle => {
    if (live()) call(114, handle);
  });
  const accessor = handle => {
    const read = () => call(101, handle);
    finalizers.register(read, handle);
    return read;
  };
  const signal = (value, options) => {
    const handle = call(100, value, options?.equals);
    const read = accessor(handle);
    // A retained setter keeps its accessor (and therefore its graph handle) alive.
    read.set = value => { void read; return call(102, handle, value); };
    read.update = update => untracked(() => read.set(update(read())));
    return read;
  };
  const computed = (fn, options) => accessor(call(103, fn, undefined, options?.equals));
  const renderEffect = (fn, initial) => call(104, fn, initial, false);
  const effect = fn => call(104, () => { const dispose = fn(); if (typeof dispose === "function") cleanup(dispose); }, undefined, true);
  const cleanup = fn => call(109, fn);
  const batch = fn => call(107, fn);
  const untracked = fn => call(108, fn);
  const root = fn => call(105, handle => fn(() => { if (handle && live()) call(106, handle); }));
  const getOwner = () => call(110);
  const runWithOwner = (owner, fn) => call(111, owner, fn);
  const context = defaultValue => {
    const key = {};
    const read = () => call(112, key, defaultValue);
    read.Provider = props => {
      const parent = getOwner();
      return root(dispose => {
        if (parent) runWithOwner(parent, () => cleanup(dispose));
        call(113, key, props.value);
        return props.children;
      });
    };
    return read;
  };
  const split = (props, ...groups) => {
    const used = new Set();
    const picks = groups.map(keys => {
      const pick = {};
      for (const key of keys) {
        used.add(key);
        Object.defineProperty(pick, key, { enumerable: true, get: () => props[key] });
      }
      return pick;
    });
    const rest = {};
    for (const key of Object.keys(props)) {
      if (!used.has(key)) Object.defineProperty(rest, key, { enumerable: true, get: () => props[key] });
    }
    return [...picks, rest];
  };
  return { signal, computed, effect, cleanup, batch, untracked, root, context, split,
    getOwner, runWithOwner, renderEffect };
}
