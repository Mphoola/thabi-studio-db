import { describe, expect, it } from "vitest";
import { formatBytes, formatDuration } from "../../src/lib/formatSize.js";

describe("formatSize", () => {
  it("formats bytes", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(16384)).toBe("16.0 KB");
    expect(formatBytes(1.5 * 1024 ** 3)).toBe("1.5 GB");
    expect(formatBytes(300 * 1024 ** 2)).toBe("300 MB");
    expect(formatBytes(null)).toBe("—");
  });

  it("formats durations", () => {
    expect(formatDuration(45)).toBe("45 s");
    expect(formatDuration(200)).toBe("3 min 20 s");
    expect(formatDuration(7500)).toBe("2 h 5 min");
    expect(formatDuration(3 * 86400 + 4 * 3600)).toBe("3 d 4 h");
    expect(formatDuration(null)).toBe("—");
  });
});
