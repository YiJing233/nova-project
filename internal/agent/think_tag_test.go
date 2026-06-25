package agent

import (
	"strings"
	"testing"
)

func TestThinkTagExtractor_Basic(t *testing.T) {
	var e thinkTagExtractor
	cParts, tParts := e.feed("你好<think>这是思考</think>世界")
	ec, et := e.flush()
	cParts = append(cParts, ec...)
	tParts = append(tParts, et...)

	content := strings.Join(cParts, "")
	thinking := strings.Join(tParts, "")

	if content != "你好世界" {
		t.Fatalf("content = %q, want %q", content, "你好世界")
	}
	if thinking != "这是思考" {
		t.Fatalf("thinking = %q, want %q", thinking, "这是思考")
	}
}

func TestThinkTagExtractor_NoTags(t *testing.T) {
	var e thinkTagExtractor
	cParts, tParts := e.feed("普通内容没有标签")
	ec, et := e.flush()
	cParts = append(cParts, ec...)
	tParts = append(tParts, et...)

	content := strings.Join(cParts, "")
	thinking := strings.Join(tParts, "")

	if content != "普通内容没有标签" {
		t.Fatalf("content = %q, want %q", content, "普通内容没有标签")
	}
	if thinking != "" {
		t.Fatalf("thinking = %q, want empty", thinking)
	}
}

func TestThinkTagExtractor_Streaming(t *testing.T) {
	var e thinkTagExtractor

	cParts, tParts := e.feed("开始<thi")
	content := strings.Join(cParts, "")
	if content != "开始" {
		t.Fatalf("chunk1 content = %q, want %q", content, "开始")
	}

	cParts2, tParts2 := e.feed("nk>思考中</")
	cParts = append(cParts, cParts2...)
	tParts = append(tParts, tParts2...)

	cParts3, tParts3 := e.feed("think>结束")
	cParts = append(cParts, cParts3...)
	tParts = append(tParts, tParts3...)

	ec, et := e.flush()
	cParts = append(cParts, ec...)
	tParts = append(tParts, et...)

	content = strings.Join(cParts, "")
	thinking := strings.Join(tParts, "")

	if content != "开始结束" {
		t.Fatalf("content = %q, want %q", content, "开始结束")
	}
	if thinking != "思考中" {
		t.Fatalf("thinking = %q, want %q", thinking, "思考中")
	}
}

func TestThinkTagExtractor_UnclosedThink(t *testing.T) {
	var e thinkTagExtractor
	cParts, tParts := e.feed("你好<think>未闭合的思考")
	ec, et := e.flush()
	cParts = append(cParts, ec...)
	tParts = append(tParts, et...)

	content := strings.Join(cParts, "")
	thinking := strings.Join(tParts, "")

	if content != "你好" {
		t.Fatalf("content = %q, want %q", content, "你好")
	}
	if thinking != "未闭合的思考" {
		t.Fatalf("thinking = %q, want %q", thinking, "未闭合的思考")
	}
}

func TestStripThinkTags(t *testing.T) {
	content, thinking := stripThinkTags("答案是<think>让我想想...42</think>42")
	if content != "答案是42" {
		t.Fatalf("content = %q, want %q", content, "答案是42")
	}
	if thinking != "让我想想...42" {
		t.Fatalf("thinking = %q, want %q", thinking, "让我想想...42")
	}
}

func TestStripThinkTags_NoTags(t *testing.T) {
	content, thinking := stripThinkTags("直接回答")
	if content != "直接回答" {
		t.Fatalf("content = %q, want %q", content, "直接回答")
	}
	if thinking != "" {
		t.Fatalf("thinking = %q, want empty", thinking)
	}
}

func TestStripThinkTags_ReasoningContentPresent(t *testing.T) {
	content, thinking := stripThinkTags("<think>思考</think>正文")
	if content != "正文" {
		t.Fatalf("content = %q, want %q", content, "正文")
	}
	if thinking != "思考" {
		t.Fatalf("thinking = %q, want %q", thinking, "思考")
	}
}

func TestStripThinkTags_OrphanCloseTag(t *testing.T) {
	// MiniMax 模式：无 <think> 开始标签，思考前言仅以 </think> 收尾。
	content, thinking := stripThinkTags("tags\n\nLet me think about this.</think>\n\n正文内容")
	if content != "正文内容" {
		t.Fatalf("content = %q, want %q", content, "正文内容")
	}
	if !strings.Contains(thinking, "Let me think") {
		t.Fatalf("thinking = %q, want to contain prelude", thinking)
	}
}

func TestStripThinkTags_NoOrphanFalsePositive(t *testing.T) {
	// 正常正文不含 </think>，不应被改动。
	content, thinking := stripThinkTags("正常的回答，没有思考标签。")
	if content != "正常的回答，没有思考标签。" || thinking != "" {
		t.Fatalf("unexpected content=%q thinking=%q", content, thinking)
	}
}
