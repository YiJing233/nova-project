package agent

import (
	"github.com/cloudwego/eino-ext/components/model/openai"

	"nova/config"
)

func chatModelConfigForAgent(cfg *config.Config, agentKind string) openai.ChatModelConfig {
	resolved := config.ResolveAgentModel(cfg, agentKind)
	modelCfg := openai.ChatModelConfig{
		APIKey:  resolved.OpenAIAPIKey,
		Model:   resolved.OpenAIModel,
		BaseURL: resolved.OpenAIBaseURL,
	}
	if resolved.Temperature != nil {
		temperature := float32(*resolved.Temperature)
		modelCfg.Temperature = &temperature
	}
	extraFields := map[string]any{}
	if resolved.EnableThinking != nil {
		extraFields["enable_thinking"] = *resolved.EnableThinking
	}
	// MiniMax-M3 默认把思考写入 content 的 <think> 标签；reasoning_split=true 让其改用标准
	// reasoning_content 字段返回，从根本上避免 <think> 泄漏到正文（见 MiniMax OpenAI 兼容文档）。
	if isMinimaxModel(modelCfg) {
		extraFields["reasoning_split"] = true
	}
	if len(extraFields) > 0 {
		modelCfg.ExtraFields = extraFields
	}
	if resolved.ReasoningEffort != "" {
		modelCfg.ReasoningEffort = openai.ReasoningEffortLevel(resolved.ReasoningEffort)
	}
	return modelCfg
}
