import type { TownBootstrap } from "@/lib/town-bootstrap";
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

const chat = vi.hoisted(() => ({
  publishText: vi.fn(() => true),
  message: {
    peerId: "00000000-0000-4000-8000-000000000001",
    agentName: "Volkov",
    peer_id: "00000000-0000-4000-8000-000000000001",
    agent_name: "Volkov",
    text: "Meet at the portal",
    ts: 42,
  },
}));

vi.mock("./TownScene", () => ({
  TownScene: () => <div data-testid="town-scene" />,
}));

vi.mock("./TownMinimap", () => ({ TownMinimap: () => null }));
vi.mock("@/lib/buildings", () => ({
  BUILDING_INFO: {
    barracks: { href: "/barracks" },
    portal: { href: "/portal" },
    chronicle: { href: "/chronicle" },
  },
}));
vi.mock("@/lib/town/scene-data", () => ({
  PLAYER_SPAWN: { x: 0, y: 0 },
  sceneTileToWs: () => ({ x: 0, y: 0 }),
}));
vi.mock("@/components/chrome/BrassButton", () => ({ BrassButton: () => null }));
vi.mock("@/components/chrome/Crest", () => ({ Crest: () => null }));
vi.mock("@/components/chrome/Eyebrow", () => ({ Eyebrow: () => null }));
vi.mock("@/components/chrome/SealBadge", () => ({ SealBadge: () => null }));
vi.mock("@/components/chrome/StatBar", () => ({ StatBar: () => null }));

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: vi.fn(), replace: vi.fn(), prefetch: vi.fn() }),
}));

vi.mock("@/hooks/useTownMultiplayer", () => ({
  useTownMultiplayer: () => ({
    status: "open",
    peers: [],
    recentChat: [],
    textChat: [chat.message],
    chatHistory: [chat.message],
    textChatHistory: [chat.message],
    recentTextChat: [chat.message],
    lastReject: null,
    publishMoveIntent: vi.fn(() => false),
    publishChat: vi.fn(() => false),
    publishTextChat: chat.publishText,
  }),
}));

import { TownHud } from "./TownHud";

const bootstrap: TownBootstrap = {
  source: "platform_api",
  agent: {
    id: "agent-profile-1",
    name: "Volkov",
    class_id: "fighter",
    level: null,
    readiness: 72,
  },
  epoch: {
    id: "epoch-42",
    label: "Эпоха 42",
    lock_at: "2099-04-18T00:00:00.000Z",
    status: "open",
  },
  multiplayer: { ws_url: "wss://town.test/ws", enabled: true },
};

describe("flagship global text chat UI contract", () => {
  it("submits through the mounted town HUD and renders named history", () => {
    render(<TownHud bootstrap={bootstrap} />);
    expect(screen.getByText(/Meet at the portal/).closest("li")).toHaveTextContent(
      /Volkov.*Meet at the portal/,
    );

    const input = screen.getByRole("textbox");
    fireEvent.change(input, { target: { value: "Reinforcements incoming" } });
    fireEvent.click(screen.getByRole("button", { name: /отправить|сказать|send/i }));
    expect(chat.publishText).toHaveBeenCalledWith("Reinforcements incoming");
    expect(input).toHaveValue("");
  });
});
