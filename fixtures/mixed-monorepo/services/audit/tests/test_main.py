import subprocess
from audit.main import AuditEvent, record

def test_record_and_external_contract():
    subprocess.run(["bash", "scripts/verify-events.sh"], check=True)
    assert record(AuditEvent("event.created")) == "event.created"
