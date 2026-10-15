<script setup>
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { closeBrackets, closeBracketsKeymap } from "@codemirror/autocomplete";
import {
  defaultKeymap,
  history,
  historyKeymap,
  indentWithTab,
} from "@codemirror/commands";
import { bracketMatching, indentOnInput } from "@codemirror/language";
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
import { editorHighlighting, editorTheme } from "./editorTheme.js";

/** SQL editor (CodeMirror 6). Emits run/runAll/cancel from its shortcuts. */
const props = defineProps({
  engine: { type: String, default: "postgres" },
  label: { type: String, default: "SQL editor" },
});
const emit = defineEmits(["run", "run-all", "cancel"]);
const model = defineModel({ type: String, default: "" });

const host = ref(null);
let view = null;
const dialect = new Compartment();
const dialects = { postgres: PostgreSQL, mysql: MySQL, sqlserver: MSSQL };

const shortcuts = Prec.highest(
  keymap.of([
    { key: "Mod-Enter", run: () => (emit("run"), true) },
    { key: "Shift-Mod-Enter", run: () => (emit("run-all"), true) },
    { key: "Mod-.", run: () => (emit("cancel"), true) },
  ]),
);

onMounted(() => {
  view = new EditorView({
    parent: host.value,
    state: EditorState.create({
      doc: model.value,
      extensions: [
        shortcuts,
        lineNumbers(),
        highlightActiveLineGutter(),
        highlightActiveLine(),
        history(),
        drawSelection(),
        indentOnInput(),
        bracketMatching(),
        closeBrackets(),
        highlightSelectionMatches(),
        dialect.of(
          sql({
            dialect: dialects[props.engine] ?? PostgreSQL,
            upperCaseKeywords: true,
          }),
        ),
        editorTheme,
        editorHighlighting,
        keymap.of([
          ...closeBracketsKeymap,
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
});

watch(model, (value) => {
  if (view && value !== view.state.doc.toString())
    view.dispatch({
      changes: { from: 0, to: view.state.doc.length, insert: value },
    });
});

watch(
  () => props.engine,
  (engine) =>
    view?.dispatch({
      effects: dialect.reconfigure(
        sql({
          dialect: dialects[engine] ?? PostgreSQL,
          upperCaseKeywords: true,
        }),
      ),
    }),
);

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

defineExpose({ runTarget, focus });
</script>

<template>
  <div ref="host" class="h-full min-h-0 overflow-hidden" />
</template>
