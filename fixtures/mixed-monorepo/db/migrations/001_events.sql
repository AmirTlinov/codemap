CREATE TABLE events_outbox (
  id TEXT PRIMARY KEY,
  topic TEXT NOT NULL,
  payload JSON NOT NULL
);
