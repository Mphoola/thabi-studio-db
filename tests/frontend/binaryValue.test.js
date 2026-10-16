import { describe, expect, it } from "vitest";
import { base64ToHex, hexDump, imageMime } from "../../src/lib/binaryValue.js";

describe("binary values", () => {
  it("recognises common image formats from their first bytes", () => {
    expect(imageMime("89504e470d0a1a0a0000")).toBe("image/png");
    expect(imageMime("FFD8FFE0")).toBe("image/jpeg");
    expect(imageMime("524946460000000057454250")).toBe("image/webp");
    expect(imageMime("00010203")).toBeNull();
  });

  it("writes every byte as hex and dumps with offsets", () => {
    const base64 = btoa("AB\u0000");
    expect(base64ToHex(base64)).toBe("0x414200");
    expect(hexDump(base64)).toMatch(/^00000000 {2}41 42 00 +AB\.$/);
    expect(hexDump(btoa("x".repeat(40)), 16)).toContain("24 more bytes");
  });
});
