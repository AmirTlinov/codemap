import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { TownHub } from "./hub";
import {
  ClientFrameSchema,
  MAX_TEXT_CHAT_LEN,
  ServerFrameSchema,
} from "./protocol";
import type { ClientFrame } from "./protocol";

const AGENT_A = "00000000-0000-4000-8000-000000000001";
const AGENT_B = "00000000-0000-4000-8000-000000000002";

function hello(
  id: string,
  name: string,
  position = { x: 7, y: 13 },
): Extract<ClientFrame, { type: "hello" }> {
  return { type: "hello", agent_id: id, agent_name: name, position };
}

describe("flagship global text chat contract", () => {
  let hub: TownHub;

  beforeEach(() => {
    hub = new TownHub();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("owns bounded text_chat and text_chat_broadcast wire frames", () => {
    const accepted = ClientFrameSchema.safeParse({
      type: "text_chat",
      text: "  Meet at the portal  ",
    });
    expect(accepted.success).toBe(true);
    hub.handle("conn-a", hello(AGENT_A, "Volkov"));
    if (accepted.success) {
      const normalized = hub.handle("conn-a", accepted.data).toSender[0];
      expect(normalized?.type).toBe("text_chat_broadcast");
      if (normalized?.type === "text_chat_broadcast") {
        expect(normalized.text).toBe("Meet at the portal");
      }
    }
    for (const text of ["   ", "line\nbreak", `nul\u0000byte`, "x".repeat(MAX_TEXT_CHAT_LEN + 1)]) {
      const parsed = ClientFrameSchema.safeParse({ type: "text_chat", text });
      if (parsed.success) {
        expect(hub.handle("conn-a", parsed.data).toSender[0]?.type).toBe("reject");
      }
    }
    expect(
      ServerFrameSchema.safeParse({
        type: "text_chat_broadcast",
        peer_id: AGENT_A,
        agent_name: "Volkov",
        text: "Meet at the portal",
        ts: 42,
      }).success,
    ).toBe(true);
  });

  it("rejects pre-hello text and broadcasts normalized server-owned identity", () => {
    vi.useFakeTimers();
    vi.setSystemTime(42);
    expect(hub.handle("conn-a", { type: "text_chat", text: "hello" }).toSender[0]).toEqual({
      type: "reject",
      reason: "hello_required",
    });
    hub.handle("conn-a", hello(AGENT_A, "Volkov"));
    const dispatch = hub.handle("conn-a", { type: "text_chat", text: "  hello  " });
    const expected = {
      type: "text_chat_broadcast",
      peer_id: AGENT_A,
      agent_name: "Volkov",
      text: "hello",
      ts: 42,
    };
    expect(dispatch.toSender).toEqual([expected]);
    expect(dispatch.toOthers).toEqual([expected]);
  });

  it("bounds newcomer history and isolates text floods from emoji allowance", () => {
    vi.useFakeTimers();
    const start = 1_700_000_000_000;
    const historyInputs = 80;
    for (let index = 0; index < historyInputs; index += 1) {
      vi.setSystemTime(start + index * 2_000);
      const connection = `history-${index}`;
      const agent = `00000000-0000-4000-8000-${String(index + 10).padStart(12, "0")}`;
      hub.handle(connection, hello(agent, `Agent ${index}`, { x: 6, y: 13 }));
      const result = hub.handle(connection, {
        type: "text_chat",
        text: `message ${index}`,
      });
      expect(result.toSender[0]?.type).toBe("text_chat_broadcast");
      hub.disconnect(connection);
    }
    const newcomerFrames = hub.handle(
      "conn-b",
      hello(AGENT_B, "Silverthorn", { x: 8, y: 13 }),
    ).toSender;
    const welcome = newcomerFrames.find((frame) => frame.type === "welcome");
    if (welcome?.type !== "welcome") throw new Error("expected welcome");
    const embeddedHistory = (welcome as unknown as {
      chat_history?: Extract<(typeof newcomerFrames)[number], { type: "text_chat_broadcast" }>[];
    }).chat_history;
    const history =
      embeddedHistory ??
      newcomerFrames.filter((frame) => frame.type === "text_chat_broadcast");
    expect(history.length).toBeGreaterThan(0);
    expect(history.length).toBeLessThan(historyInputs);
    expect(history[0]?.text).toBe(
      `message ${historyInputs - history.length}`,
    );
    expect(history.at(-1)?.text).toBe(`message ${historyInputs - 1}`);

    const floodHub = new TownHub();
    floodHub.handle("conn-a", hello(AGENT_A, "Volkov"));
    vi.setSystemTime(start + 100_000);
    let rejected = false;
    for (let index = 0; index < 64; index += 1) {
      const dispatch = floodHub.handle("conn-a", { type: "text_chat", text: `flood ${index}` });
      if (dispatch.toSender[0]?.type === "reject") {
        rejected = true;
        break;
      }
    }
    expect(rejected).toBe(true);
    expect(floodHub.handle("conn-a", { type: "chat", emoji: "sword" }).toOthers[0]?.type).toBe(
      "chat_broadcast",
    );
  });
});
