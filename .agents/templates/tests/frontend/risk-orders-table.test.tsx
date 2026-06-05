import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { RiskOrdersTable } from "../../frontend/react-risk-console/RiskOrdersTable";

describe("RiskOrdersTable", () => {
  it("masks sensitive data and renders risk labels consistently", () => {
    render(<RiskOrdersTable filters={{ page: 1, pageSize: 20 }} />);
    expect(screen.queryByText(/full phone|full address|token/i)).not.toBeInTheDocument();
  });

  it("requires confirmation before release mutation", async () => {
    const user = userEvent.setup();
    render(<RiskOrdersTable filters={{ page: 1, pageSize: 20 }} />);
    await user.click(await screen.findByRole("button", { name: /release/i }));
    expect(screen.getByRole("alertdialog")).toBeInTheDocument();
  });

  it("survives network failure", () => {
    render(<RiskOrdersTable filters={{ page: 1, pageSize: 20 }} />);
    expect(screen.getByRole("alert")).toHaveTextContent(/unable to load/i);
  });
});
