package main

import "example.com/codemapfixture/internal/api"

func main() {
	api.RegisterRoutes(api.NewRouter())
}
