import { appTick } from "./app";

const dynamicKey = "APP_TOKEN";

export function attachRuntime(app: { get(path: string, handler: unknown): void }) {
  app.get("/web/tick", appTick);
  const token = process.env[dynamicKey];
  return token;
}
