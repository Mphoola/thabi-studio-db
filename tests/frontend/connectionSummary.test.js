import { describe, expect, it } from "vitest";
import {
  connectionSummary,
  tlsLabel,
} from "../../src/lib/connectionSummary.js";

const profile = {
  driver: "mysql",
  name: "Saris",
  username: "mustsaris",
  databaseName: "",
  tlsMode: "verify-full",
  sshEnabled: true,
};

const texts = (parts) => parts.map((part) => part.text);

describe("connectionSummary", () => {
  it("uses the server version and open database once connected", () => {
    const status = {
      state: "connected",
      server: { version: "MySQL 5.7.33", database: "academicyear" },
    };
    expect(texts(connectionSummary(profile, status))).toEqual([
      "MySQL 5.7.33",
      "Verified TLS",
      "SSH",
      "Saris",
      "mustsaris",
      "academicyear",
    ]);
  });

  it("falls back to the engine and saved database before connecting", () => {
    const parts = connectionSummary(
      {
        ...profile,
        sshEnabled: false,
        databaseName: "shop",
        tlsMode: "disable",
      },
      null,
    );
    expect(texts(parts)).toEqual([
      "MySQL",
      "No TLS",
      "Saris",
      "mustsaris",
      "shop",
    ]);
  });

  it("flags unverified TLS", () => {
    expect(tlsLabel("require")).toBe("TLS, unverified");
    const tls = connectionSummary({ ...profile, tlsMode: "require" }, null)[1];
    expect(tls.tone).toBe("warning");
  });
});
