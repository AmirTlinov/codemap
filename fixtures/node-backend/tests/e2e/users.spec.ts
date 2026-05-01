import { test, expect } from "@playwright/test";

test("user route opens", async ({ page }) => {
  await page.goto("/users/42");
  await expect(page).toHaveURL("/users/42");
});
