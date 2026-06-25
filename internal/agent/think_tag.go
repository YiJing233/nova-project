package agent

import (
	"strings"
)

const (
	thinkOpenTag  = "<think>"
	thinkCloseTag = "</think>"
)

type thinkTagState int

const (
	thinkStateNormal thinkTagState = iota
	thinkStateInThink
	thinkStateInOpenTag
	thinkStateInCloseTag
)

type thinkTagExtractor struct {
	state       thinkTagState
	thinkBuf    strings.Builder
	contentBuf  strings.Builder
	openTagBuf  strings.Builder
	closeTagBuf strings.Builder
}

func (e *thinkTagExtractor) reset() {
	e.state = thinkStateNormal
	e.thinkBuf.Reset()
	e.contentBuf.Reset()
	e.openTagBuf.Reset()
	e.closeTagBuf.Reset()
}

func (e *thinkTagExtractor) feed(input string) (contentParts, thinkingParts []string) {
	if input == "" {
		return nil, nil
	}

	for i := 0; i < len(input); i++ {
		ch := input[i]

		switch e.state {
		case thinkStateNormal:
			if ch == '<' {
				e.openTagBuf.Reset()
				e.openTagBuf.WriteByte(ch)
				e.state = thinkStateInOpenTag
			} else {
				e.contentBuf.WriteByte(ch)
			}

		case thinkStateInOpenTag:
			e.openTagBuf.WriteByte(ch)
			openStr := e.openTagBuf.String()
			if strings.HasPrefix(thinkOpenTag, openStr) {
				if openStr == thinkOpenTag {
					e.flushContent(&contentParts)
					e.state = thinkStateInThink
				}
			} else {
				e.contentBuf.WriteString(openStr)
				e.openTagBuf.Reset()
				e.state = thinkStateNormal
			}

		case thinkStateInThink:
			if ch == '<' {
				e.closeTagBuf.Reset()
				e.closeTagBuf.WriteByte(ch)
				e.state = thinkStateInCloseTag
			} else {
				e.thinkBuf.WriteByte(ch)
			}

		case thinkStateInCloseTag:
			e.closeTagBuf.WriteByte(ch)
			closeStr := e.closeTagBuf.String()
			if strings.HasPrefix(thinkCloseTag, closeStr) {
				if closeStr == thinkCloseTag {
					e.flushThinking(&thinkingParts)
					e.state = thinkStateNormal
				}
			} else {
				e.thinkBuf.WriteString(closeStr)
				e.closeTagBuf.Reset()
				e.state = thinkStateInThink
			}
		}
	}

	e.flushContent(&contentParts)
	return contentParts, thinkingParts
}

func (e *thinkTagExtractor) flush() (contentParts, thinkingParts []string) {
	e.flushContent(&contentParts)

	switch e.state {
	case thinkStateInOpenTag:
		if e.openTagBuf.Len() > 0 {
			contentParts = append(contentParts, e.openTagBuf.String())
			e.openTagBuf.Reset()
		}
	case thinkStateInThink:
		if e.thinkBuf.Len() > 0 {
			thinkingParts = append(thinkingParts, e.thinkBuf.String())
			e.thinkBuf.Reset()
		}
	case thinkStateInCloseTag:
		if e.closeTagBuf.Len() > 0 {
			e.thinkBuf.WriteString(e.closeTagBuf.String())
			e.closeTagBuf.Reset()
		}
		if e.thinkBuf.Len() > 0 {
			thinkingParts = append(thinkingParts, e.thinkBuf.String())
			e.thinkBuf.Reset()
		}
	}

	e.state = thinkStateNormal
	return contentParts, thinkingParts
}

func (e *thinkTagExtractor) flushContent(parts *[]string) {
	if e.contentBuf.Len() > 0 {
		*parts = append(*parts, e.contentBuf.String())
		e.contentBuf.Reset()
	}
}

func (e *thinkTagExtractor) flushThinking(parts *[]string) {
	if e.thinkBuf.Len() > 0 {
		*parts = append(*parts, e.thinkBuf.String())
		e.thinkBuf.Reset()
	}
}

func stripThinkTags(text string) (content, thinking string) {
	var extractor thinkTagExtractor
	cParts, tParts := extractor.feed(text)
	extraC, extraT := extractor.flush()
	cParts = append(cParts, extraC...)
	tParts = append(tParts, extraT...)
	content = strings.Join(cParts, "")
	thinking = strings.Join(tParts, "")
	// 兼容无 <think> 开始标签、思考正文直接出现并仅以 </think> 收尾的模型（如 MiniMax）。
	content, thinking = stripOrphanCloseThink(content, thinking)
	return content, thinking
}

// stripOrphanCloseThink 处理 content 中残留的孤立 </think>：
// 部分模型不输出 <think> 开始标签，思考正文直接出现、仅以 </think> 收尾，
// 配对解析无法剥离，此时把首个 </think> 之前的内容归为思考。
func stripOrphanCloseThink(content, thinking string) (string, string) {
	idx := indexFold(content, thinkCloseTag)
	if idx < 0 {
		return content, thinking
	}
	prelude := content[:idx]
	rest := content[idx+len(thinkCloseTag):]
	if strings.TrimSpace(prelude) != "" {
		if thinking != "" {
			thinking += "\n"
		}
		thinking += prelude
	}
	content = strings.TrimLeft(rest, " \t\r\n")
	return content, thinking
}

// indexFold 大小写不敏感地查找子串首次出现的位置。
func indexFold(s, sub string) int {
	return strings.Index(strings.ToLower(s), strings.ToLower(sub))
}
