package agent

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"regexp"
	"strings"

	"github.com/cloudwego/eino-ext/components/model/openai"
	"github.com/cloudwego/eino/components/model"
	"github.com/cloudwego/eino/schema"
)

// MiniMax 等模型不返回标准 tool_calls，而是把工具调用以 antml 风格的 XML 文本写入 content，
// 并夹带内部特殊 token（如 ]<]minimax[>[）。minimaxToolCallModel 在模型输出层解析这些文本
// 工具调用，填充为标准 schema.ToolCall，并清理特殊 token 与 <think> 思考标签，使上层框架能
// 正常识别并执行工具。仅用于需要真正执行工具的创作类 Agent；流式下会累积完整输出后再修复，
// 因此正文不再逐字流式（这是让 MiniMax 工具调用可用的必要取舍）。
type minimaxToolCallModel struct {
	inner model.ToolCallingChatModel
}

func wrapMinimaxToolCalls(inner model.ToolCallingChatModel) *minimaxToolCallModel {
	return &minimaxToolCallModel{inner: inner}
}

func (m *minimaxToolCallModel) Generate(ctx context.Context, input []*schema.Message, opts ...model.Option) (*schema.Message, error) {
	msg, err := m.inner.Generate(ctx, input, opts...)
	if err != nil || msg == nil {
		return msg, err
	}
	repairMinimaxMessage(msg)
	return msg, nil
}

func (m *minimaxToolCallModel) Stream(ctx context.Context, input []*schema.Message, opts ...model.Option) (*schema.StreamReader[*schema.Message], error) {
	sr, err := m.inner.Stream(ctx, input, opts...)
	if err != nil {
		return nil, err
	}
	defer sr.Close()

	var frames []*schema.Message
	for {
		frame, recvErr := sr.Recv()
		if errors.Is(recvErr, io.EOF) {
			break
		}
		if recvErr != nil {
			return nil, recvErr
		}
		if frame != nil {
			frames = append(frames, frame)
		}
	}
	if len(frames) == 0 {
		return schema.StreamReaderFromArray([]*schema.Message{}), nil
	}
	full, concatErr := schema.ConcatMessages(frames)
	if concatErr != nil || full == nil {
		// 拼接失败时原样回放，避免吞掉模型输出。
		return schema.StreamReaderFromArray(frames), nil
	}
	repairMinimaxMessage(full)
	return schema.StreamReaderFromArray([]*schema.Message{full}), nil
}

func (m *minimaxToolCallModel) WithTools(tools []*schema.ToolInfo) (model.ToolCallingChatModel, error) {
	inner, err := m.inner.WithTools(tools)
	if err != nil {
		return nil, err
	}
	return &minimaxToolCallModel{inner: inner}, nil
}

var (
	minimaxMarkerRe   = regexp.MustCompile(`\]<\]minimax\[>\[`)
	xmlInvokeRe       = regexp.MustCompile(`(?s)<invoke\s+name="([^"]+)"\s*>(.*?)</invoke>`)
	xmlToolCallWrapRe = regexp.MustCompile(`(?s)<tool_call>(.*?)</tool_call>`)
	xmlParamNamedRe   = regexp.MustCompile(`(?s)<parameter\s+name="([^"]+)"\s*>(.*?)</parameter>`)
	xmlParamTagRe     = regexp.MustCompile(`(?s)<([a-zA-Z_][\w.-]*)>(.*?)</[a-zA-Z_][\w.-]*>`)
)

// repairMinimaxMessage 原地修复 MiniMax 的非标准输出：清除特殊 token、解析文本工具调用、剥离思考。
func repairMinimaxMessage(msg *schema.Message) {
	if msg == nil {
		return
	}
	content := minimaxMarkerRe.ReplaceAllString(msg.Content, "")
	if len(msg.ToolCalls) == 0 {
		if calls, cleaned, ok := parseMinimaxToolCalls(content); ok {
			msg.ToolCalls = calls
			content = cleaned
		}
	}
	clean, thinking := stripThinkTags(content)
	if strings.TrimSpace(thinking) != "" && strings.TrimSpace(msg.ReasoningContent) == "" {
		msg.ReasoningContent = thinking
	}
	msg.Content = strings.TrimSpace(clean)
}

// parseMinimaxToolCalls 从文本中解析 antml 风格工具调用，返回工具调用与剔除调用片段后的正文。
func parseMinimaxToolCalls(content string) ([]schema.ToolCall, string, bool) {
	invokes := xmlInvokeRe.FindAllStringSubmatch(content, -1)
	if len(invokes) == 0 {
		return nil, content, false
	}
	calls := make([]schema.ToolCall, 0, len(invokes))
	for i, sm := range invokes {
		name := strings.TrimSpace(sm[1])
		if name == "" {
			continue
		}
		params := parseMinimaxInvokeParams(sm[2])
		argsJSON, err := json.Marshal(params)
		if err != nil {
			argsJSON = []byte("{}")
		}
		idx := i
		calls = append(calls, schema.ToolCall{
			Index: &idx,
			ID:    fmt.Sprintf("minimax_call_%d", i),
			Type:  "function",
			Function: schema.FunctionCall{
				Name:      name,
				Arguments: string(argsJSON),
			},
		})
	}
	if len(calls) == 0 {
		return nil, content, false
	}
	cleaned := xmlToolCallWrapRe.ReplaceAllString(content, "")
	cleaned = xmlInvokeRe.ReplaceAllString(cleaned, "")
	return calls, cleaned, true
}

// parseMinimaxInvokeParams 解析 <invoke> 内的参数，兼容 <parameter name="x">v</parameter> 与 <x>v</x> 两种写法。
func parseMinimaxInvokeParams(body string) map[string]string {
	params := map[string]string{}
	if named := xmlParamNamedRe.FindAllStringSubmatch(body, -1); len(named) > 0 {
		for _, sm := range named {
			if key := strings.TrimSpace(sm[1]); key != "" {
				params[key] = strings.TrimSpace(sm[2])
			}
		}
		return params
	}
	for _, sm := range xmlParamTagRe.FindAllStringSubmatch(body, -1) {
		key := strings.TrimSpace(sm[1])
		if key == "" || strings.EqualFold(key, "parameter") {
			continue
		}
		params[key] = strings.TrimSpace(sm[2])
	}
	return params
}

// isMinimaxModel 根据模型名或 base_url 判断是否为 MiniMax 端点。
func isMinimaxModel(cfg openai.ChatModelConfig) bool {
	return strings.Contains(strings.ToLower(cfg.BaseURL), "minimax") ||
		strings.Contains(strings.ToLower(cfg.Model), "minimax")
}
