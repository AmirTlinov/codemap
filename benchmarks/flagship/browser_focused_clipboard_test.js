#!/usr/bin/env node
const fs = require("fs");
const path = require("path");
const vm = require("vm");

const worktree = process.argv[2];
if (!worktree) throw new Error("usage: browser_focused_clipboard_test.js WORKTREE");
const criterion = process.argv[3] || "all";
const enabled = (name) => criterion === "all" || criterion === name;
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
  const sourceMatches = (...actuals) => {
    if (!expectedSource) return true;
    const accepted = expectedSource === "requested"
      ? ["request", "requested", "explicit", "explicit_request"]
      : [expectedSource, `${expectedSource}_state`];
    return actuals.some((actual) => accepted.includes(actual));
  };
  if (result.context === "focused_tab") {
    return String(result.tabId) === String(expectedTab);
  }
  if (result.context === "tab") {
    const attempt = attempts(result).find((row) => row?.context === "tab");
    return String(result.tabId ?? result.selectedTabId ?? result.selectedTab?.tabId) === String(expectedTab)
      && isTopFrame(result)
      && carrierWorld(result) === "ISOLATED"
      && sourceMatches(
        result.source,
        result.selectedTabSource,
        result.selectedTab?.source,
        result.selectedBy,
        result.selectedFrom,
        result.selectionSource,
        result.tabIdSource,
        attempt?.source,
        attempt?.selectedBy,
        attempt?.selectedFrom,
        attempt?.selectionSource,
        attempt?.tabIdSource,
      );
  }
  const value = typeof result.context === "object" ? result.context : result;
  return value && contextKind(value) === "tab"
    && String(value.tabId ?? value.selectedTabId ?? value.selectedTab?.tabId) === String(expectedTab)
    && isTopFrame(value)
    && carrierWorld(value) === "ISOLATED"
    && sourceMatches(
      value.source,
      value.selectedTabSource,
      value.selectedTab?.source,
      value.selectedBy,
      value.selectedFrom,
      value.selectionSource,
      value.tabIdSource,
    );
}

function isTopFrame(value) {
  return [value, value?.carrierDetails, value?.carrier].some((candidate) =>
    Number(candidate?.frameId) === 0
      || (Array.isArray(candidate?.frameIds)
        && candidate.frameIds.length === 1
        && Number(candidate.frameIds[0]) === 0));
}

function carrierWorld(value) {
  return value?.world ?? value?.carrierDetails?.world ?? value?.carrier?.world;
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
  if (Array.isArray(result?.attemptedContexts)) return result.attemptedContexts;
  return Array.isArray(result?.attempts) ? result.attempts : [];
}

function contextKind(row) {
  const context = row?.context;
  const carrier = row?.carrier;
  return typeof row === "string"
    ? row
    : row?.kind || row?.type || (typeof carrier === "string"
      ? carrier
      : carrier?.kind || carrier?.type || carrier?.carrier) || (typeof context === "string"
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

  if (enabled("tab-provenance")) {
    const requested = await context.dispatchRpc("clipboard.write", { tabId: "77", items: textItems });
    if (!tabProvenance(requested, 77, "requested") || !topFrameCall(scriptCalls.at(-1), 77)) {
      throw new Error(`requested-tab write lost bounded provenance: ${JSON.stringify(requested)}`);
    }

    vm.runInContext('state.focusedTabId = "41"', context);
    const focused = await context.dispatchRpc("clipboard.write", { items: textItems });
    if (!tabProvenance(focused, 41, "focused") || !topFrameCall(scriptCalls.at(-1), 41)) {
      throw new Error(`focused-tab write lost bounded provenance: ${JSON.stringify(focused)}`);
    }
  }

  if (enabled("offscreen-fallback")) {
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
  }

  if (enabled("svg-carrier")) {
    scriptMode = "success";
    const svg = await context.dispatchRpc("clipboard.writeSvg", {
      tabId: "77",
      svg: '<svg xmlns="http://www.w3.org/2000/svg"/>',
      includePng: false,
    });
    if (!tabProvenance(svg, 77, "requested") || !topFrameCall(scriptCalls.at(-1), 77)) {
      throw new Error(`writeSvg bypassed focused-tab contract: ${JSON.stringify(svg)}`);
    }
  }

  if (enabled("combined-failure")) {
    scriptMode = "failure";
    offscreenMode = "failure";
    let combined;
    try {
      await context.dispatchRpc("clipboard.write", { tabId: "77", items: textItems });
    } catch (error) {
      combined = error;
    }
    const combinedAttempts = combined?.data?.attemptedContexts || combined?.data?.attempts || [];
    if (!combined
        || !hasAttempt(combinedAttempts, "tab")
        || !hasAttempt(combinedAttempts, "offscreen")
        || !String(combined.message).includes("tab")
        || !String(combined.message).includes("offscreen")) {
      throw new Error(`combined failure lost one context: ${combined?.stack || combined}`);
    }
  }

  if (enabled("kill-switch")) {
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
  }

  console.log(JSON.stringify({
    passed: true,
    criterion,
    scriptCalls: scriptCalls.length,
    offscreenCalls: offscreenCalls.length,
  }));
  process.exit(0);
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
