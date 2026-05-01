import { LoginDto } from "../lib/auth.schema";

export default function Page() {
  const dto: LoginDto = { token: "demo" };
  return <main>{dto.token}</main>;
}
