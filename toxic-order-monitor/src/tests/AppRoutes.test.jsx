import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen } from "@testing-library/react";
import React from "react";
import { MemoryRouter, useLocation } from "react-router-dom";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "../App.jsx";

vi.mock("../pages/Dashboard.jsx", () => ({
  default: function DashboardRouteProbe() {
    const location = useLocation();
    return <div data-testid="current-path">{location.pathname}</div>;
  },
}));

afterEach(() => {
  cleanup();
});

describe("App asset routes", () => {
  it("redirects legacy contract whale route to BTC", async () => {
    renderApp("/contract-whale");

    expect(await screen.findByTestId("current-path")).toHaveTextContent("/contract-whale/btc");
  });

  it("redirects legacy spot monitor aliases to BTC", async () => {
    renderApp("/spot-monitor");

    expect(await screen.findByTestId("current-path")).toHaveTextContent("/spot-monitor/btc");
  });

  it("redirects invalid mainstream asset routes back to BTC", async () => {
    renderApp("/contract-whale/sol");

    expect(await screen.findByTestId("current-path")).toHaveTextContent("/contract-whale/btc");
  });

  it("does not register the removed altcoin manipulation route", async () => {
    renderApp("/altcoin-manipulation");

    expect(await screen.findByTestId("current-path")).toHaveTextContent("/");
  });
});

function renderApp(path) {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <App />
    </MemoryRouter>,
  );
}
