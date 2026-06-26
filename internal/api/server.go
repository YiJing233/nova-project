package api

import (
	"fmt"

	hertzserver "github.com/cloudwego/hertz/pkg/app/server"

	"nova/config"
	"nova/internal/api/handlers"
	"nova/internal/app"
)

// Server 包含 Hertz 引擎和应用运行时。
type Server struct {
	engine *hertzserver.Hertz
	app    *app.App
	port   string
	host   string
}

// NewServer 构造 HTTP 服务。
func NewServer(application *app.App, port string) *Server {
	return NewServerWithHost(application, port, "")
}

// NewServerWithHost 构造 HTTP 服务并指定监听地址。
func NewServerWithHost(application *app.App, port string, host string) *Server {
	remoteAccess := application.RemoteAccessConfig()
	if host == "" {
		host = config.HTTPListenHost(remoteAccess.AllowLANAccess)
	}
	s := &Server{
		app:  application,
		port: port,
		host: host,
	}

	h := hertzserver.Default(
		hertzserver.WithHostPorts(host+":"+port),
		hertzserver.WithMaxRequestBodySize(int(handlers.MaxCharacterCardUploadBytes)),
	)
	h.Use(corsMiddleware)
	h.Use(remoteAccessMiddleware(application))
	s.registerRoutes(h)
	s.engine = h
	return s
}

// Run 启动 HTTP 服务。
func (s *Server) Run() {
	fmt.Printf("Nova HTTP 服务启动: http://%s:%s\n", s.host, s.port)
	s.engine.Spin()
}

// RunWithHost 保持兼容性，直接启动。
func (s *Server) RunWithHost(host string) {
	s.Run()
}
