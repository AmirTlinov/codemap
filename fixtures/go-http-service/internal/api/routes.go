package api

type Router interface {
	HandleFunc(path string, handler func()) Route
}

type Route interface {
	Methods(methods ...string) Route
}

func NewRouter() Router {
	return nil
}

func RegisterRoutes(router Router) {
	method := "GET"
	router.HandleFunc("/health", health).Methods("GET")
	router.HandleFunc("/dynamic", dynamic).Methods(method)
}

func health() {}

func dynamic() {}
