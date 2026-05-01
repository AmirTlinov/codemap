import { loadUser } from "../src/users";

test("loads user", () => {
  expect(loadUser("42").id).toBe("42");
});
