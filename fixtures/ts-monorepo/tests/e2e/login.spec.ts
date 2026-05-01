import { test, expect } from "@playwright/test";

test("login route opens", async ({ page }) => {
  await page.goto("/auth/login");
  await expect(page).toHaveURL("/auth/login");
});
