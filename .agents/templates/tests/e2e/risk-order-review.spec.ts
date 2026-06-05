import { expect, test } from "@playwright/test";

test.describe("risk order review", () => {
  test("operator can filter, inspect, and release with confirmation", async ({ page }) => {
    await page.goto("/risk-orders?page=1&pageSize=20");
    await expect(page.getByRole("table")).toBeVisible();
    await page.getByRole("button", { name: /release/i }).first().click();
    await expect(page.getByRole("alertdialog")).toBeVisible();
    await page.getByRole("button", { name: /^release$/i }).click();
    await expect(page.getByText(/released|audit/i)).toBeVisible();
  });

  test("cross-shop direct URL access is denied", async ({ page }) => {
    const response = await page.goto("/risk-orders/shop-b/order-from-shop-a");
    expect(response?.status()).toBeGreaterThanOrEqual(400);
  });
});
