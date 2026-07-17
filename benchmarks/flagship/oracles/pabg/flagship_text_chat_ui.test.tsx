import { fireEvent, render, screen } from "@testing-library/react";
import type { ComponentType } from "react";
import { describe, expect, it, vi } from "vitest";
import { ChatStrip } from "./ChatStrip";

const FlexibleChatStrip = ChatStrip as ComponentType<Record<string, unknown>>;

describe("flagship global text chat UI contract", () => {
  it("submits text through the existing town chat and renders named history", () => {
    const publishText = vi.fn().mockReturnValue(true);
    const message = {
      peerId: "00000000-0000-4000-8000-000000000001",
      agentName: "Volkov",
      peer_id: "00000000-0000-4000-8000-000000000001",
      agent_name: "Volkov",
      text: "Meet at the portal",
      ts: 42,
    };
    render(
      <FlexibleChatStrip
        onSend={() => true}
        onSendText={publishText}
        onTextSend={publishText}
        recent={[]}
        messages={[message]}
        chatHistory={[message]}
        textHistory={[message]}
        textMessages={[message]}
        textRecent={[message]}
      />,
    );
    expect(screen.getByText(/Volkov/)).toBeInTheDocument();
    expect(screen.getByText("Meet at the portal")).toBeInTheDocument();

    const input = screen.getByRole("textbox");
    fireEvent.change(input, { target: { value: "Reinforcements incoming" } });
    fireEvent.click(screen.getByRole("button", { name: /отправить/i }));
    expect(publishText).toHaveBeenCalledWith("Reinforcements incoming");
    expect(input).toHaveValue("");
  });
});
