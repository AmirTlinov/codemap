import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const transport = vi.hoisted(() => ({
  options: null as null | {
    onFrame: (frame: unknown) => void;
    onStatus: (status: string) => void;
  },
  send: vi.fn(() => true),
}));

vi.mock("@/lib/ws-client", () => ({
  WsClient: class {
    constructor(options: typeof transport.options) {
      transport.options = options;
    }
    connect() {
      transport.options?.onStatus("open");
    }
    close() {}
    send(frame: unknown) {
      return transport.send(frame);
    }
  },
}));

import { useTownMultiplayer } from "./useTownMultiplayer";

const AGENT = "00000000-0000-4000-8000-000000000001";

describe("flagship text chat multiplayer consumer", () => {
  beforeEach(() => {
    transport.options = null;
    transport.send.mockClear();
    vi.stubGlobal("WebSocket", class {});
  });

  it("publishes normalized text and consumes welcome history plus broadcasts", () => {
    const { result } = renderHook(() =>
      useTownMultiplayer({
        url: "ws://town.test",
        agentId: AGENT,
        agentName: "Volkov",
        spawn: { x: 7, y: 13 },
        enabled: true,
      }),
    );
    expect(transport.options).not.toBeNull();

    act(() => {
      transport.options?.onFrame({
        type: "welcome",
        your_id: AGENT,
        peers: [],
        chat_history: [
          { peer_id: AGENT, agent_name: "Volkov", text: "First", ts: 1 },
        ],
      });
      transport.options?.onFrame({
        type: "text_chat_broadcast",
        peer_id: AGENT,
        agent_name: "Volkov",
        text: "Second",
        ts: 2,
      });
    });

    const state = result.current as unknown as {
      textChat?: readonly { text: string }[];
      chatHistory?: readonly { text: string }[];
      textChatHistory?: readonly { text: string }[];
      recentTextChat?: readonly { text: string }[];
    };
    const history =
      state.textChat ?? state.chatHistory ?? state.textChatHistory ?? state.recentTextChat;
    expect(history?.map((message) => message.text)).toEqual(["First", "Second"]);
    expect(result.current.publishTextChat("Third")).toBe(true);
    expect(transport.send).toHaveBeenLastCalledWith({ type: "text_chat", text: "Third" });
  });
});
