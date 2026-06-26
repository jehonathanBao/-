import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen } from "@testing-library/react";
import React from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import PageShellSkeleton from "../components/PageShellSkeleton.jsx";
import { PageErrorBoundary } from "../components/PageErrorBoundary.jsx";

function Boom() {
  throw new Error("boom");
}

describe("page shell safety", () => {
  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  it("renders a visible dashboard shell skeleton", () => {
    render(<PageShellSkeleton />);

    expect(screen.getByTestId("page-shell-skeleton")).toBeInTheDocument();
    expect(screen.getByText("Loading dashboard")).toBeInTheDocument();
  });

  it("shows a readable fallback when the page crashes", () => {
    const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});

    render(
      <PageErrorBoundary>
        <Boom />
      </PageErrorBoundary>,
    );

    expect(screen.getByTestId("page-error-boundary")).toBeInTheDocument();
    expect(screen.getByText("页面加载失败")).toBeInTheDocument();
    expect(consoleSpy).toHaveBeenCalled();
  });
});
