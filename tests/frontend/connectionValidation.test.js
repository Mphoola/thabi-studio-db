import { describe, expect, it } from "vitest";
import {
  defaultPortFor,
  hostError,
  isValidHost,
  validateConnectionProfile,
} from "../../src/lib/connectionValidation.js";
import { suggestedTlsMode, tlsHint } from "../../src/lib/tls.js";

const validProfile = {
  name: "Local database",
  environment: "local",
  driver: "postgres",
  host: "localhost",
  port: "5432",
  databaseName: "demo",
  username: "user",
  sshEnabled: false,
  sshHost: "",
  sshPort: "22",
  sshUsername: "",
  databasePassword: "",
  sshPassword: "",
  clearDatabasePassword: false,
  clearSshPassword: false,
  tlsMode: "disable",
  tlsCaPath: "",
};

describe("isValidHost", () => {
  it.each([
    "db.example.com",
    "localhost",
    "127.0.0.1",
    "::1",
    "[2001:db8::1]",
    "my_db",
  ])("accepts %s", (host) => expect(isValidHost(host)).toBe(true));

  it.each(["", "-bad.example", "db..example", "bad host", "256.0.0.1"])(
    "rejects %s",
    (host) => expect(isValidHost(host)).toBe(false),
  );
});

describe("hostError", () => {
  it("explains that named SQL Server instances need a port instead", () => {
    expect(hostError("sql01\\prod")).toContain("Named instances");
  });
});

describe("TLS helpers", () => {
  it("suggests TLS off only for this Mac", () => {
    expect(suggestedTlsMode("localhost")).toBe("disable");
    expect(suggestedTlsMode("127.0.0.1")).toBe("disable");
    expect(suggestedTlsMode("db.example.com")).toBe("verify-full");
  });

  it("warns when TLS is off for a remote host", () => {
    expect(tlsHint("disable", "db.example.com")).toContain("plain text");
    expect(tlsHint("disable", "localhost")).toContain("this computer");
  });
});

describe("validateConnectionProfile", () => {
  it("rejects unknown TLS settings", () => {
    expect(
      validateConnectionProfile({ ...validProfile, tlsMode: "prefer" }),
    ).toMatchObject({ tlsMode: "Choose a TLS setting." });
  });

  it("uses database-specific default ports", () => {
    expect(defaultPortFor("mysql")).toBe("3306");
    expect(defaultPortFor("postgres")).toBe("5432");
    expect(defaultPortFor("sqlserver")).toBe("1433");
  });

  it("accepts a profile with a valid driver and default connection details", () => {
    expect(validateConnectionProfile(validProfile)).toEqual({});
  });

  it("returns field-specific errors for invalid values", () => {
    expect(
      validateConnectionProfile({
        ...validProfile,
        name: " ",
        host: "bad host",
        port: "0",
      }),
    ).toMatchObject({
      name: "Enter a connection name.",
      host: "Enter a valid host name or IP address.",
      port: "Enter a port from 1 to 65,535.",
    });
  });

  it("validates SSH fields only when SSH is enabled", () => {
    expect(
      validateConnectionProfile({
        ...validProfile,
        sshEnabled: true,
        sshHost: "",
      }),
    ).toMatchObject({
      sshHost: "Enter a valid SSH host name or IP address.",
      sshUsername: "Enter an SSH user name.",
    });
  });
});

describe("SSH sign-in", () => {
  const ssh = {
    ...validProfile,
    sshEnabled: true,
    sshHost: "bastion.example.com",
    sshUsername: "deploy",
  };

  it("needs a key file for key authentication", () => {
    expect(
      validateConnectionProfile({ ...ssh, sshAuthMode: "key", sshKeyPath: "" }),
    ).toMatchObject({ sshKeyPath: "Choose your SSH private key file." });
  });

  it("accepts the agent without a password or key", () => {
    expect(validateConnectionProfile({ ...ssh, sshAuthMode: "agent" })).toEqual(
      {},
    );
  });
});

describe("ask every time", () => {
  const base = {
    name: "Prod",
    environment: "production",
    driver: "postgres",
    host: "db.example.com",
    port: "5432",
    username: "app",
    tlsMode: "verify-full",
  };

  it("accepts asking instead of saving", () => {
    expect(
      validateConnectionProfile({
        ...base,
        askDatabasePassword: true,
        askSshPassword: true,
        askSshKeyPassphrase: true,
        sshKeepaliveSeconds: "0",
      }),
    ).toEqual({});
  });

  it("rejects a typed secret while asking for it", () => {
    const errors = validateConnectionProfile({
      ...base,
      askDatabasePassword: true,
      databasePassword: "typed",
    });
    expect(errors.databasePassword).toMatch(/not both/);
  });

  it("checks the keep-alive range", () => {
    expect(
      validateConnectionProfile({ ...base, sshKeepaliveSeconds: "" })
        .sshKeepaliveSeconds,
    ).toBeUndefined();
    expect(
      validateConnectionProfile({ ...base, sshKeepaliveSeconds: "30" })
        .sshKeepaliveSeconds,
    ).toBeUndefined();
    expect(
      validateConnectionProfile({ ...base, sshKeepaliveSeconds: "99999" })
        .sshKeepaliveSeconds,
    ).toMatch(/3,600/);
    expect(
      validateConnectionProfile({ ...base, sshKeepaliveSeconds: "-1" })
        .sshKeepaliveSeconds,
    ).toBeDefined();
  });
});
