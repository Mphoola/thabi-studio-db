import { describe, expect, it } from "vitest";
import { parseConnectionUrl } from "../../src/lib/connectionUrl.js";
import { relativeTime } from "../../src/lib/relativeTime.js";

describe("parseConnectionUrl", () => {
  it("reads PostgreSQL URLs with credentials and TLS", () => {
    expect(
      parseConnectionUrl(
        "postgresql://app:s%40cret@db.internal:6543/shop?sslmode=require",
      ),
    ).toEqual({
      driver: "postgres",
      host: "db.internal",
      port: "6543",
      username: "app",
      databasePassword: "s@cret",
      databaseName: "shop",
      tlsMode: "require",
    });
  });

  it("fills default ports and handles MySQL, SQL Server and JDBC", () => {
    expect(parseConnectionUrl("mysql://root@localhost/app")).toMatchObject({
      driver: "mysql",
      port: "3306",
      databaseName: "app",
    });
    expect(
      parseConnectionUrl("mssql://sa@[::1]/master?encrypt=false"),
    ).toMatchObject({
      driver: "sqlserver",
      host: "::1",
      port: "1433",
      tlsMode: "disable",
    });
    expect(parseConnectionUrl("jdbc:postgresql://h/db")).toMatchObject({
      driver: "postgres",
    });
  });

  it("rejects anything else", () => {
    expect(parseConnectionUrl("redis://localhost")).toBeNull();
    expect(parseConnectionUrl("not a url")).toBeNull();
  });
});

describe("relativeTime", () => {
  const now = new Date("2026-10-15T12:00:00Z");
  it("reads SQLite UTC timestamps", () => {
    expect(relativeTime("2026-10-15 11:59:30", now)).toBe("just now");
    expect(relativeTime("2026-10-15 11:45:00", now)).toBe("15 min ago");
    expect(relativeTime("2026-10-15 09:00:00", now)).toBe("3 h ago");
    expect(relativeTime("2026-10-14 10:00:00", now)).toBe("yesterday");
    expect(relativeTime("2026-10-11 12:00:00", now)).toBe("4 days ago");
    expect(relativeTime(null, now)).toBe("");
  });
});
