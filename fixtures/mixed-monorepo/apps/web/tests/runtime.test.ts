import { attachRuntime } from "../src/runtime";

test("runtime attaches route", () => {
  expect(attachRuntime({ get() {} })).toBeUndefined();
});
