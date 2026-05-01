class LoginRequest:
    token: str


class LoginResponse:
    user_id: str


def schema_version() -> int:
    return 1
