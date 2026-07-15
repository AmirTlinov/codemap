package main

import "net/http"

func events(w http.ResponseWriter, _ *http.Request) { w.WriteHeader(http.StatusOK) }
func main() { http.HandleFunc("/events", events) }
