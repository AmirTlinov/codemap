import { test, expect } from "@playwright/test";

test("login endpoint is reachable", async ({ page }) => {
  await page.goto("/api/login");
  await expect(page).toHaveURL("/api/login");
});
