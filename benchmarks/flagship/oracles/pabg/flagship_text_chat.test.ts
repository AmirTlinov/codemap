import { beforeEach, describe, expect, it } from "vitest";
import { TownHub } from "./hub";
import {
  ClientFrameSchema,
  MAX_TEXT_CHAT_LEN,
  ServerFrameSchema,
  TEXT_CHAT_HISTORY_LIMIT,
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

  it("owns bounded text_chat and text_chat_broadcast wire frames", () => {
    const accepted = ClientFrameSchema.safeParse({
      type: "text_chat",
      text: "  Meet at the portal  ",
    });
    expect(accepted.success).toBe(true);
    if (accepted.success && accepted.data.type === "text_chat") {
      expect(accepted.data.text).toBe("Meet at the portal");
    }
    for (const text of ["   ", "line\nbreak", `nul\u0000byte`, "x".repeat(MAX_TEXT_CHAT_LEN + 1)]) {
      expect(ClientFrameSchema.safeParse({ type: "text_chat", text }).success).toBe(false);
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
    expect(hub.handle("conn-a", { type: "text_chat", text: "hello" }).toSender[0]).toEqual({
      type: "reject",
      reason: "hello_required",
    });
    hub.handle("conn-a", hello(AGENT_A, "Volkov"));
    const dispatch = hub.handle("conn-a", { type: "text_chat", text: "  hello  " }, 42);
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
    hub.handle("conn-a", hello(AGENT_A, "Volkov", { x: 6, y: 13 }));
    for (let index = 0; index < TEXT_CHAT_HISTORY_LIMIT + 3; index += 1) {
      const result = hub.handle(
        "conn-a",
        { type: "text_chat", text: `message ${index}` },
        index * 2_000,
      );
      expect(result.toSender[0]?.type).toBe("text_chat_broadcast");
    }
    const flood = hub.handle("conn-a", { type: "text_chat", text: "too soon" }, 1);
    expect(flood.toSender[0]?.type).toBe("reject");
    expect(hub.handle("conn-a", { type: "chat", emoji: "sword" }, 1).toOthers[0]?.type).toBe(
      "chat_broadcast",
    );

    const welcome = hub.handle(
      "conn-b",
      hello(AGENT_B, "Silverthorn", { x: 8, y: 13 }),
    ).toSender[0];
    if (welcome?.type !== "welcome") throw new Error("expected welcome");
    expect(welcome.chat_history).toHaveLength(TEXT_CHAT_HISTORY_LIMIT);
    expect(welcome.chat_history[0]?.text).toBe("message 3");
    expect(welcome.chat_history.at(-1)?.text).toBe(`message ${TEXT_CHAT_HISTORY_LIMIT + 2}`);
  });
});
