import { computed, nextTick, reactive, ref, watch } from "vue";
import { useConnectionProfiles } from "../../../composables/useConnectionProfiles.js";
import { useDialogs } from "../../../composables/useDialogs.js";
import { defaultSafeMode } from "../../../lib/sqlSafety.js";
import { suggestedTlsMode } from "../../../lib/tls.js";
import {
  defaultPortFor,
  validateConnectionProfile,
} from "../../../lib/connectionValidation.js";

/**
 * The connection form's state: defaults, loading a profile to edit, the
 * fields that follow each other (port with driver, TLS with host, safe mode
 * with environment), and the request Rust receives.
 */
export function useConnectionForm({ fields, testResult }) {
  const store = useConnectionProfiles();
  const dialogs = useDialogs();

  // Until the user picks a TLS setting, it follows the host (see lib/tls.js).
  const tlsTouched = ref(false);
  // The safe mode follows the environment until the user picks one (Plan §7b:
  // Production starts on "Confirm Writes").
  const safeModeTouched = ref(false);

  const defaults = () => ({
    name: "",
    environment: "local",
    driver: "postgres",
    host: "localhost",
    port: "5432",
    databaseName: "",
    username: "",
    databasePassword: "",
    clearDatabasePassword: false,
    tlsMode: suggestedTlsMode("localhost"),
    tlsCaPath: "",
    tlsClientCert: "",
    tlsClientKey: "",
    sshEnabled: false,
    sshHost: "",
    sshPort: "22",
    sshUsername: "",
    sshPassword: "",
    clearSshPassword: false,
    sshAuthMode: "password",
    sshKeyPath: "",
    sshKeyPassphrase: "",
    clearSshKeyPassphrase: false,
    safeMode: defaultSafeMode("local"),
    groupName: "",
    colour: "",
    askDatabasePassword: false,
    askSshPassword: false,
    askSshKeyPassphrase: false,
    sshKeepaliveSeconds: "0",
    startupCommands: "",
    queryTimeoutSeconds: "0",
    maxResultRows: "0",
  });
  const form = reactive(defaults());
  const open = computed({
    get: () => dialogs.open.value === "connection",
    set: (value) => (value ? dialogs.show("connection") : dialogs.close()),
  });
  const editingProfile = computed(
    () =>
      store.profiles.value.find(
        (profile) => profile.id === store.editingId.value,
      ) ?? null,
  );

  function resetForm() {
    Object.assign(form, defaults());
    fields.value = {};
    testResult.value = null;
    tlsTouched.value = false;
    safeModeTouched.value = false;
    const profile = editingProfile.value;
    if (!profile) return;
    tlsTouched.value = true;
    safeModeTouched.value = true;
    Object.assign(form, {
      name: profile.name,
      environment: profile.environment,
      driver: profile.driver,
      host: profile.host,
      port: String(profile.port),
      databaseName: profile.databaseName,
      username: profile.username,
      tlsMode: profile.tlsMode,
      tlsCaPath: profile.tlsCaPath,
      tlsClientCert: profile.tlsClientCert ?? "",
      tlsClientKey: profile.tlsClientKey ?? "",
      sshEnabled: profile.sshEnabled,
      sshHost: profile.sshHost,
      sshPort: String(profile.sshPort),
      sshUsername: profile.sshUsername,
      sshAuthMode: profile.sshAuthMode,
      sshKeyPath: profile.sshKeyPath,
      safeMode: profile.safeMode ?? defaultSafeMode(profile.environment),
      groupName: profile.groupName ?? "",
      colour: profile.colour ?? "",
      askDatabasePassword: !!profile.askDatabasePassword,
      askSshPassword: !!profile.askSshPassword,
      askSshKeyPassphrase: !!profile.askSshKeyPassphrase,
      sshKeepaliveSeconds: String(profile.sshKeepaliveSeconds ?? 0),
      startupCommands: profile.startupCommands ?? "",
      queryTimeoutSeconds: String(profile.queryTimeoutSeconds ?? 0),
      maxResultRows: String(profile.maxResultRows ?? 0),
    });
  }

  watch(
    () => [open.value, store.editingId.value],
    ([isOpen]) => isOpen && resetForm(),
  );
  watch(
    () => form.driver,
    (next, previous) => {
      if (form.port === defaultPortFor(previous))
        form.port = defaultPortFor(next);
    },
  );
  watch(
    () => form.host,
    (host) => {
      if (!tlsTouched.value) form.tlsMode = suggestedTlsMode(host);
    },
  );
  watch(
    () => form.environment,
    (environment, previous) => {
      if (safeModeTouched.value && form.safeMode !== defaultSafeMode(previous))
        return;
      form.safeMode = defaultSafeMode(environment);
    },
  );
  watch(
    () => form.sshEnabled,
    (enabled) => {
      if (!enabled) form.sshPassword = "";
      // Through a tunnel the database is usually on the SSH server itself.
      else if (!form.host.trim()) form.host = "localhost";
    },
  );
  // A test result describes the values it ran with; clear it when they change.
  watch(
    () => JSON.stringify(form),
    () => (testResult.value = null),
  );

  /** The request body for Rust, or null (with field errors) when invalid. */
  function buildInput() {
    fields.value = validateConnectionProfile(form);
    if (Object.keys(fields.value).length) return null;
    const ssh = form.sshEnabled;
    return {
      ...form,
      // A secret that is asked for each time is never sent to be saved.
      databasePassword: form.askDatabasePassword ? "" : form.databasePassword,
      sshPassword:
        ssh && form.sshAuthMode === "password" && !form.askSshPassword
          ? form.sshPassword
          : "",
      sshKeyPassphrase:
        ssh && form.sshAuthMode === "key" && !form.askSshKeyPassphrase
          ? form.sshKeyPassphrase
          : "",
    };
  }

  async function fillFromUrl(parsed) {
    Object.assign(form, parsed);
    if (parsed.tlsMode) tlsTouched.value = true;
    if (!form.name.trim()) form.name = parsed.databaseName || parsed.host;
    fields.value = {};
    // The driver watcher may reset the port to a default; keep the URL's.
    await nextTick();
    form.port = parsed.port;
  }

  return {
    form,
    open,
    editingProfile,
    tlsTouched,
    safeModeTouched,
    buildInput,
    fillFromUrl,
  };
}
