import { confirm } from "./useConfirm.js";
import { useToasts } from "./useToasts.js";
import { forgetSshHostKeys, trustSshHostKey } from "../lib/tauri/commands.js";

/**
 * Runs a connect or test action and handles SSH host keys the way OpenSSH
 * does, but with a dialog: an unknown key is shown with its fingerprint and
 * only trusted when the user agrees; a changed key is refused, and the user
 * can choose to forget the old one.
 */
export function useHostKeyTrust() {
  const toasts = useToasts();

  async function askToTrust(data) {
    return confirm({
      title: `Trust “${data.host}”?`,
      message: `This is the first connection to the SSH server ${data.host}:${data.port}. Check that its ${data.algorithm} fingerprint matches before continuing.`,
      detail: data.fingerprint,
      confirmLabel: "Trust and Connect",
    });
  }

  async function askToForget(data) {
    const ok = await confirm({
      title: `Forget the saved key for “${data.host}”?`,
      message:
        "Do this only if you know the server was rebuilt or its key was replaced. You'll be asked to check the new fingerprint next.",
      detail: `New key: ${data.fingerprint}`,
      confirmLabel: "Forget Key",
      destructive: true,
    });
    if (!ok) return false;
    const { stillInOpensshKnownHosts } = await forgetSshHostKeys(
      data.host,
      data.port,
    );
    if (stillInOpensshKnownHosts) {
      toasts.error(
        "The old key is also in ~/.ssh/known_hosts",
        `Remove it in Terminal with: ssh-keygen -R ${data.port === 22 ? data.host : `[${data.host}]:${data.port}`}`,
      );
      return false;
    }
    return true;
  }

  /**
   * @template T
   * @param {() => Promise<T>} action
   * @returns {Promise<T>}
   */
  async function run(action) {
    for (let attempt = 0; attempt < 3; attempt += 1) {
      try {
        return await action();
      } catch (error) {
        const data = error?.data;
        if (error?.kind === "ssh_host_unknown" && data) {
          if (!(await askToTrust(data))) throw error;
          await trustSshHostKey(data.host, data.port, data.publicKey);
          continue;
        }
        if (error?.kind === "ssh_host_changed" && data) {
          if (!(await askToForget(data))) throw error;
          continue;
        }
        throw error;
      }
    }
    return action();
  }

  return { run };
}
