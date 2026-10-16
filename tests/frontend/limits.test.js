import { describe, expect, it } from "vitest";
import {
  describeLimit,
  limitsFor,
  overridesDefaults,
} from "../../src/lib/limits.js";

const settings = { queryTimeoutSeconds: 30, maxResultRows: 10_000 };

describe("limitsFor", () => {
  it("uses the app defaults when a connection sets nothing", () => {
    expect(
      limitsFor({ queryTimeoutSeconds: 0, maxResultRows: 0 }, settings),
    ).toEqual({ timeoutSeconds: 30, maxRows: 10_000 });
    expect(limitsFor(null, settings)).toEqual({
      timeoutSeconds: 30,
      maxRows: 10_000,
    });
  });

  it("lets a connection override either value", () => {
    expect(
      limitsFor({ queryTimeoutSeconds: 120, maxResultRows: 0 }, settings),
    ).toEqual({ timeoutSeconds: 120, maxRows: 10_000 });
    expect(
      limitsFor({ queryTimeoutSeconds: 0, maxResultRows: 500 }, settings),
    ).toEqual({ timeoutSeconds: 30, maxRows: 500 });
  });

  it("treats zero as no limit, like the settings dialog", () => {
    expect(
      limitsFor(
        { queryTimeoutSeconds: 0, maxResultRows: 0 },
        {
          queryTimeoutSeconds: 0,
          maxResultRows: 0,
        },
      ),
    ).toEqual({ timeoutSeconds: null, maxRows: null });
  });

  it("never exceeds what the drivers will accept", () => {
    expect(
      limitsFor(
        { queryTimeoutSeconds: 99_999_999, maxResultRows: 9_999_999 },
        settings,
      ),
    ).toEqual({ timeoutSeconds: 24 * 60 * 60, maxRows: 1_000_000 });
  });

  it("ignores nonsense rather than passing it to a driver", () => {
    expect(
      limitsFor({ queryTimeoutSeconds: "abc", maxResultRows: -5 }, settings),
    ).toEqual({ timeoutSeconds: 30, maxRows: 10_000 });
  });

  it("knows when a connection overrides the defaults", () => {
    expect(overridesDefaults({ queryTimeoutSeconds: 60 })).toBe(true);
    expect(overridesDefaults({ maxResultRows: 5 })).toBe(true);
    expect(
      overridesDefaults({ queryTimeoutSeconds: 0, maxResultRows: 0 }),
    ).toBe(false);
    expect(overridesDefaults(null)).toBe(false);
  });
});

describe("describeLimit", () => {
  it("reads the way the settings dialog does", () => {
    expect(describeLimit(0, "seconds")).toBe("no limit");
    expect(describeLimit(30, "seconds")).toBe("30 s");
    expect(describeLimit(120, "seconds")).toBe("2 min");
    expect(describeLimit(2000, "rows")).toBe("2,000 rows");
  });
});
