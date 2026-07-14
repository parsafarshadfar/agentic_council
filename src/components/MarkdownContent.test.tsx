// @vitest-environment jsdom

import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { MarkdownContent } from "./MarkdownContent";

afterEach(cleanup);

describe("MarkdownContent", () => {
  it("renders model Markdown and GFM tables as formatted elements", () => {
    const { container } = render(
      <MarkdownContent content={"## Recommendation\n\nUse **Option A**.\n\n| Choice | Score |\n| --- | ---: |\n| A | 9 |"} />,
    );

    expect(screen.getByRole("heading", { name: "Recommendation" })).toBeInTheDocument();
    expect(screen.getByText("Option A").tagName).toBe("STRONG");
    expect(screen.getByRole("table")).toBeInTheDocument();
    expect(container.querySelector(".markdown-table-wrap")).toBeInTheDocument();
  });

  it("does not render HTML supplied by a model", () => {
    const { container } = render(<MarkdownContent content={'Safe\n\n<script data-testid="unsafe">alert(1)</script>'} />);

    expect(container.querySelector("script")).not.toBeInTheDocument();
    expect(screen.getByText("Safe")).toBeInTheDocument();
  });
});
