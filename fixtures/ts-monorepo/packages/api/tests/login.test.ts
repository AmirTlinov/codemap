import { loginHandler } from "../src/server";

test("login maps token", () => {
  expect(loginHandler({ token: "u1" }).userId).toBe("u1");
});
