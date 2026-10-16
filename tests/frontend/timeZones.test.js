import { describe, expect, it } from "vitest";
import { inTimeZone } from "../../src/lib/timeZones.js";

describe("inTimeZone", () => {
  it("converts values that carry an offset", () => {
    expect(inTimeZone("2026-10-15 09:30:00+00", "UTC")).toBe(
      "2026-10-15 09:30:00 +00:00",
    );
    expect(inTimeZone("2026-10-15 09:30:00.123+00", "Africa/Blantyre")).toBe(
      "2026-10-15 11:30:00.123 +02:00",
    );
    expect(inTimeZone("2026-10-15 23:30:00 -05:00", "UTC")).toBe(
      "2026-10-16 04:30:00 +00:00",
    );
  });

  it("leaves everything else exactly as stored", () => {
    // No offset: a wall-clock time, not an instant.
    expect(inTimeZone("2026-10-15 09:30:00", "UTC")).toBe(
      "2026-10-15 09:30:00",
    );
    expect(inTimeZone("hello", "UTC")).toBe("hello");
    expect(inTimeZone("2026-10-15 09:30:00+00", "")).toBe(
      "2026-10-15 09:30:00+00",
    );
    expect(inTimeZone("2026-10-15 09:30:00+00", "Not/AZone")).toBe(
      "2026-10-15 09:30:00+00",
    );
  });
});
