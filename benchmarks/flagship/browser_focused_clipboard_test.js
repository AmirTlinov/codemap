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
const offscreenCalls = [];
let scriptMode = "success";
let offscreenMode = "success";
const port = { onMessage: event(), onDisconnect: event(), postMessage() {}, disconnect() {} };
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
    sendMessage(payload, callback) {
      offscreenCalls.push(payload);
      if (payload.type === "offscreen.ping") return callback({ ok: true });
      if (offscreenMode === "failure") return callback({ ok: false, error: "offscreen denied" });
      callback({ ok: true, result: { ok: true, method: "offscreen" } });
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
      if (scriptMode === "throw") throw new Error("tab injection denied");
      if (scriptMode === "failure") {
        return [{ frameId: 0, result: { ok: false, error: "tab clipboard denied" } }];
      }
      return [{
        frameId: 0,
        result: { ok: true, method: "navigator.clipboard.write", mimes: ["text/plain"] },
      }];
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

function tabProvenance(result, expectedTab, expectedSource) {
  const sourceMatches = (actual) => !expectedSource
    || actual === expectedSource
    || (expectedSource === "requested" && actual === "explicit");
  if (result.context === "focused_tab") {
    return String(result.tabId) === String(expectedTab);
  }
  if (result.context === "tab") {
    const attempt = attempts(result).find((row) => row?.context === "tab");
    const selectedBy = result.source || result.selectedBy || attempt?.source || attempt?.selectedBy;
    return String(result.tabId) === String(expectedTab)
      && Number(result.frameId) === 0
      && result.world === "ISOLATED"
      && sourceMatches(selectedBy);
  }
  const value = result.context;
  const selectedBy = value?.source || value?.selectedBy;
  return value && (value.kind === "tab" || value.type === "tab" || value.carrier === "tab")
    && String(value.tabId) === String(expectedTab)
    && Number(value.frameId) === 0
    && value.world === "ISOLATED"
    && sourceMatches(selectedBy);
}

function topFrameCall(call, tabId) {
  const target = call?.target || {};
  const bounded = target.allFrames === false
    || (Array.isArray(target.frameIds)
      && target.frameIds.length === 1
      && target.frameIds[0] === 0);
  return target.tabId === tabId && target.allFrames !== true && bounded && call.world === "ISOLATED";
}

function attempts(result) {
  return Array.isArray(result?.attemptedContexts) ? result.attemptedContexts : [];
}

function contextKind(row) {
  const context = row?.context;
  return typeof row === "string"
    ? row
    : row?.kind || row?.type || row?.carrier || (typeof context === "string"
      ? context
      : context?.kind || context?.type || context?.carrier);
}

function hasAttempt(rows, kind, status) {
  return rows.some((row) => {
    const rowKind = contextKind(row);
    const rowStatus = row?.status || (row?.ok === true ? "succeeded" : row?.ok === false ? "failed" : undefined);
    return rowKind === kind && (!status || rowStatus === status);
  });
}

function hasContextError(rows, kind) {
  return rows.some((row) => contextKind(row) === kind
    && typeof row?.error === "string"
    && row.error.length > 0);
}

const textItems = [{
  mime: "text/plain",
  dataBase64: Buffer.from("hello").toString("base64"),
}];

(async () => {
  await Promise.resolve();

  const requested = await context.dispatchRpc("clipboard.write", { tabId: "77", items: textItems });
  if (!tabProvenance(requested, 77, "requested") || !topFrameCall(scriptCalls.at(-1), 77)) {
    throw new Error(`requested-tab write lost bounded provenance: ${JSON.stringify(requested)}`);
  }

  vm.runInContext('state.focusedTabId = "41"', context);
  const focused = await context.dispatchRpc("clipboard.write", { items: textItems });
  if (!tabProvenance(focused, 41, "focused") || !topFrameCall(scriptCalls.at(-1), 41)) {
    throw new Error(`focused-tab write lost bounded provenance: ${JSON.stringify(focused)}`);
  }

  scriptMode = "failure";
  offscreenMode = "success";
  const fallback = await context.dispatchRpc("clipboard.write", { tabId: "77", items: textItems });
  const fallbackAttempts = attempts(fallback);
  if (contextKind(fallback) !== "offscreen"
      || !hasAttempt(fallbackAttempts, "tab")
      || !hasAttempt(fallbackAttempts, "offscreen")
      || (!hasAttempt(fallbackAttempts, "tab", "failed")
        && !hasContextError(fallbackAttempts, "tab")
        && !hasContextError(fallback.errors || fallback.contextErrors || [], "tab"))) {
    throw new Error(`fallback did not preserve both attempts: ${JSON.stringify(fallback)}`);
  }

  scriptMode = "success";
  const svg = await context.dispatchRpc("clipboard.writeSvg", {
    tabId: "77",
    svg: '<svg xmlns="http://www.w3.org/2000/svg"/>',
    includePng: false,
  });
  if (!tabProvenance(svg, 77, "requested") || !topFrameCall(scriptCalls.at(-1), 77)) {
    throw new Error(`writeSvg bypassed focused-tab contract: ${JSON.stringify(svg)}`);
  }

  scriptMode = "failure";
  offscreenMode = "failure";
  let combined;
  try {
    await context.dispatchRpc("clipboard.write", { tabId: "77", items: textItems });
  } catch (error) {
    combined = error;
  }
  const combinedAttempts = combined?.data?.attemptedContexts || [];
  if (!combined
      || !hasAttempt(combinedAttempts, "tab")
      || !hasAttempt(combinedAttempts, "offscreen")
      || !String(combined.message).includes("tab")
      || !String(combined.message).includes("offscreen")) {
    throw new Error(`combined failure lost one context: ${combined?.stack || combined}`);
  }

  scriptMode = "success";
  offscreenMode = "success";
  const callsBeforeKill = scriptCalls.length;
  await context.dispatchRpc("state.set", { enabled: false });
  let killed = false;
  try {
    await context.dispatchRpc("clipboard.write", { tabId: "77", items: textItems });
  } catch (error) {
    killed = /off|disabled/i.test(String(error.message));
  }
  if (!killed || scriptCalls.length !== callsBeforeKill) {
    throw new Error("clipboard path bypassed the extension kill switch");
  }

  console.log(JSON.stringify({
    passed: true,
    scriptCalls: scriptCalls.length,
    offscreenCalls: offscreenCalls.length,
  }));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
