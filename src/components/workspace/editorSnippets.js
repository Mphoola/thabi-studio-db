import { snippetCompletion } from "@codemirror/autocomplete";
import { EditorState } from "@codemirror/state";
import { useLibrary } from "../../composables/useLibrary.js";

/**
 * Saved queries with a keyword expand in the editor: type the keyword and
 * pick it from autocomplete. `${name}` in the saved SQL becomes a field to
 * fill in, Tab moving between them.
 */
function snippetSource(context) {
  const word = context.matchBefore(/\w+/);
  if (!word || (word.from === word.to && !context.explicit)) return null;
  const typed = word.text.toLowerCase();
  const options = useLibrary()
    .saved.value.filter(
      (q) => q.keyword && q.keyword.toLowerCase().startsWith(typed),
    )
    .map((q) =>
      snippetCompletion(q.queryText, {
        label: q.keyword,
        detail: q.name,
        type: "text",
        boost: 99,
      }),
    );
  return options.length ? { from: word.from, options } : null;
}

/** Adds saved-query snippets to whatever the language completes. */
export function snippetCompletions() {
  return EditorState.languageData.of(() => [{ autocomplete: snippetSource }]);
}
