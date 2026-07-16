import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ChatStrip } from "./ChatStrip";

describe("flagship global text chat UI contract", () => {
  it("submits text through the existing town chat and renders named history", () => {
    const publishText = vi.fn().mockReturnValue(true);
    render(
      <ChatStrip
        onSend={() => true}
        onSendText={publishText}
        recent={[]}
        messages={[
          {
            peerId: "00000000-0000-4000-8000-000000000001",
            agentName: "Volkov",
            text: "Meet at the portal",
            ts: 42,
          },
        ]}
      />,
    );
    expect(screen.getByText("Volkov")).toBeInTheDocument();
    expect(screen.getByText("Meet at the portal")).toBeInTheDocument();

    const input = screen.getByRole("textbox");
    fireEvent.change(input, { target: { value: "Reinforcements incoming" } });
    fireEvent.click(screen.getByRole("button", { name: /отправить/i }));
    expect(publishText).toHaveBeenCalledWith("Reinforcements incoming");
    expect(input).toHaveValue("");
  });
});
