<script setup>
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import {
  autocompletion,
  closeBrackets,
  closeBracketsKeymap,
  completionKeymap,
} from "@codemirror/autocomplete";
import {
  defaultKeymap,
  history,
  historyKeymap,
  indentWithTab,
} from "@codemirror/commands";
import {
  bracketMatching,
  indentOnInput,
  indentUnit,
} from "@codemirror/language";
import { MSSQL, MySQL, PostgreSQL, sql } from "@codemirror/lang-sql";
import { highlightSelectionMatches, searchKeymap } from "@codemirror/search";
import { Compartment, EditorState, Prec } from "@codemirror/state";
import {
  EditorView,
  drawSelection,
  highlightActiveLine,
  highlightActiveLineGutter,
  keymap,
  lineNumbers,
} from "@codemirror/view";
import { splitStatements, statementAt } from "../../lib/sqlStatements.js";
import { formatSql, minifySql } from "../../lib/sqlFormat.js";
import { useAppSettings } from "../../composables/useAppSettings.js";
import { useToasts } from "../../composables/useToasts.js";
import {
  defaultSchemaFor,
  toEditorSchema,
} from "../../composables/useCompletionSchema.js";
import { snippetCompletions } from "./editorSnippets.js";
import { editorHighlighting, editorTheme } from "./editorTheme.js";

/**
 * SQL editor (CodeMirror 6). Emits run/runAll/cancel from its shortcuts.
 * Autocomplete knows the tab's tables and columns (`tables`) and saved
 * snippets; editor preferences come from Settings.
 */
const props = defineProps({
  engine: { type: String, default: "postgres" },
  label: { type: String, default: "SQL editor" },
  /** [{ schema, table, columns }] for autocomplete. */
  tables: { type: Array, default: () => [] },
});
const emit = defineEmits(["run", "run-all", "cancel"]);
const model = defineModel({ type: String, default: "" });

const host = ref(null);
let view = null;
const dialect = new Compartment();
const preferences = new Compartment();
const keyBindings = new Compartment();
const dialects = { postgres: PostgreSQL, mysql: MySQL, sqlserver: MSSQL };
const { settings } = useAppSettings();
const toasts = useToasts();

/** The SQL language with this engine's dialect and the known tables. */
function language() {
  return sql({
    dialect: dialects[props.engine] ?? PostgreSQL,
    upperCaseKeywords: settings.value.editorKeywordCase !== "lower",
    schema: toEditorSchema(props.tables),
    defaultSchema: defaultSchemaFor(props.engine),
  });
}

function preferenceExtensions() {
  const size = settings.value.editorTabSize || 2;
  return [
    EditorState.tabSize.of(size),
    indentUnit.of(" ".repeat(size)),
    settings.value.editorWordWrap ? EditorView.lineWrapping : [],
  ];
}

/** Replaces the selection, or the whole text, with `transform(text)`. */
function rewrite(transform) {
  if (!view) return false;
  const { from, to } = view.state.selection.main;
  const whole = from === to;
  const range = whole ? { from: 0, to: view.state.doc.length } : { from, to };
  const original = view.state.sliceDoc(range.from, range.to);
  Promise.resolve()
    .then(() => transform(original))
    .then((text) => {
      // Only if nothing was typed meanwhile: the range must still hold it.
      if (view?.state.sliceDoc(range.from, range.to) === original)
        view.dispatch({ changes: { ...range, insert: text } });
    })
    .catch((error) =>
      toasts.error(
        "Couldn’t format this SQL",
        error?.message?.split("\n")[0] ?? "",
      ),
    );
  return true;
}
const format = () =>
  rewrite((text) =>
    formatSql(text, props.engine, {
      keywordCase: settings.value.editorKeywordCase,
      tabWidth: settings.value.editorTabSize || 2,
    }),
  );
const minify = () => rewrite(minifySql);

const shortcuts = Prec.highest(
  keymap.of([
    { key: "Mod-Enter", run: () => (emit("run"), true) },
    { key: "Shift-Mod-Enter", run: () => (emit("run-all"), true) },
    { key: "Mod-.", run: () => (emit("cancel"), true) },
    { key: "Mod-i", run: format },
    { key: "Shift-Mod-i", run: minify },
  ]),
);

onMounted(() => {
  view = new EditorView({
    parent: host.value,
    state: EditorState.create({
      doc: model.value,
      extensions: [
        // Vim must come first so its keys win when it is on.
        keyBindings.of([]),
        shortcuts,
        preferences.of(preferenceExtensions()),
        snippetCompletions(),
        // The suggestion popup itself: tables, columns (through aliases
        // like `a.`), keywords and snippets, as you type.
        autocompletion({ icons: false }),
        lineNumbers(),
        highlightActiveLineGutter(),
        highlightActiveLine(),
        history(),
        drawSelection(),
        indentOnInput(),
        bracketMatching(),
        closeBrackets(),
        highlightSelectionMatches(),
        dialect.of(language()),
        editorTheme,
        editorHighlighting,
        keymap.of([
          ...closeBracketsKeymap,
          ...completionKeymap,
          ...defaultKeymap,
          ...searchKeymap,
          ...historyKeymap,
          indentWithTab,
        ]),
        EditorView.contentAttributes.of({ "aria-label": props.label }),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) model.value = update.state.doc.toString();
        }),
      ],
    }),
  });
  if (settings.value.editorVim) applyVim(true);
});

watch(model, (value) => {
  if (view && value !== view.state.doc.toString())
    view.dispatch({
      changes: { from: 0, to: view.state.doc.length, insert: value },
    });
});

watch(
  () => [props.engine, props.tables, settings.value.editorKeywordCase],
  () => view?.dispatch({ effects: dialect.reconfigure(language()) }),
);
watch(
  () => [settings.value.editorTabSize, settings.value.editorWordWrap],
  () =>
    view?.dispatch({
      effects: preferences.reconfigure(preferenceExtensions()),
    }),
);
/** Vim bindings, loaded only when turned on (they're a sizeable module). */
async function applyVim(on) {
  const bindings = on ? (await import("@replit/codemirror-vim")).vim() : [];
  view?.dispatch({ effects: keyBindings.reconfigure(bindings) });
}
watch(() => settings.value.editorVim, applyVim);

onBeforeUnmount(() => view?.destroy());

/**
 * What Run should execute: the selection's statements, or the statement
 * under the cursor. With `all`, every statement in the editor.
 * @returns {string[]}
 */
function runTarget(all = false) {
  if (!view) return [];
  const doc = view.state.doc.toString();
  const { from, to } = view.state.selection.main;
  if (all) return splitStatements(doc, props.engine).map((s) => s.text);
  if (from !== to)
    return splitStatements(doc.slice(from, to), props.engine).map(
      (s) => s.text,
    );
  const statement = statementAt(splitStatements(doc, props.engine), from);
  return statement ? [statement.text] : [];
}

function focus() {
  view?.focus();
}

/** The statement under the cursor (or selected), for Explain. */
function currentStatement() {
  return runTarget(false)[0] ?? "";
}

/** Opens `text` in the editor, replacing what's there (Open SQL File). */
function setText(text) {
  model.value = text;
}

defineExpose({ runTarget, focus, format, minify, currentStatement, setText });
</script>

<template>
  <div ref="host" class="h-full min-h-0 overflow-hidden" />
</template>
