from fastapi import FastAPI

from app.schemas import LoginRequest, LoginResponse

app = FastAPI()
prefix = "/tenant"


@app.post("/auth/login")
def login(payload: LoginRequest) -> LoginResponse:
    return LoginResponse()


@app.get(prefix + "/profile")
def dynamic_profile():
    return {"ok": True}


def main():
    return app
