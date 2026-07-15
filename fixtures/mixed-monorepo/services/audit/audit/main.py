from dataclasses import dataclass

@dataclass
class AuditEvent:
    topic: str

def record(event: AuditEvent) -> str:
    return event.topic
