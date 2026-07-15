#!/usr/bin/env node
const fs = require("fs");
const path = require("path");
const vm = require("vm");

const worktree = process.argv[2];
if (!worktree) throw new Error("usage: browser_focused_clipboard_test.js WORKTREE");
const source = fs.readFileSync(
  path.join(worktree, "vendor/browser_extension/service_worker.js"),
  "utf8",
);

const event = () => ({ addListener() {}, removeListener() {} });
const scriptCalls = [];
const port = {
  onMessage: event(),
  onDisconnect: event(),
  postMessage() {},
  disconnect() {},
};
const chrome = {
  runtime: {
    id: "benchmark-extension",
    lastError: null,
    onMessage: event(),
    onInstalled: event(),
    onStartup: event(),
    getManifest: () => ({ version: "1.0.0" }),
    getURL: (value) => `chrome-extension://benchmark/${value}`,
    getContexts: async () => [],
    connectNative: () => port,
    sendMessage(_payload, callback) {
      callback({ ok: true, result: { ok: true, context: "offscreen" } });
    },
  },
  storage: { local: { get: async (fallback) => fallback, set: async () => {} } },
  tabs: {
    onActivated: event(),
    query: async () => [{ id: 41, active: true, windowId: 7 }],
    get: async (id) => ({ id: Number(id), active: id === 41, windowId: 7 }),
    create: async () => ({ id: 42, windowId: 7 }),
    update: async (id) => ({ id, windowId: 7 }),
    remove: async () => {},
  },
  scripting: {
    executeScript: async (options) => {
      scriptCalls.push(options);
      return [{ result: { ok: true, method: "navigator.clipboard.write", mimes: ["text/plain"] } }];
    },
  },
  debugger: {
    onEvent: event(),
    onDetach: event(),
    getTargets: async () => [],
    attach: async () => {},
    detach: async () => {},
    sendCommand: async () => ({}),
  },
  offscreen: {
    hasDocument: async () => false,
    createDocument: async () => {},
    closeDocument: async () => {},
  },
  alarms: { onAlarm: event(), create() {} },
  windows: { update: async () => {} },
};

const context = vm.createContext({
  chrome,
  console,
  setTimeout,
  clearTimeout,
  Date,
  Math,
  JSON,
  Promise,
  Uint8Array,
  Blob,
  Buffer,
  btoa: (value) => Buffer.from(value, "binary").toString("base64"),
  atob: (value) => Buffer.from(value, "base64").toString("binary"),
  unescape,
  encodeURIComponent,
});
vm.runInContext(source, context, { filename: "service_worker.js" });

(async () => {
  const result = await context.dispatchRpc("clipboard.write", {
    tabId: "77",
    items: [{ mime: "text/plain", dataBase64: Buffer.from("hello").toString("base64") }],
  });
  if (result.context !== "focused_tab" || result.tabId !== "77") {
    throw new Error(`clipboard result did not preserve focused-tab provenance: ${JSON.stringify(result)}`);
  }
  if (scriptCalls.length !== 1 || scriptCalls[0].target.tabId !== 77) {
    throw new Error(`clipboard did not prefer requested focused tab: ${JSON.stringify(scriptCalls)}`);
  }
  if (scriptCalls[0].target.allFrames !== false || scriptCalls[0].world !== "ISOLATED") {
    throw new Error("clipboard write did not stay in the bounded isolated top frame");
  }
  console.log(JSON.stringify({ passed: true, tabId: result.tabId, context: result.context }));
  process.exit(0);
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
