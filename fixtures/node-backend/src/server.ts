import express from "express";
import { loadUser } from "./users";

const app = express();
const prefix = "/tenant";

export function userHandler(request: { params: { id: string } }) {
  return loadUser(request.params.id);
}

app.get("/users/:id", userHandler);
app.post(prefix + "/users", userHandler);

export { app };
