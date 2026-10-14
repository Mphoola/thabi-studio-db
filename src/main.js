import { createApp } from "vue";
import App from "./App.vue";
import { initTheme } from "./composables/useTheme.js";
import { useToasts } from "./composables/useToasts.js";
import { toUserMessage } from "./lib/errors.js";
import "./styles/main.css";

initTheme();

const app = createApp(App);
const toasts = useToasts();

// Last-resort handlers. Errors inside the component tree are caught by
// ErrorBoundary first; these cover event handlers and stray promises.
app.config.errorHandler = (error, _instance, info) => {
  console.error(`[vue] ${info}`, error);
  toasts.error("Something went wrong", toUserMessage(error));
};

window.addEventListener("unhandledrejection", (event) => {
  console.error("[unhandledrejection]", event.reason);
  toasts.error("Something went wrong", toUserMessage(event.reason));
});

app.mount("#app");
