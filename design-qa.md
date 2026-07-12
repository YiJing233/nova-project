# New Story Setup Design QA

- Source visual truth: `/Users/bytedance/.codex/generated_images/019f5726-da09-7001-bdb9-d45d97771d0c/exec-e09eaf21-07e7-4f78-945e-a756d4b65470.png`
- Implementation screenshot: `/tmp/denova-new-story-wide.png`
- Comparison board: `/Users/bytedance/.codex/generated_images/019f5726-da09-7001-bdb9-d45d97771d0c/exec-5dd0d50f-c3fb-4746-bedd-99b4987a8677.png`
- Viewport: desktop 1440 × 1024; responsive check at 390 × 844
- State: new-story setup, inherited module summary expanded for desktop; default inherited state checked on mobile

## Full-view comparison evidence

The implementation preserves the selected mock's editorial single-column form, pure-black Denova shell, compact top controls, restrained gold primary action, six-module summary, and existing Director Console. The implemented form uses the product's existing spacing and control tokens rather than copying generated-image artifacts.

## Focused region evidence

The form and module region remained readable in the full comparison board, so no additional crop was required. The 390 × 844 capture separately verified the field stack, module summary, hidden message composer, and absence of horizontal overflow (`scrollWidth === clientWidth === 390`).

## Findings

- No actionable P0/P1/P2 mismatches remain.
- P3: the source mock uses pictorial module tiles while the implementation uses lighter text rows with dividers. This is intentional: the existing Denova design system favors lower-density separators and it keeps the advanced section from competing with the story fields.
- P3: the implementation adds the existing “故事起点” eyebrow used by the opening screen, strengthening continuity between setup and opening.

## Required fidelity surfaces

- Fonts and typography: existing Inter variable font, weights, line heights, labels, and muted helper copy match the surrounding Denova product.
- Spacing and layout rhythm: single-column hierarchy, two-column director/length row, section divider, and responsive single-column collapse pass.
- Colors and visual tokens: existing surface, border, text, accent, danger, focus, and hover tokens are reused; no gradients or foreign palette introduced.
- Image and asset fidelity: no new raster assets are required; standard interface icons use the existing Lucide dependency.
- Copy and content: Chinese and English strings cover title, explanation, fields, module inheritance, customization, disabled/default states, and continuation.

## Interaction checks

- New opens the inline setup instead of a popover.
- Module customization expands inline and exposes per-module selectors.
- Cancel restores the previous empty-story opening without creating a placeholder.
- The message composer is hidden during setup and no longer obscures the footer.
- Opening tabs center each visible icon/label group inside its equal-width third; the optional book-preset count stays adjacent without shifting the other tabs.
- Back to Setup restores the existing empty story's name, brief, director, reply length, and module selections without creating another story.
- Browser console reported no errors.

## Comparison history

1. Initial browser capture found a P1 overlap: the floating message composer covered the setup footer.
2. The composer is now removed from the DOM while the new-story draft is active.
3. Desktop and mobile recaptures confirm the footer/content region is unobstructed and the narrow layout has no horizontal overflow.

## Implementation checklist

- [x] Inline stage setup
- [x] Deferred story creation
- [x] Story-level module overrides
- [x] Default director inheritance
- [x] Desktop and mobile layouts
- [x] Cancel and error states
- [x] Bilingual copy

final result: passed
