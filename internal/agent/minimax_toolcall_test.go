package agent

import (
	"strings"
	"testing"

	"github.com/cloudwego/eino-ext/components/model/openai"
	"github.com/cloudwego/eino/schema"
)

func TestRepairMinimaxMessage_RealToolCall(t *testing.T) {
	// 复刻真实 MiniMax-M3 session 的输出结构：think + 文本工具调用 + ]<]minimax[>[ 特殊 token。
	content := "<think>The user is invoking rewrite. Let me load the skill.</think>\n\n" +
		"加载 `rewrite` skill 的具体流程。<tool_call>\n" +
		"<invoke name=\"skill\">]<]minimax[>[<skill>rewrite]<]minimax[>[</skill>]<]minimax[>[</invoke>\n" +
		"]<]minimax[>[</tool_call>"
	msg := &schema.Message{Role: schema.Assistant, Content: content}
	repairMinimaxMessage(msg)

	if len(msg.ToolCalls) != 1 {
		t.Fatalf("expected 1 tool call, got %d (%#v)", len(msg.ToolCalls), msg.ToolCalls)
	}
	tc := msg.ToolCalls[0]
	if tc.Function.Name != "skill" {
		t.Fatalf("tool name = %q, want skill", tc.Function.Name)
	}
	if tc.Function.Arguments != `{"skill":"rewrite"}` {
		t.Fatalf("args = %q, want {\"skill\":\"rewrite\"}", tc.Function.Arguments)
	}
	if strings.Contains(msg.Content, "minimax") || strings.Contains(msg.Content, "<tool_call") || strings.Contains(msg.Content, "<invoke") {
		t.Fatalf("content still leaks markers: %q", msg.Content)
	}
	if msg.Content != "加载 `rewrite` skill 的具体流程。" {
		t.Fatalf("content = %q", msg.Content)
	}
	if !strings.Contains(msg.ReasoningContent, "load the skill") {
		t.Fatalf("reasoning not captured: %q", msg.ReasoningContent)
	}
}

func TestRepairMinimaxMessage_ParameterStyle(t *testing.T) {
	// 兼容 antml 的 <parameter name="x"> 写法。
	content := "正文。<invoke name=\"read_file\"><parameter name=\"file_path\">ch1.md</parameter></invoke>"
	msg := &schema.Message{Role: schema.Assistant, Content: content}
	repairMinimaxMessage(msg)
	if len(msg.ToolCalls) != 1 || msg.ToolCalls[0].Function.Name != "read_file" {
		t.Fatalf("unexpected tool calls: %#v", msg.ToolCalls)
	}
	if msg.ToolCalls[0].Function.Arguments != `{"file_path":"ch1.md"}` {
		t.Fatalf("args = %q", msg.ToolCalls[0].Function.Arguments)
	}
}

func TestRepairMinimaxMessage_NoToolCall(t *testing.T) {
	// 普通正文（无工具调用），仅清理 think，不破坏内容。
	msg := &schema.Message{Role: schema.Assistant, Content: "<think>思考</think>这是正常回答。"}
	repairMinimaxMessage(msg)
	if len(msg.ToolCalls) != 0 {
		t.Fatalf("unexpected tool calls: %d", len(msg.ToolCalls))
	}
	if msg.Content != "这是正常回答。" {
		t.Fatalf("content = %q", msg.Content)
	}
}

func TestRepairMinimaxMessage_PreservesNativeToolCalls(t *testing.T) {
	idx := 0
	msg := &schema.Message{
		Role:    schema.Assistant,
		Content: "正文",
		ToolCalls: []schema.ToolCall{{
			Index: &idx, ID: "x", Type: "function",
			Function: schema.FunctionCall{Name: "read_file", Arguments: "{}"},
		}},
	}
	repairMinimaxMessage(msg)
	if len(msg.ToolCalls) != 1 || msg.ToolCalls[0].Function.Name != "read_file" {
		t.Fatalf("native tool calls altered: %#v", msg.ToolCalls)
	}
}

func TestIsMinimaxModel(t *testing.T) {
	if !isMinimaxModel(openai.ChatModelConfig{BaseURL: "https://minimaxi.com/v1/", Model: "MiniMax-M3"}) {
		t.Fatal("expected minimax detection")
	}
	if isMinimaxModel(openai.ChatModelConfig{BaseURL: "https://api.openai.com/v1", Model: "gpt-4o"}) {
		t.Fatal("false positive for openai")
	}
}
