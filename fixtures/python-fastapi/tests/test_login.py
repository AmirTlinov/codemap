from app.main import login
from app.schemas import LoginRequest


def test_login():
    assert login(LoginRequest()) is not None
