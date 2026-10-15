import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { EditorView } from "@codemirror/view";
import { tags } from "@lezer/highlight";

/**
 * Editor colours come from the design tokens (CSS variables), so the editor
 * follows light and dark mode with no colours of its own.
 */
export const editorTheme = EditorView.theme({
  "&": {
    height: "100%",
    fontSize: "12.5px",
    backgroundColor: "var(--canvas)",
    color: "var(--fg)",
  },
  ".cm-scroller": {
    fontFamily:
      "var(--font-mono, ui-monospace, SFMono-Regular, Menlo, monospace)",
    lineHeight: "20px",
  },
  ".cm-content": { caretColor: "var(--accent)", padding: "8px 0" },
  ".cm-cursor, .cm-dropCursor": { borderLeftColor: "var(--accent)" },
  "&.cm-focused": { outline: "none" },
  "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection":
    { backgroundColor: "var(--selection)" },
  ".cm-activeLine": { backgroundColor: "var(--surface-hover)" },
  ".cm-gutters": {
    backgroundColor: "var(--canvas)",
    color: "var(--fg-disabled)",
    border: "none",
  },
  ".cm-activeLineGutter": {
    backgroundColor: "transparent",
    color: "var(--fg-muted)",
  },
  ".cm-lineNumbers .cm-gutterElement": { padding: "0 12px 0 16px" },
  ".cm-matchingBracket": {
    backgroundColor: "var(--accent-subtle)",
    outline: "1px solid var(--accent-muted)",
  },
  ".cm-searchMatch": { backgroundColor: "var(--warning-subtle)" },
  ".cm-searchMatch.cm-searchMatch-selected": {
    backgroundColor: "var(--accent-muted)",
  },
  ".cm-panels": {
    backgroundColor: "var(--toolbar)",
    color: "var(--fg)",
    borderColor: "var(--line)",
  },
  ".cm-panel.cm-search input, .cm-panel.cm-search button": {
    fontFamily: "var(--font-sans)",
    fontSize: "12px",
  },
  ".cm-tooltip": {
    backgroundColor: "var(--surface-raised)",
    color: "var(--fg)",
    border: "1px solid var(--line)",
    borderRadius: "6px",
  },
  ".cm-tooltip-autocomplete > ul > li[aria-selected]": {
    backgroundColor: "var(--accent)",
    color: "var(--accent-fg)",
  },
  ".cm-runTarget": { backgroundColor: "var(--accent-subtle)" },
});

const highlight = HighlightStyle.define([
  {
    tag: [tags.keyword, tags.operatorKeyword, tags.modifier],
    color: "var(--syntax-keyword)",
    fontWeight: "500",
  },
  {
    tag: [tags.string, tags.special(tags.string)],
    color: "var(--syntax-string)",
  },
  { tag: [tags.number, tags.bool, tags.null], color: "var(--syntax-number)" },
  {
    tag: [tags.lineComment, tags.blockComment, tags.comment],
    color: "var(--syntax-comment)",
    fontStyle: "italic",
  },
  {
    tag: [tags.function(tags.variableName), tags.standard(tags.name)],
    color: "var(--syntax-function)",
  },
  { tag: [tags.typeName, tags.className], color: "var(--syntax-type)" },
  {
    tag: [tags.operator, tags.punctuation, tags.bracket],
    color: "var(--syntax-operator)",
  },
  {
    tag: [tags.special(tags.name)],
    color: "var(--fg)",
  },
]);

export const editorHighlighting = syntaxHighlighting(highlight);
